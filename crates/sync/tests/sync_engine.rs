use std::{collections::BTreeMap, sync::Arc};

use eitmad_authorization::{
    AuthorizationGate, BoundaryAuditContext, BoundaryError, BoundaryKind, RelationshipPolicy,
};
use eitmad_contracts::{
    authorization::{
        ActionId, AuthorizationRequest, ObjectId, ObjectKind, PermissionRule, RelationId,
        RelationshipSubject, RelationshipTuple, ScopedObject, TupleSubject,
    },
    identity::{
        AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind, ScopeId,
        ScopeKind, ScopeRef, SessionId, TenantId,
    },
    permissions::PermissionDecision,
    sync::{
        CacheFreshness, ChangeId, ChangeOperation, ChangeRecord, Checkpoint, CommandDisposition,
        CommandResult, ConflictStatus, DeliveryId, EncodedDomainPayload, ErrorCodeRef,
        PendingCommandId, ReconciliationDelivery, RecordAuthority, RecordId, SnapshotId, SyncMode,
        SyncSnapshot,
    },
    transport::{CapabilityId, CorrelationId, IdempotencyKey, SchemaId, UnixMillis},
    updates::ReleaseVersion,
    versioning::{PeerHello, PeerKind, SchemaSupport, SupportedProtocol},
};
use eitmad_observability_audit::AuditTarget;
use eitmad_storage::AuthorityStore;
use eitmad_sync::{
    CommandDraft, ConflictHook, ConflictResolution, DeliveryOutcome, LocalChangeDraft,
    LocalChangeOutcome, PendingCommandOutcome, SyncAuthorization, SyncEngine, SyncEngineError,
};
use tempfile::TempDir;
use uuid::Uuid;

const SYNC_ACTION: &str = "eitmad.action.sync.write.v1";
const SYNC_RELATION: &str = "eitmad.relation.organization.sync-operator.v1";

struct Fixture {
    _directory: TempDir,
    store: AuthorityStore,
    actor: AuthorizationContext,
    request: AuthorizationRequest,
}

impl Fixture {
    fn new() -> Self {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let actor = actor();
        let request = request();
        Self {
            _directory: directory,
            store,
            actor,
            request,
        }
    }

    fn engine(&self, mode: SyncMode, allowed: bool) -> SyncEngine {
        SyncEngine::open(
            self.store.clone(),
            self.actor.scope.clone(),
            mode,
            self.authorization(allowed),
            &self.actor,
            &audit(0),
        )
        .unwrap()
    }

    fn engine_with_hook(&self, resolution: ConflictResolution) -> SyncEngine {
        SyncEngine::open_with_conflict_hook(
            self.store.clone(),
            self.actor.scope.clone(),
            SyncMode::LocalFirst,
            self.authorization(true),
            &self.actor,
            &audit(0),
            Arc::new(StaticConflictHook(resolution)),
        )
        .unwrap()
    }

    fn authorization(&self, allowed: bool) -> SyncAuthorization {
        let tuples = allowed
            .then(|| RelationshipTuple {
                subject: TupleSubject::Principal(RelationshipSubject {
                    principal_id: self.actor.identity.principal_id,
                    principal_kind: self.actor.identity.principal_kind,
                }),
                relation: RelationId::parse(SYNC_RELATION).unwrap(),
                object: self.request.object.clone(),
                condition: None,
            })
            .into_iter()
            .collect();
        let policy = RelationshipPolicy::new(
            tuples,
            vec![PermissionRule {
                action: ActionId::parse(SYNC_ACTION).unwrap(),
                object_kind: ObjectKind::parse("organization").unwrap(),
                relations: vec![RelationId::parse(SYNC_RELATION).unwrap()],
                inherits_via: Vec::new(),
            }],
        )
        .unwrap();
        let gate = AuthorizationGate::new(policy, self.store.clone());
        SyncAuthorization::new(gate)
    }
}

struct StaticConflictHook(ConflictResolution);

impl ConflictHook for StaticConflictHook {
    fn resolve(&self, _conflict: &eitmad_contracts::sync::ConflictRecord) -> ConflictResolution {
        self.0.clone()
    }
}

