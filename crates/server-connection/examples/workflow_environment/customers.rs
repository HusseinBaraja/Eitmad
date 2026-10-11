use super::{Client, Result, checked, correlation, now};
use eitmad_authorization::{
    AuthorizationGate, AuthorizationService, BoundaryAuditContext, BoundaryKind, RelationshipPolicy,
};
use eitmad_contracts::{
    authorization::{
        ActionId, AuthorizationRequest, ObjectId, ObjectKind, PermissionRule, RelationshipTuple,
        ScopedObject, TupleSubject,
    },
    sync::SyncMode,
    transport::{CapabilityId, SchemaId},
    updates::ReleaseVersion,
    versioning::{PeerHello, PeerKind, SchemaSupport, SupportedProtocol},
};
use eitmad_customer::{CustomerService, CustomerSyncCycle};
use eitmad_observability_audit::AuditTarget;
use eitmad_server_connection::DirectServerDriver;
use eitmad_sync::{
    RetryPolicy, SyncAuthorization, SyncEngine, TransportAuthentication, WanAdapter,
};

pub(super) fn synchronize(client: &Client) -> Result<()> {
    let actor = &client.branch;
    let authorization = AuthorizationService::new(client.store.clone());
    checked(
        authorization.authorize(actor, eitmad_authorization::CUSTOMER_READ_PERMISSION),
        "customer sync permission",
    )?;
    let object = ScopedObject {
        tenant_id: actor.tenant_id,
        workspace_id: None,
        kind: checked(ObjectKind::parse("branch"), "branch object")?,
        id: ObjectId::new(actor.scope.id.value()),
    };
    let action = checked(
        ActionId::parse("eitmad.action.sync.write.v1"),
        "sync action",
    )?;
    let relationships = checked(
        client.store.list_relationships(&actor.scope, None, 100),
        "persisted branch relationships",
    )?
    .relationships;
    let relations = relationships
        .iter()
        .filter(|r| r.subject.principal_id == actor.identity.principal_id)
        .map(|r| r.relation.clone())
        .collect::<Vec<_>>();
    if relations.is_empty() {
        return Err("persisted branch role missing");
    }
    let tuples = relationships
        .into_iter()
        .map(|r| RelationshipTuple {
            subject: TupleSubject::Principal(r.subject),
            relation: r.relation,
            object: object.clone(),
            condition: None,
        })
        .collect();
    let policy = checked(
        RelationshipPolicy::new(
            tuples,
            vec![PermissionRule {
                action: action.clone(),
                object_kind: object.kind.clone(),
                relations,
                inherits_via: vec![],
            }],
        ),
        "persisted sync policy",
    )?;
    let request = AuthorizationRequest {
        action,
        object,
        attributes: std::collections::BTreeMap::new(),
    };
    let audit = BoundaryAuditContext {
        kind: BoundaryKind::Sync,
        operation: "eitmad.customer.sync-cycle.v1".into(),
        target: AuditTarget {
            kind: "branch".into(),
            identifiers: vec![actor.scope.id.value().to_string()],
        },
        occurred_at: now(),
        correlation_id: correlation(),
        causation_id: None,
        idempotency_key: None,
        extension_points: vec![],
    };
    let mut engine = checked(
        SyncEngine::open(
            client.store.clone(),
            actor.scope.clone(),
            SyncMode::LocalFirst,
            SyncAuthorization::new(AuthorizationGate::new(policy, client.store.clone())),
            actor,
            &audit,
        ),
        "customer sync engine",
    )?;
    let mut transport = transport(client)?;
    checked(
        CustomerSyncCycle {
            customers: &CustomerService::new(client.store.clone(), authorization),
            engine: &mut engine,
            transport: &mut transport,
            actor,
            server_scope: &actor.scope,
            request: &request,
            audit: &audit,
        }
        .run(),
        "customer TLS synchronization",
    )
}

fn peer_hello() -> Result<PeerHello> {
    Ok(PeerHello {
        peer_kind: PeerKind::Engine,
        product_version: ReleaseVersion::new(semver::Version::new(0, 0, 0)),
        protocols: vec![SupportedProtocol {
            major: 1,
            minimum_minor: 17,
            maximum_minor: eitmad_contracts::PROTOCOL_VERSION.minor,
        }],
        capabilities: eitmad_contracts::catalog::CAPABILITIES
            .iter()
            .map(|id| CapabilityId::parse(*id))
            .collect::<std::result::Result<_, _>>()
            .map_err(|_| "capabilities")?,
        required_capabilities: vec![],
        schemas: vec![SchemaSupport {
            schema_id: checked(
                SchemaId::parse("eitmad.schema.customer.v1"),
                "customer schema",
            )?,
            minimum_version: 1,
            maximum_version: 1,
            required: true,
        }],
    })
}

fn transport(client: &Client) -> Result<WanAdapter<DirectServerDriver>> {
    let config = client.customer_config.clone();
    let hello = peer_hello()?;
    checked(
        WanAdapter::new(
            config.wan_endpoint(),
            DirectServerDriver::new(config, client.secrets.clone(), hello.clone()),
            hello,
            TransportAuthentication::AccountDevice {
                account_id: client.account,
                device_id: client.device,
                credential: client.credential.clone(),
            },
            RetryPolicy::default(),
        ),
        "customer WAN route",
    )
}
