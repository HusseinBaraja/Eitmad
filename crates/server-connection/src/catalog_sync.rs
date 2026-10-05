//! Catalog cycles over the existing SubmitLocal/Pull/Acknowledge frames and pinned WAN route.
use crate::{DirectServerConfig, DirectServerDriver};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use eitmad_authorization::{AuthorizationService, now};
use eitmad_contracts::{
    catalog_revision::{CatalogEntry, CatalogRevision},
    identity::AuthorizationContext,
    secrets::SecretId,
    sync::{
        BatchAcknowledgement, ChangeId, ChangeOperation, ChangeRecord, EncodedDomainPayload,
        LocalChangeDisposition, LocalChangeSubmission, PullRequest, RecordId, SnapshotManifest,
        SyncMessage,
    },
    sync_transport::{SyncFrameId, SyncStreamId, SyncTransportFrame, SyncTransportPayload},
    transport::{CapabilityId, CorrelationId, IdempotencyKey, SchemaId, UnixMillis},
    versioning::{PeerHello, PeerKind, SchemaSupport, SupportedProtocol},
};
use eitmad_observability_audit::{AuditTarget, MutationAuditRecord};
use eitmad_pricing::{CatalogReplication, PricingError};
use eitmad_storage::AuthorityStore;
use eitmad_sync::{
    ReceiveOutcome, RetryPolicy, SyncTransport, TransportAuthentication, WanAdapter,
};
use sha2::{Digest as _, Sha256};
use std::time::{Duration, Instant};
use uuid::Uuid;