#[test]
fn local_first_offline_edits_remain_usable_and_durable() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);
    let draft = local_draft(10, 1, "كرسي Chair-١");

    let LocalChangeOutcome::Queued(queued) = engine
        .apply_local_change(&fixture.actor, &fixture.request, &audit(1), draft.clone())
        .unwrap()
    else {
        panic!("first edit must queue");
    };
    assert_eq!(
        engine
            .read_record(
                &fixture.actor,
                &fixture.request,
                &audit(101),
                queued.record_id,
                UnixMillis(50),
            )
            .unwrap()
            .unwrap()
            .record,
        queued
    );
    assert_eq!(engine.pending_changes().len(), 1);

    drop(engine);
    let mut reopened = fixture.engine(SyncMode::LocalFirst, true);
    assert_eq!(reopened.pending_changes(), std::slice::from_ref(&queued));
    assert_eq!(
        reopened
            .read_record(
                &fixture.actor,
                &fixture.request,
                &audit(102),
                queued.record_id,
                UnixMillis(500),
            )
            .unwrap()
            .unwrap()
            .authority,
        RecordAuthority::LocalDurable
    );
    assert_eq!(
        reopened
            .apply_local_change(&fixture.actor, &fixture.request, &audit(2), draft)
            .unwrap(),
        LocalChangeOutcome::Replayed(queued)
    );
}

#[test]
fn reconnect_acknowledges_offline_change_and_advances_checkpoint() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);
    let LocalChangeOutcome::Queued(change) = engine
        .apply_local_change(
            &fixture.actor,
            &fixture.request,
            &audit(3),
            local_draft(11, 2, "طاولة Table-٢"),
        )
        .unwrap()
    else {
        panic!("edit must queue");
    };
    connect(&mut engine, &fixture, SyncMode::LocalFirst, 103);
    let delivery = delivery(20, 21, 22, vec![change], None, Vec::new(), 100);

    assert_eq!(
        engine
            .reconcile(&fixture.actor, &fixture.request, &audit(4), &delivery)
            .unwrap(),
        DeliveryOutcome::Applied
    );
    assert!(engine.pending_changes().is_empty());
    assert_eq!(engine.metadata().checkpoint, Some(Checkpoint::new(id(22))));
}

#[test]
fn concurrent_local_first_edits_create_explicit_conflict() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);
    let LocalChangeOutcome::Queued(local) = engine
        .apply_local_change(
            &fixture.actor,
            &fixture.request,
            &audit(5),
            local_draft(12, 3, "خزانة Wardrobe local"),
        )
        .unwrap()
    else {
        panic!("edit must queue");
    };
    connect(&mut engine, &fixture, SyncMode::LocalFirst, 104);
    let remote = record(40, 12, 4, 1, None, "خزانة Wardrobe remote");

    engine
        .reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(6),
            &delivery(41, 42, 43, vec![remote.clone()], None, Vec::new(), 110),
        )
        .unwrap();

    assert_eq!(engine.conflicts().len(), 1);
    assert_eq!(engine.conflicts()[0].status, ConflictStatus::Open);
    assert_eq!(engine.conflicts()[0].local, local.clone());
    assert_eq!(engine.conflicts()[0].remote, remote);
    assert_eq!(
        engine
            .read_record(
                &fixture.actor,
                &fixture.request,
                &audit(105),
                local.record_id,
                UnixMillis(500),
            )
            .unwrap()
            .unwrap()
            .record,
        local
    );
}

#[test]
fn acknowledging_an_earlier_offline_edit_keeps_the_newest_edit_visible() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);
    let LocalChangeOutcome::Queued(first) = engine
        .apply_local_change(
            &fixture.actor,
            &fixture.request,
            &audit(116),
            local_draft(17, 20, "كرسي Chair first"),
        )
        .unwrap()
    else {
        panic!("first edit must queue");
    };
    let LocalChangeOutcome::Queued(second) = engine
        .apply_local_change(
            &fixture.actor,
            &fixture.request,
            &audit(117),
            local_draft(17, 21, "كرسي Chair second"),
        )
        .unwrap()
    else {
        panic!("second edit must queue");
    };
    connect(&mut engine, &fixture, SyncMode::LocalFirst, 118);

    engine
        .reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(119),
            &delivery(120, 121, 122, vec![first], None, Vec::new(), 200),
        )
        .unwrap();

    assert_eq!(engine.pending_changes(), std::slice::from_ref(&second));
    assert_eq!(
        engine
            .read_record(
                &fixture.actor,
                &fixture.request,
                &audit(123),
                second.record_id,
                UnixMillis(200),
            )
            .unwrap()
            .unwrap()
            .record,
        second
    );
}

