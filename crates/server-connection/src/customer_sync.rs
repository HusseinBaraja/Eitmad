//! Customer outbox delivery over the shared authenticated WAN transport.
use crate::{DirectServerConfig, DirectServerDriver};
use eitmad_authorization::{
    AuthorizationGate, AuthorizationService, BoundaryAuditContext, BoundaryKind,
    RelationshipPolicy, now,
};
use eitmad_contracts::{
    authorization::{
        ActionId, AuthorizationRequest, ObjectId, ObjectKind, PermissionRule, RelationshipSubject,
        RelationshipTuple, ScopedObject, TupleSubject,
    },
    identity::AuthorizationContext,
    secrets::SecretId,
    sync::SyncMode,
    transport::{CapabilityId, CorrelationId, SchemaId},
    updates::ReleaseVersion,
    versioning::{PeerHello, PeerKind, SchemaSupport, SupportedProtocol},
};
use eitmad_customer::{CUSTOMER_SCHEMA_ID, CustomerError, CustomerService, CustomerSyncCycle};
use eitmad_observability_audit::AuditTarget;
use eitmad_secret_storage::SecretStore;
use eitmad_storage::AuthorityStore;
use eitmad_sync::{
    RetryPolicy, SyncAuthorization, SyncEngine, TransportAuthentication, WanAdapter,
};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct DirectCustomerSyncClient {
    config: DirectServerConfig,
    secrets: SecretStore,
    credential: SecretId,
    store: AuthorityStore,
    worker: std::sync::Mutex<()>,
}

impl DirectCustomerSyncClient {
    #[must_use]
    pub fn from_config(
        config: DirectServerConfig,
        secrets: SecretStore,
        credential: SecretId,
        store: AuthorityStore,
    ) -> Self {
        Self {
            config,
            secrets,
            credential,
            store,
            worker: std::sync::Mutex::new(()),
        }
    }

    /// Retries pending work as its original author, with current local and server authority.
    /// # Errors
    /// Retains pending work on failure and continues other authors before returning an error.
    pub fn retry_pending(&self) -> Result<usize, CustomerError> {
        let mut count = 0;
        let mut failure = None;
        for actor in self
            .store
            .pending_customer_actors()
            .map_err(|_| CustomerError::Unavailable)?
        {
            match self.synchronize(&actor) {
                Ok(transferred) => count += transferred,
                Err(CustomerError::Denied) => (),
                Err(error) => {
                    failure.get_or_insert(error);
                }
            }
        }
        failure.map_or(Ok(count), Err)
    }

    /// Runs one serialized customer cycle on a Rust worker thread.
    /// # Errors
    /// Leaves durable work pending on transport, credential, or authorization failure.
    pub fn synchronize(&self, actor: &AuthorizationContext) -> Result<usize, CustomerError> {
        let _worker = self.worker.lock().map_err(|_| CustomerError::Unavailable)?;
        if actor.scope != self.config.scope || actor.scope.kind.as_str() != "branch" {
            return Err(CustomerError::Denied);
        }
        let authorization = AuthorizationService::new(self.store.clone());
        authorization
            .authorize(actor, eitmad_customer::CUSTOMER_WRITE_PERMISSION)
            .map_err(|_| CustomerError::Denied)?;
        let customers = CustomerService::new(self.store.clone(), authorization);
        let before = customers.sync_batch_for_author(actor)?.len();
        let (request, policy) = self.policy(actor)?;
        let audit = BoundaryAuditContext {
            kind: BoundaryKind::Sync,
            operation: "eitmad.customer.sync-cycle.v1".into(),
            target: AuditTarget {
                kind: "branch".into(),
                identifiers: vec![actor.scope.id.value().to_string()],
            },
            occurred_at: now(),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            causation_id: None,
            idempotency_key: None,
            extension_points: vec![],
        };
        let mut engine = SyncEngine::open(
            self.store.clone(),
            actor.scope.clone(),
            SyncMode::LocalFirst,
            SyncAuthorization::new(AuthorizationGate::new(policy, self.store.clone())),
            actor,
            &audit,
        )
        .map_err(|_| CustomerError::Unavailable)?;
        let mut transport = self.transport(actor)?;
        CustomerSyncCycle {
            customers: &customers,
            engine: &mut engine,
            transport: &mut transport,
            actor,
            server_scope: &actor.scope,
            request: &request,
            audit: &audit,
        }
        .run()
        .map_err(|_| CustomerError::Unavailable)?;
        Ok(before.saturating_sub(customers.sync_batch_for_author(actor)?.len()))
    }

    fn policy(
        &self,
        actor: &AuthorizationContext,
    ) -> Result<(AuthorizationRequest, RelationshipPolicy), CustomerError> {
        let object = ScopedObject {
            tenant_id: actor.tenant_id,
            workspace_id: None,
            kind: ObjectKind::parse("branch").expect("static kind"),
            id: ObjectId::new(actor.scope.id.value()),
        };
        let action = ActionId::parse("eitmad.action.sync.write.v1").expect("static action");
        let relationships = self
            .store
            .relationships_for_subject(
                &actor.scope,
                &RelationshipSubject {
                    principal_id: actor.identity.principal_id,
                    principal_kind: actor.identity.principal_kind,
                },
            )
            .map_err(|_| CustomerError::Unavailable)?;
        let relations = relationships.iter().map(|r| r.relation.clone()).collect();
        let tuples = relationships
            .into_iter()
            .map(|r| RelationshipTuple {
                subject: TupleSubject::Principal(r.subject),
                relation: r.relation,
                object: object.clone(),
                condition: None,
            })
            .collect();
        let policy = RelationshipPolicy::new(
            tuples,
            vec![PermissionRule {
                action: action.clone(),
                object_kind: object.kind.clone(),
                relations,
                inherits_via: vec![],
            }],
        )
        .map_err(|_| CustomerError::Denied)?;
        Ok((
            AuthorizationRequest {
                action,
                object,
                attributes: std::collections::BTreeMap::new(),
            },
            policy,
        ))
    }

    fn transport(
        &self,
        actor: &AuthorizationContext,
    ) -> Result<WanAdapter<DirectServerDriver>, CustomerError> {
        let hello = PeerHello {
            peer_kind: PeerKind::Engine,
            product_version: ReleaseVersion::new(semver::Version::new(0, 0, 0)),
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
                schema_id: SchemaId::parse(CUSTOMER_SCHEMA_ID).expect("static schema"),
                minimum_version: 1,
                maximum_version: 1,
                required: true,
            }],
        };
        let driver =
            DirectServerDriver::new(self.config.clone(), self.secrets.clone(), hello.clone());
        let (credential, stored) = driver
            .account_credential(
                &self.credential,
                actor,
                Instant::now() + Duration::from_secs(10),
            )
            .map_err(|_| CustomerError::Unavailable)?;
        WanAdapter::new(
            self.config.wan_endpoint(),
            driver,
            hello,
            TransportAuthentication::AccountDevice {
                account_id: stored.account_id,
                device_id: stored.device_id,
                credential,
            },
            RetryPolicy::default(),
        )
        .map_err(|_| CustomerError::Unavailable)
    }
}