const PUBLIC_SCHEMA: &str = "eitmad.schema.catalog-public.v1";
pub struct DirectCatalogSyncClient {
    config: DirectServerConfig,
    secrets: eitmad_secret_storage::SecretStore,
    credential: SecretId,
    store: AuthorityStore,
    authorization: AuthorizationService,
    worker: std::sync::Mutex<()>,
}
impl DirectCatalogSyncClient {
    #[must_use]
    pub fn from_config(
        config: DirectServerConfig,
        secrets: eitmad_secret_storage::SecretStore,
        credential: SecretId,
        store: AuthorityStore,
    ) -> Self {
        let authorization = AuthorizationService::new(store.clone());
        Self {
            config,
            secrets,
            credential,
            store,
            authorization,
            worker: std::sync::Mutex::new(()),
        }
    }
    fn require(
        &self,
        actor: &AuthorizationContext,
        schema: &str,
        write: bool,
    ) -> Result<(), PricingError> {
        let permission = match schema {
            "eitmad.schema.material.v1" => {
                if write {
                    "eitmad.permission.material.write.v1"
                } else {
                    "eitmad.permission.material.read.v1"
                }
            }
            "eitmad.schema.part.v1" => {
                if write {
                    "eitmad.permission.part.write.v1"
                } else {
                    "eitmad.permission.part.read.v1"
                }
            }
            "eitmad.schema.product.v1" => {
                if write {
                    "eitmad.permission.product.write.v1"
                } else {
                    "eitmad.permission.product.cost.read.v1"
                }
            }
            "eitmad.schema.furniture.v1" => {
                if write {
                    "eitmad.permission.furniture.write.v1"
                } else {
                    "eitmad.permission.furniture.read.v1"
                }
            }
            PUBLIC_SCHEMA => "eitmad.permission.catalog.read.v1",
            _ => return Err(PricingError::Invalid),
        };
        self.authorization
            .authorize(actor, permission)
            .map_err(Into::into)
    }
    fn transport(
        &self,
        actor: &AuthorizationContext,
        schema: &str,
        deadline: UnixMillis,
    ) -> Result<WanAdapter<DirectServerDriver>, PricingError> {
        if actor.scope.kind.as_str() != "organization"
            || actor.scope.id.value() != actor.tenant_id.value()
        {
            return Err(PricingError::Denied);
        }
        let mut config = self.config.clone();
        config.schema_id = SchemaId::parse(schema).map_err(|_| PricingError::Invalid)?;
        let schema_id = config.schema_id.clone();
        let hello = PeerHello {
            peer_kind: PeerKind::Engine,
            product_version: eitmad_contracts::updates::ReleaseVersion::new(semver::Version::new(
                0, 0, 0,
            )),
            protocols: vec![SupportedProtocol {
                major: 1,
                minimum_minor: 17,
                maximum_minor: eitmad_contracts::PROTOCOL_VERSION.minor,
            }],
            capabilities: eitmad_contracts::catalog::CAPABILITIES
                .iter()
                .map(|id| CapabilityId::parse(*id).expect("registered capability"))
                .collect(),
            required_capabilities: vec![
                CapabilityId::parse("eitmad.capability.sync.v1").expect("static capability"),
            ],
            schemas: vec![SchemaSupport {
                schema_id,
                minimum_version: 1,
                maximum_version: 1,
                required: true,
            }],
        };
        let driver = DirectServerDriver::new(config.clone(), self.secrets.clone(), hello.clone());
        let mut credential = driver
            .load_credential(&self.credential)
            .map_err(|_| PricingError::Denied)?;
        driver
            .refresh_if_due(
                &self.credential,
                &mut credential,
                Instant::now()
                    + Duration::from_millis(
                        u64::try_from(deadline.0.saturating_sub(now().0))
                            .map_err(|_| PricingError::Unconfirmed)?,
                    ),
            )
            .map_err(|_| PricingError::Unconfirmed)?;
        if credential
            .user_id
            .map(eitmad_contracts::identity::UserId::value)
            != Some(actor.identity.principal_id.value())
            || credential.tenant_id != Some(actor.tenant_id)
        {
            return Err(PricingError::Denied);
        }
        WanAdapter::new(
            config.wan_endpoint(),
            driver,
            hello,
            TransportAuthentication::AccountDevice {
                account_id: credential.account_id,
                device_id: credential.device_id,
                credential: self.credential.clone(),
            },
            RetryPolicy::default(),
        )
        .map_err(|_| PricingError::Unconfirmed)
    }
    fn download(
        &self,
        actor: &AuthorizationContext,
        schema: &str,
        deadline: UnixMillis,
    ) -> Result<(), PricingError> {
        self.require(actor, schema, false)?;
        let mut transport = self.transport(actor, schema, deadline)?;
        transport
            .connect(now())
            .map_err(|_| PricingError::Unconfirmed)?;
        let schema_id = SchemaId::parse(schema).map_err(|_| PricingError::Invalid)?;
        loop {
            let after = self.store.catalog_checkpoint(actor, &schema_id)?;
            let response = exchange(
                &mut transport,
                SyncMessage::Pull(PullRequest {
                    after,
                    maximum_records: 50,
                }),
                deadline,
            )?;
            let (records, checkpoint, has_more, acknowledgement) = match response.message {
                SyncMessage::Changes(batch) => {
                    if batch.from_checkpoint != after
                        || batch.records.len() > 50
                        || batch.has_more && batch.records.is_empty()
                    {
                        return Err(PricingError::Unconfirmed);
                    }
                    let ack = BatchAcknowledgement {
                        delivery_id: batch.delivery_id,
                        checkpoint: batch.checkpoint,
                        accepted_records: u32::try_from(batch.records.len())
                            .map_err(|_| PricingError::Invalid)?,
                    };
                    (batch.records, batch.checkpoint, batch.has_more, Some(ack))
                }
                SyncMessage::SnapshotManifest(manifest) => {
                    if manifest.scope != self.config.scope {
                        return Err(PricingError::Denied);
                    }
                    let checkpoint = manifest.checkpoint;
                    (
                        complete_snapshot(&mut transport, &manifest, response.stream, deadline)?,
                        checkpoint,
                        true,
                        None,
                    )
                }
                _ => return Err(PricingError::Unconfirmed),
            };
            let (private, public) = self.decode_page(actor, &schema_id, &records)?;
            self.require(actor, schema, false)?;
            self.store.project_catalog_page(
                actor,
                &schema_id,
                checkpoint,
                &eitmad_storage::CatalogSyncProjection {
                    private: &private,
                    public: &public,
                    normalize_name: eitmad_material::normalize_search,
                },
                &audit(actor, "eitmad.catalog.sync.project.v1"),
            )?;
            if let Some(ack) = acknowledgement {
                if exchange(
                    &mut transport,
                    SyncMessage::Acknowledge(ack.clone()),
                    deadline,
                )?
                .message
                    != SyncMessage::Acknowledge(ack)
                {
                    return Err(PricingError::Unconfirmed);
                }
            }
            if !has_more {
                break;
            }
        }
        transport.disconnect(now());
        Ok(())
    }
    fn decode_page(
        &self,
        actor: &AuthorizationContext,
        schema_id: &SchemaId,
        records: &[ChangeRecord],
    ) -> Result<DecodedPage, PricingError> {
        let schema = schema_id.as_str();
        let mut private = vec![];
        let mut public = vec![];
        for change in records {
            if change.scope != self.config.scope {
                return Err(PricingError::Denied);
            }
            let mut local_change = change.clone();
            local_change.scope = actor.scope.clone();
            if let Some(payload) = &change.payload {
                if payload.schema_id != *schema_id
                    || payload.schema_version != 1
                    || payload.base64.len() > 700_000
                {
                    return Err(PricingError::Invalid);
                }
                let bytes = STANDARD
                    .decode(&payload.base64)
                    .map_err(|_| PricingError::Invalid)?;
                if schema == PUBLIC_SCHEMA {
                    let mut entry: CatalogEntry =
                        serde_json::from_slice(&bytes).map_err(|_| PricingError::Invalid)?;
                    let (kind, id, variant) = entry.price.target.identity();
                    if change.operation != ChangeOperation::Upsert
                        || change.revision == 0
                        || entry.price.target.schema_version() != 1
                        || id.is_nil()
                        || variant.is_nil()
                        || change.record_id.value()
                            != eitmad_pricing::catalog_record_id(&format!("{kind}:{id}:{variant}"))
                        || entry.price.target.scope() != &self.config.scope
                        || entry.price.revision == 0
                        || entry.price.currency != "YER"
                        || entry.price.selling_price_yer <= 0
                    {
                        return Err(PricingError::Invalid);
                    }
                    remap_price(&mut entry.price, &actor.scope);
                    public.push((local_change, Some(entry)));
                } else {
                    let mut record: CatalogRevision =
                        serde_json::from_slice(&bytes).map_err(|_| PricingError::Invalid)?;
                    crate::pricing::remap_catalog_scope(
                        &mut record,
                        &self.config.scope,
                        &actor.scope,
                    )?;
                    if eitmad_pricing::revision_schema(&record) != schema
                        || eitmad_pricing::revision_record_id(&record) != change.record_id.value()
                    {
                        return Err(PricingError::Invalid);
                    }
                    private.push(record);
                }
            } else if schema == PUBLIC_SCHEMA && change.operation == ChangeOperation::Tombstone {
                public.push((local_change, None));
            } else {
                return Err(PricingError::Invalid);
            }
        }
        private.sort_by_key(|record| {
            let (kind, id, revision, _) = record.identity();
            let order = match kind {
                "unit" => 1,
                "material" => 2,
                "material-category" | "part-category" | "product-category"
                | "furniture-category" => 0,
                _ => 3,
            };
            (order, id, revision)
        });
        Ok((private, public))
    }
}
impl CatalogReplication for DirectCatalogSyncClient {
    fn synchronize(
        &self,
        actor: &AuthorizationContext,
        deadline: UnixMillis,
    ) -> Result<usize, PricingError> {
        let _worker = self
            .worker
            .try_lock()
            .map_err(|_| PricingError::Unconfirmed)?;
        self.require(actor, PUBLIC_SCHEMA, false)?;
        self.store
            .register_catalog_client(actor, &audit(actor, "eitmad.catalog.sync.register.v1"))?;
        let manager = match self
            .authorization
            .authorize(actor, "eitmad.permission.material.write.v1")
        {
            Ok(()) => true,
            Err(eitmad_authorization::AuthorizationError::Denied) => false,
            Err(e) => return Err(e.into()),
        };
        let mut count = 0;
        if manager {
            self.store
                .seed_catalog_sync(actor, &audit(actor, "eitmad.catalog.sync.bootstrap.v1"))?;
            for record in self.store.pending_catalog_revisions(&actor.scope)? {
                let schema = eitmad_pricing::revision_schema(&record);
                self.require(actor, schema, true)?;
                if matches!(&record, CatalogRevision::Unit(_)) {
                    self.authorization
                        .authorize(actor, "eitmad.permission.material-unit.manage.v1")?;
                }
                let mut remote = record.clone();
                crate::pricing::remap_catalog_scope(&mut remote, &actor.scope, &self.config.scope)?;
                let id = eitmad_pricing::revision_record_id(&remote);
                let change = ChangeRecord {
                    change_id: ChangeId::new(id),
                    record_id: RecordId::new(id),
                    scope: self.config.scope.clone(),
                    operation: ChangeOperation::Upsert,
                    base_revision: None,
                    revision: 1,
                    changed_at: UnixMillis(0),
                    idempotency_key: IdempotencyKey::new(id),
                    payload: Some(EncodedDomainPayload {
                        schema_id: SchemaId::parse(schema).map_err(|_| PricingError::Invalid)?,
                        schema_version: 1,
                        base64: STANDARD.encode(
                            serde_json::to_vec(&remote).map_err(|_| PricingError::Invalid)?,
                        ),
                    }),
                    merge: None,
                };
                let mut transport = self.transport(actor, schema, deadline)?;
                transport
                    .connect(now())
                    .map_err(|_| PricingError::Unconfirmed)?;
                let response = exchange(
                    &mut transport,
                    SyncMessage::SubmitLocal(LocalChangeSubmission {
                        change: change.clone(),
                    }),
                    deadline,
                )?;
                match response.message {
                    SyncMessage::LocalResult(result)
                        if result.submitted_change_id == change.change_id =>
                    {
                        match result.disposition {
                            LocalChangeDisposition::Applied {
                                authoritative_change,
                            }
                            | LocalChangeDisposition::Replayed {
                                authoritative_change,
                            } if authoritative_change.record_id == change.record_id
                                && authoritative_change.payload == change.payload
                                && authoritative_change.scope == change.scope => {}
                            LocalChangeDisposition::Rejected { .. }
                            | LocalChangeDisposition::Conflicted { .. } => {
                                return Err(PricingError::Reference);
                            }
                            _ => return Err(PricingError::Unconfirmed),
                        }
                    }
                    _ => return Err(PricingError::Unconfirmed),
                }
                self.require(actor, schema, true)?;
                self.store.acknowledge_catalog_revision(
                    &record,
                    &audit(actor, "eitmad.catalog.sync.acknowledge.v1"),
                )?;
                transport.disconnect(now());
                count += 1;
            }
            for schema in [
                "eitmad.schema.material.v1",
                "eitmad.schema.part.v1",
                "eitmad.schema.product.v1",
                "eitmad.schema.furniture.v1",
            ] {
                self.download(actor, schema, deadline)?;
            }
        }
        self.download(actor, PUBLIC_SCHEMA, deadline)?;
        Ok(count)
    }
}
fn audit(actor: &AuthorizationContext, operation: &str) -> MutationAuditRecord {
    MutationAuditRecord::from_authorization(
        actor,
        now(),
        CorrelationId::new(Uuid::new_v4()),
        operation,
        AuditTarget {
            kind: "catalog-sync".into(),
            identifiers: vec![actor.scope.id.value().to_string()],
        },
    )
}
fn remap_price(
    price: &mut eitmad_contracts::pricing::PublishedPrice,
    scope: &eitmad_contracts::identity::ScopeRef,
) {
    match &mut price.target {
        eitmad_contracts::pricing::PriceTarget::Product(r) => r.scope = scope.clone(),
        eitmad_contracts::pricing::PriceTarget::Furniture(r) => r.scope = scope.clone(),
    }
}
fn exchange<T: SyncTransport>(
    transport: &mut T,
    message: SyncMessage,
    deadline: UnixMillis,
) -> Result<Response, PricingError> {
    if now().0 >= deadline.0 {
        return Err(PricingError::Unconfirmed);
    }
    let protocol = transport
        .negotiated_session()
        .ok_or(PricingError::Unconfirmed)?
        .protocol;
    let frame = SyncTransportFrame {
        frame_id: SyncFrameId::new(Uuid::new_v4()),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        protocol_version: protocol,
        correlation_id: CorrelationId::new(Uuid::new_v4()),
        stream_id: SyncStreamId::new(Uuid::new_v4()),
        sequence: 0,
        end_of_stream: true,
        payload: SyncTransportPayload::Message(Box::new(message)),
    };
    transport
        .send(&frame, now())
        .map_err(|_| PricingError::Unconfirmed)?;
    Ok(Response {
        message: receive_message(transport, frame.stream_id, deadline)?,
        stream: frame.stream_id,
    })
}
struct Response {
    message: SyncMessage,
    stream: SyncStreamId,
}
fn receive_message<T: SyncTransport>(
    transport: &mut T,
    stream: SyncStreamId,
    deadline: UnixMillis,
) -> Result<SyncMessage, PricingError> {
    let until = Instant::now()
        + Duration::from_millis(
            u64::try_from(deadline.0.saturating_sub(now().0))
                .map_err(|_| PricingError::Unconfirmed)?,
        );
    loop {
        if Instant::now() >= until {
            return Err(PricingError::Unconfirmed);
        }
        match transport
            .receive(now())
            .map_err(|_| PricingError::Unconfirmed)?
        {
            ReceiveOutcome::NoFrame | ReceiveOutcome::DuplicateIgnored { .. } => (),
            ReceiveOutcome::Frame(reply) => {
                if reply.stream_id != stream {
                    return Err(PricingError::Unconfirmed);
                }
                if let SyncTransportPayload::Message(message) = reply.payload {
                    return Ok(*message);
                }
                return Err(PricingError::Unconfirmed);
            }
        }
    }
}