#[test]
fn replay_still_requires_current_authorization() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);
    let draft = local_draft(18, 22, "مكتب Desk replay");
    engine
        .apply_local_change(&fixture.actor, &fixture.request, &audit(124), draft.clone())
        .unwrap();
    let mut denied_request = fixture.request.clone();
    denied_request.action = ActionId::parse("eitmad.action.sync.denied.v1").unwrap();

    assert_eq!(
        engine.apply_local_change(&fixture.actor, &denied_request, &audit(125), draft),
        Err(SyncEngineError::Authorization(BoundaryError::Denied))
    );
    assert_eq!(
        engine.read_record(
            &fixture.actor,
            &denied_request,
            &audit(126),
            RecordId::new(id(18)),
            UnixMillis(20),
        ),
        Err(SyncEngineError::Authorization(BoundaryError::Denied))
    );
}

#[test]
fn keep_remote_resolves_conflict_to_authoritative_record() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine_with_hook(ConflictResolution::KeepRemote);
    engine
        .apply_local_change(
            &fixture.actor,
            &fixture.request,
            &audit(126),
            local_draft(19, 23, "خزانة Wardrobe local"),
        )
        .unwrap();
    connect(&mut engine, &fixture, SyncMode::LocalFirst, 127);
    let remote = record(128, 19, 24, 3, None, "خزانة Wardrobe remote");
    engine
        .reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(129),
            &delivery(130, 131, 132, vec![remote.clone()], None, Vec::new(), 210),
        )
        .unwrap();

    assert!(engine.pending_changes().is_empty());
    assert_eq!(engine.conflicts()[0].status, ConflictStatus::Resolved);
    assert_eq!(
        engine
            .read_record(
                &fixture.actor,
                &fixture.request,
                &audit(133),
                remote.record_id,
                UnixMillis(210),
            )
            .unwrap()
            .unwrap()
            .record,
        remote
    );
}

#[test]
fn keep_local_rebases_one_revision_above_remote() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine_with_hook(ConflictResolution::KeepLocal);
    engine
        .apply_local_change(
            &fixture.actor,
            &fixture.request,
            &audit(134),
            local_draft(20, 25, "باب Door local"),
        )
        .unwrap();
    connect(&mut engine, &fixture, SyncMode::LocalFirst, 135);
    let remote = record(136, 20, 26, 7, None, "باب Door remote");
    engine
        .reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(137),
            &delivery(138, 139, 140, vec![remote], None, Vec::new(), 220),
        )
        .unwrap();

    assert_eq!(engine.pending_changes()[0].base_revision, Some(7));
    assert_eq!(engine.pending_changes()[0].revision, 8);
}

#[test]
fn domain_merge_rebases_one_revision_above_remote() {
    let fixture = Fixture::new();
    let merged_payload = payload("باب Door merged");
    let mut engine = fixture.engine_with_hook(ConflictResolution::Merge(merged_payload.clone()));
    engine
        .apply_local_change(
            &fixture.actor,
            &fixture.request,
            &audit(141),
            local_draft(21, 27, "باب Door local"),
        )
        .unwrap();
    connect(&mut engine, &fixture, SyncMode::LocalFirst, 142);
    let remote = record(143, 21, 28, 9, None, "باب Door remote");
    engine
        .reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(144),
            &delivery(145, 146, 147, vec![remote], None, Vec::new(), 230),
        )
        .unwrap();

    assert_eq!(engine.pending_changes()[0].base_revision, Some(9));
    assert_eq!(engine.pending_changes()[0].revision, 10);
    assert_eq!(engine.pending_changes()[0].payload, Some(merged_payload));
}

#[test]
fn rebase_overflow_rolls_back_conflict_mutation() {
    for resolution in [
        ConflictResolution::KeepLocal,
        ConflictResolution::Merge(payload("overflow merge")),
    ] {
        let fixture = Fixture::new();
        let mut engine = fixture.engine_with_hook(resolution);
        engine
            .apply_local_change(
                &fixture.actor,
                &fixture.request,
                &audit(148),
                local_draft(22, 29, "overflow local"),
            )
            .unwrap();
        let pending = engine.pending_changes()[0].clone();
        connect(&mut engine, &fixture, SyncMode::LocalFirst, 149);
        let remote = record(150, 22, 30, u64::MAX, None, "overflow remote");

        assert_eq!(
            engine.reconcile(
                &fixture.actor,
                &fixture.request,
                &audit(151),
                &delivery(152, 153, 154, vec![remote], None, Vec::new(), 240),
            ),
            Err(SyncEngineError::InvalidChange)
        );
        assert_eq!(engine.pending_changes(), std::slice::from_ref(&pending));
        assert!(engine.conflicts().is_empty());
    }
}

#[test]
fn duplicate_delivery_is_ignored_without_reapplying_state() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);
    connect(&mut engine, &fixture, SyncMode::LocalFirst, 106);
    let incoming = record(50, 13, 5, 1, None, "مكتب Desk-٣");
    let delivery = delivery(51, 52, 53, vec![incoming], None, Vec::new(), 120);

    assert_eq!(
        engine
            .reconcile(&fixture.actor, &fixture.request, &audit(7), &delivery)
            .unwrap(),
        DeliveryOutcome::Applied
    );
    assert_eq!(
        engine
            .reconcile(&fixture.actor, &fixture.request, &audit(8), &delivery)
            .unwrap(),
        DeliveryOutcome::DuplicateIgnored
    );
    assert!(engine.conflicts().is_empty());
}

#[test]
fn denied_server_command_rolls_back_optimistic_state() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::ServerAuthoritative, true);
    connect(&mut engine, &fixture, SyncMode::ServerAuthoritative, 107);
    let confirmed = record(60, 14, 6, 1, None, "كنبة Sofa confirmed");
    let initial = snapshot(61, 62, vec![confirmed.clone()], 130, 300);
    engine
        .reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(9),
            &delivery(63, 64, 62, Vec::new(), Some(initial), Vec::new(), 130),
        )
        .unwrap();
    let optimistic = record(65, 14, 7, 2, Some(1), "كنبة Sofa optimistic");
    let command_id = PendingCommandId::new(id(66));
    assert!(matches!(
        engine
            .queue_command(
                &fixture.actor,
                &fixture.request,
                &audit(10),
                command_draft(command_id, 67, optimistic.clone()),
            )
            .unwrap(),
        PendingCommandOutcome::Queued(_)
    ));
    assert_eq!(
        engine
            .read_record(
                &fixture.actor,
                &fixture.request,
                &audit(108),
                optimistic.record_id,
                UnixMillis(140),
            )
            .unwrap()
            .unwrap()
            .authority,
        RecordAuthority::Optimistic
    );
    let denied = CommandResult {
        command_id,
        disposition: CommandDisposition::Denied {
            reason: ErrorCodeRef::parse("eitmad.error.authorization-denied.v1").unwrap(),
        },
    };

    engine
        .reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(11),
            &delivery(68, 69, 70, Vec::new(), None, vec![denied], 150),
        )
        .unwrap();

    assert!(engine.pending_commands().is_empty());
    let view = engine
        .read_record(
            &fixture.actor,
            &fixture.request,
            &audit(109),
            confirmed.record_id,
            UnixMillis(160),
        )
        .unwrap()
        .unwrap();
    assert_eq!(view.record, confirmed);
    assert_eq!(view.authority, RecordAuthority::ServerConfirmed);
}

#[test]
fn command_idempotency_key_rejects_different_drafts() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::ServerAuthoritative, true);
    let command_id = PendingCommandId::new(id(155));
    let optimistic = record(156, 23, 31, 1, None, "كنبة Sofa optimistic");
    let first = command_draft(command_id, 32, optimistic.clone());
    engine
        .queue_command(&fixture.actor, &fixture.request, &audit(157), first.clone())
        .unwrap();
    let mut different = first;
    different.base64 = "ZGlmZmVyZW50".to_owned();

    assert_eq!(
        engine.queue_command(&fixture.actor, &fixture.request, &audit(158), different,),
        Err(SyncEngineError::IdempotencyMismatch)
    );
}