/// Stages an entire bounded snapshot. No durable projection exists until completion and checksums agree.
fn complete_snapshot<T: SyncTransport>(
    transport: &mut T,
    manifest: &SnapshotManifest,
    stream: SyncStreamId,
    deadline: UnixMillis,
) -> Result<Vec<ChangeRecord>, PricingError> {
    const MAX_RECORDS: u64 = 16_000;
    const MAX_BYTES: usize = 32 * 1024 * 1024;
    if manifest.valid_until.0 <= now().0
        || manifest.total_records > MAX_RECORDS
        || u64::from(manifest.total_chunks) != manifest.total_records.div_ceil(500)
    {
        return Err(PricingError::Invalid);
    }
    let mut records = Vec::new();
    let mut bytes = 0usize;
    for index in 0..manifest.total_chunks {
        let SyncMessage::SnapshotChunk(chunk) = receive_message(transport, stream, deadline)?
        else {
            return Err(PricingError::Unconfirmed);
        };
        let serialized = serde_json::to_vec(&chunk.records).map_err(|_| PricingError::Invalid)?;
        bytes = bytes
            .checked_add(serialized.len())
            .ok_or(PricingError::Invalid)?;
        if chunk.snapshot_id != manifest.snapshot_id
            || chunk.chunk_index != index
            || chunk.records.is_empty()
            || chunk.records.len() > 500
            || URL_SAFE_NO_PAD.encode(Sha256::digest(&serialized)) != chunk.checksum
            || bytes > MAX_BYTES
        {
            return Err(PricingError::Invalid);
        }
        records.extend(chunk.records);
        if u64::try_from(records.len()).map_err(|_| PricingError::Invalid)? > manifest.total_records
        {
            return Err(PricingError::Invalid);
        }
    }
    let SyncMessage::SnapshotComplete(completion) = receive_message(transport, stream, deadline)?
    else {
        return Err(PricingError::Unconfirmed);
    };
    if completion.snapshot_id != manifest.snapshot_id
        || completion.checksum != manifest.checksum
        || u64::try_from(records.len()).map_err(|_| PricingError::Invalid)?
            != manifest.total_records
        || URL_SAFE_NO_PAD.encode(Sha256::digest(
            serde_json::to_vec(&records).map_err(|_| PricingError::Invalid)?,
        )) != manifest.checksum
        || manifest.valid_until.0 <= now().0
    {
        return Err(PricingError::Invalid);
    }
    Ok(records)
}

type DecodedPage = (
    Vec<CatalogRevision>,
    Vec<(ChangeRecord, Option<CatalogEntry>)>,
);