#[test]
fn stale_server_cache_fails_closed_until_snapshot_refresh() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::ServerAuthoritative, true);
    connect(&mut engine, &fixture, SyncMode::ServerAuthoritative, 110);
    let cached = record(80, 15, 8, 1, None, "سرير Bed-٤");
    engine
        .reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(12),
            &delivery(
                81,
                82,
                83,
                Vec::new(),
                Some(snapshot(84, 83, vec![cached.clone()], 100, 120)),
                Vec::new(),
                100,
            ),
        )
        .unwrap();
    assert_eq!(
        engine.read_record(
            &fixture.actor,
            &fixture.request,
            &audit(111),
            cached.record_id,
            UnixMillis(121),
        ),
        Err(SyncEngineError::StaleCache)
    );
    engine
        .reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(13),
            &delivery(
                85,
                86,
                87,
                Vec::new(),
                Some(snapshot(88, 87, vec![cached.clone()], 121, 200)),
                Vec::new(),
                121,
            ),
        )
        .unwrap();
    let view = engine
        .read_record(
            &fixture.actor,
            &fixture.request,
            &audit(112),
            cached.record_id,
            UnixMillis(150),
        )
        .unwrap()
        .unwrap();
    assert_eq!(view.freshness, CacheFreshness::Fresh);
}

#[test]
fn unauthorized_remote_change_never_mutates_local_state() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);
    connect(&mut engine, &fixture, SyncMode::LocalFirst, 113);
    let incoming = record(90, 16, 9, 1, None, "باب Door-٥");
    let record_id = incoming.record_id;
    let mut denied_request = fixture.request.clone();
    denied_request.action = ActionId::parse("eitmad.action.sync.denied.v1").unwrap();

    assert_eq!(
        engine.reconcile(
            &fixture.actor,
            &denied_request,
            &audit(14),
            &delivery(91, 92, 93, vec![incoming], None, Vec::new(), 160),
        ),
        Err(SyncEngineError::Authorization(BoundaryError::Denied))
    );
    assert_eq!(
        engine
            .read_record(
                &fixture.actor,
                &fixture.request,
                &audit(114),
                record_id,
                UnixMillis(200),
            )
            .unwrap(),
        None
    );
    assert!(engine.pending_changes().is_empty());
    assert_eq!(
        RelationshipPolicy::new(Vec::new(), Vec::new())
            .unwrap()
            .decide(&fixture.actor, &fixture.request)
            .decision,
        PermissionDecision::Denied
    );
}

#[test]
fn incompatible_protocol_is_rejected_before_sync_traffic() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);
    let local = hello(1, 3);
    let remote = hello(2, 0);

    assert!(matches!(
        engine.connect(
            &fixture.actor,
            &fixture.request,
            &audit(115),
            &local,
            &remote,
            SyncMode::LocalFirst,
        ),
        Err(SyncEngineError::IncompatiblePeer(_))
    ));
    assert_eq!(
        engine.reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(15),
            &delivery(100, 101, 102, Vec::new(), None, Vec::new(), 170),
        ),
        Err(SyncEngineError::IncompatiblePeer(
            eitmad_contracts::versioning::NegotiationRejection::NoCommonProtocol
        ))
    );
}

#[test]
fn offline_reconciliation_reports_disconnected() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);

    assert_eq!(
        engine.reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(159),
            &delivery(160, 161, 162, Vec::new(), None, Vec::new(), 250),
        ),
        Err(SyncEngineError::Disconnected)
    );
}

#[test]
fn connection_changes_require_current_authorization() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);
    let local = hello(1, 3);
    let remote = hello(1, 3);
    let mut denied_request = fixture.request.clone();
    denied_request.action = ActionId::parse("eitmad.action.sync.denied.v1").unwrap();

    assert_eq!(
        engine.connect(
            &fixture.actor,
            &denied_request,
            &audit(177),
            &local,
            &remote,
            SyncMode::LocalFirst,
        ),
        Err(SyncEngineError::Authorization(BoundaryError::Denied))
    );
    assert_eq!(
        engine.metadata().connection,
        eitmad_contracts::sync::ConnectionState::Offline
    );
    engine
        .connect(
            &fixture.actor,
            &fixture.request,
            &audit(178),
            &local,
            &remote,
            SyncMode::LocalFirst,
        )
        .unwrap();
    assert_eq!(
        engine.disconnect(&fixture.actor, &denied_request, &audit(179)),
        Err(SyncEngineError::Authorization(BoundaryError::Denied))
    );
    assert_eq!(
        engine.metadata().connection,
        eitmad_contracts::sync::ConnectionState::Connected
    );
    engine
        .disconnect(&fixture.actor, &fixture.request, &audit(180))
        .unwrap();
    assert_eq!(
        engine.metadata().connection,
        eitmad_contracts::sync::ConnectionState::Offline
    );
}

#[test]
fn incompatible_status_overrides_queued_work() {
    let fixture = Fixture::new();
    let mut engine = fixture.engine(SyncMode::LocalFirst, true);
    engine
        .apply_local_change(
            &fixture.actor,
            &fixture.request,
            &audit(163),
            local_draft(24, 33, "طاولة Table queued"),
        )
        .unwrap();
    let local = hello(1, 3);
    let remote = hello(2, 0);
    assert!(matches!(
        engine.connect(
            &fixture.actor,
            &fixture.request,
            &audit(164),
            &local,
            &remote,
            SyncMode::LocalFirst,
        ),
        Err(SyncEngineError::IncompatiblePeer(_))
    ));
    assert!(matches!(
        engine.status(),
        eitmad_contracts::sync::SyncStatus::Failed { .. }
    ));
}

#[test]
fn failed_reconciliation_commit_discards_generated_events_and_state() {
    let fixture = Fixture::new();
    let mut initial = fixture.engine(SyncMode::ServerAuthoritative, true);
    connect(&mut initial, &fixture, SyncMode::ServerAuthoritative, 165);
    drop(initial);
    let mut winner = fixture.engine(SyncMode::ServerAuthoritative, true);
    let mut stale = fixture.engine(SyncMode::ServerAuthoritative, true);
    winner
        .reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(166),
            &delivery(167, 168, 169, Vec::new(), None, Vec::new(), 260),
        )
        .unwrap();
    let snapshot = snapshot(
        170,
        171,
        vec![record(172, 25, 34, 1, None, "سرير Bed stale")],
        261,
        300,
    );

    assert_eq!(
        stale.reconcile(
            &fixture.actor,
            &fixture.request,
            &audit(173),
            &delivery(174, 175, 171, Vec::new(), Some(snapshot), Vec::new(), 261),
        ),
        Err(SyncEngineError::StorageConflict)
    );
    assert!(stale.drain_events().is_empty());
    assert_eq!(
        stale
            .read_last_snapshot(&fixture.actor, &fixture.request, &audit(176))
            .unwrap(),
        None
    );
}

#[test]
fn public_errors_have_stable_payload_free_messages() {
    assert_eq!(
        SyncEngineError::Authorization(BoundaryError::Denied).to_string(),
        "sync authorization failed"
    );
    assert_eq!(
        SyncEngineError::IncompatiblePeer(
            eitmad_contracts::versioning::NegotiationRejection::NoCommonProtocol,
        )
        .to_string(),
        "sync peer is incompatible"
    );
}

fn connect(engine: &mut SyncEngine, fixture: &Fixture, mode: SyncMode, audit_value: u128) {
    let local = hello(1, 3);
    let remote = hello(1, 3);
    engine
        .connect(
            &fixture.actor,
            &fixture.request,
            &audit(audit_value),
            &local,
            &remote,
            mode,
        )
        .unwrap();
}

fn hello(major: u16, minor: u16) -> PeerHello {
    PeerHello {
        peer_kind: PeerKind::Engine,
        product_version: ReleaseVersion::new(semver::Version::new(1, 0, 0)),
        protocols: vec![SupportedProtocol {
            major,
            minimum_minor: minor,
            maximum_minor: minor,
        }],
        capabilities: vec![CapabilityId::parse("eitmad.capability.sync.v1").unwrap()],
        required_capabilities: vec![CapabilityId::parse("eitmad.capability.sync.v1").unwrap()],
        schemas: vec![SchemaSupport {
            schema_id: SchemaId::parse("eitmad.schema.furniture-record.v1").unwrap(),
            minimum_version: 1,
            maximum_version: 1,
            required: true,
        }],
    }
}

fn actor() -> AuthorizationContext {
    AuthorizationContext {
        session_id: SessionId::new(id(1)),
        identity: AuthenticatedIdentity {
            principal_id: PrincipalId::new(id(2)),
            principal_kind: PrincipalKind::User,
            device_id: None,
            service_id: None,
        },
        tenant_id: TenantId::new(id(3)),
        workspace_id: None,
        scope: scope(),
    }
}

fn scope() -> ScopeRef {
    ScopeRef {
        kind: ScopeKind::parse("organization").unwrap(),
        id: ScopeId::new(id(4)),
    }
}

fn request() -> AuthorizationRequest {
    AuthorizationRequest {
        action: ActionId::parse(SYNC_ACTION).unwrap(),
        object: ScopedObject {
            tenant_id: TenantId::new(id(3)),
            workspace_id: None,
            kind: ObjectKind::parse("organization").unwrap(),
            id: ObjectId::new(id(4)),
        },
        attributes: BTreeMap::new(),
    }
}

fn audit(value: u128) -> BoundaryAuditContext {
    BoundaryAuditContext {
        kind: BoundaryKind::Sync,
        operation: "eitmad.sync.apply.v1".to_owned(),
        target: AuditTarget {
            kind: "organization".to_owned(),
            identifiers: vec!["organization:synthetic".to_owned()],
        },
        occurred_at: UnixMillis(i64::try_from(value).unwrap()),
        correlation_id: CorrelationId::new(id(value + 1_000)),
        causation_id: None,
        idempotency_key: None,
        extension_points: Vec::new(),
    }
}

fn local_draft(record: u128, key: u128, value: &str) -> LocalChangeDraft {
    LocalChangeDraft {
        record_id: RecordId::new(id(record)),
        operation: ChangeOperation::Upsert,
        changed_at: UnixMillis(10),
        idempotency_key: IdempotencyKey::new(id(key + 2_000)),
        payload: Some(payload(value)),
    }
}

fn command_draft(
    command_id: PendingCommandId,
    key: u128,
    optimistic_change: ChangeRecord,
) -> CommandDraft {
    CommandDraft {
        command_id,
        idempotency_key: IdempotencyKey::new(id(key + 2_000)),
        submitted_at: UnixMillis(140),
        command_schema: SchemaId::parse("eitmad.schema.update-furniture.v1").unwrap(),
        command_schema_version: 1,
        base64: "c3ludGhldGlj".to_owned(),
        optimistic_change: Some(optimistic_change),
    }
}

fn record(
    change: u128,
    record: u128,
    key: u128,
    revision: u64,
    base_revision: Option<u64>,
    value: &str,
) -> ChangeRecord {
    ChangeRecord {
        change_id: ChangeId::new(id(change)),
        record_id: RecordId::new(id(record)),
        scope: scope(),
        operation: ChangeOperation::Upsert,
        base_revision,
        revision,
        changed_at: UnixMillis(20),
        idempotency_key: IdempotencyKey::new(id(key + 3_000)),
        payload: Some(payload(value)),
        merge: None,
    }
}

fn payload(value: &str) -> EncodedDomainPayload {
    EncodedDomainPayload {
        schema_id: SchemaId::parse("eitmad.schema.furniture-record.v1").unwrap(),
        schema_version: 1,
        base64: value.to_owned(),
    }
}

fn snapshot(
    snapshot: u128,
    checkpoint: u128,
    records: Vec<ChangeRecord>,
    created_at: i64,
    valid_until: i64,
) -> SyncSnapshot {
    SyncSnapshot {
        snapshot_id: SnapshotId::new(id(snapshot)),
        scope: scope(),
        checkpoint: Checkpoint::new(id(checkpoint)),
        server_generation: 1,
        created_at: UnixMillis(created_at),
        valid_until: UnixMillis(valid_until),
        records,
    }
}

#[allow(clippy::too_many_arguments)]
fn delivery(
    delivery: u128,
    key: u128,
    checkpoint: u128,
    changes: Vec<ChangeRecord>,
    snapshot: Option<SyncSnapshot>,
    command_results: Vec<CommandResult>,
    received_at: i64,
) -> ReconciliationDelivery {
    ReconciliationDelivery {
        delivery_id: DeliveryId::new(id(delivery)),
        idempotency_key: IdempotencyKey::new(id(key + 4_000)),
        checkpoint: Checkpoint::new(id(checkpoint)),
        received_at: UnixMillis(received_at),
        snapshot,
        changes,
        command_results,
    }
}

const fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
