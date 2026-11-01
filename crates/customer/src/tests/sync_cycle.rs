use super::*;
use eitmad_authorization::{
    AuthorizationGate, BoundaryAuditContext, BoundaryKind, RelationshipPolicy,
};
use eitmad_contracts::{
    authorization::{
        ActionId, AuthorizationRequest, ObjectId, ObjectKind, PermissionRule, RelationshipTuple,
        ScopedObject, TupleSubject,
    },
    sync::{
        ChangeBatch, Checkpoint, ConflictId, DeliveryId, ErrorCodeRef, LocalChangeDisposition,
        LocalChangeResult, SyncMessage, SyncMode,
    },
    sync_transport::{
        SyncCancellationReason, SyncStreamId, SyncTransportFrame, SyncTransportPayload,
    },
    transport::CapabilityId,
    updates::ReleaseVersion,
    versioning::{NegotiatedSession, PeerHello, PeerKind, SchemaSupport, SupportedProtocol},
};
use eitmad_sync::{
    ConnectionHealth, ReceiveOutcome, RetryPolicy, SimulatedTransport, SyncAuthorization,
    SyncEngine, SyncTransport, TransportFailure, TransportKind,
};

fn keep_wal_open(store: &AuthorityStore) -> Connection {
    let connection = Connection::open(store.path()).unwrap();
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    connection
}

#[test]
fn customer_sync_counts_confirmations_while_the_next_batch_stays_full() {
    let directory = TempDir::new().unwrap();
    let (store, customers, actor) = authorized_service(&directory, 41, 51, MANAGER_RELATION);
    // Keep WAL open across page operations instead of checkpointing each closed connection.
    let _database_connection = keep_wal_open(&store);
    for index in 0..102 {
        customers
            .create(
                &mutation(actor.clone(), 1_000 + index, 10),
                &create_command("عميل اختبار", &format!("777{index:06}")),
            )
            .unwrap();
    }
    let (mut engine, request, audit) = sync_engine(&store, &actor);
    let peer = hello();
    let mut server = CustomerServer {
        transport: SimulatedTransport::new(peer.clone(), peer, RetryPolicy::default()),
        records: vec![],
        submitted: 0,
    };
    for (expected_confirmed, expected_next_batch) in [(48, 50), (50, 2), (2, 0)] {
        let confirmed = CustomerSyncCycle {
            customers: &customers,
            engine: &mut engine,
            transport: &mut server,
            actor: &actor,
            server_scope: &actor.scope,
            request: &request,
            audit: &audit,
        }
        .run()
        .unwrap();
        assert_eq!(confirmed, expected_confirmed);
        assert_eq!(
            customers.sync_batch_for_author(&actor).unwrap().len(),
            expected_next_batch
        );
    }
    assert_eq!(server.submitted, 102);
    assert_eq!(server.records.len(), 100);
    let retained: i64 = Connection::open(store.path())
        .unwrap()
        .query_row("SELECT count(*) FROM customer_sync_outbox", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(retained, 2);
}

#[test]
fn incoming_customer_pages_catch_up_and_resume_without_an_outbox() {
    let source = TempDir::new().unwrap();
    let (source_store, writer, author) = authorized_service(&source, 42, 51, MANAGER_RELATION);
    let _source_connection = keep_wal_open(&source_store);
    let directory = TempDir::new().unwrap();
    let (store, customers, actor) = authorized_service(&directory, 43, 51, MANAGER_RELATION);
    let database_connection = keep_wal_open(&store);
    let mut server = customer_server();
    server.records = customer_history(&writer, &author, 51);
    let (mut engine, request, audit) = sync_engine(&store, &actor);
    download(
        &customers,
        &mut engine,
        &mut server,
        &actor,
        &request,
        &audit,
    )
    .unwrap();
    assert_eq!(
        engine.metadata().checkpoint,
        Some(Checkpoint::new(Uuid::from_u128(51)))
    );
    assert_eq!(
        store
            .search_customers(&actor.scope, "", None, None, 100)
            .unwrap()
            .items
            .len(),
        51
    );
    assert_eq!(outbox_count(&store), 0);
    assert_eq!(server.submitted, 0);
    assert!(store.pending_publications(64).unwrap().len() >= 51);

    let initial = server.records[0].clone();
    let current = writer
        .get(
            &author,
            &GetCustomer {
                customer_id: CustomerId::new(initial.record_id.value()),
                branch_id: None,
            },
        )
        .unwrap();
    let context = mutation(author, 3000, 20);
    let updated = writer
        .update(
            &context,
            &UpdateCustomer {
                customer_id: current.id,
                expected_revision: 1,
                name: CustomerName::parse("عميل محدث").unwrap(),
                phone: current.phone,
                address: None,
                notes: None,
            },
        )
        .unwrap();
    server.records.push(
        sync_change(
            &context,
            &updated.customer,
            Some(1),
            ChangeId::new(Uuid::new_v4()),
        )
        .unwrap(),
    );
    // Close the fixture connection before exercising persisted-state recovery.
    drop(database_connection);
    drop(engine);
    let reopened = AuthorityStore::open(directory.path()).unwrap();
    let (mut engine, request, audit) = sync_engine(&reopened, &actor);
    assert_eq!(
        engine.metadata().checkpoint,
        Some(Checkpoint::new(Uuid::from_u128(51)))
    );
    download(
        &customers,
        &mut engine,
        &mut server,
        &actor,
        &request,
        &audit,
    )
    .unwrap();
    assert_eq!(
        customers
            .get(
                &actor,
                &GetCustomer {
                    customer_id: current.id,
                    branch_id: None
                }
            )
            .unwrap()
            .name,
        updated.customer.name
    );
    assert_eq!(outbox_count(&reopened), 0);
    assert_eq!(server.submitted, 0);
}

#[test]
fn incoming_history_preserves_unstaged_local_edits_and_later_conflicts() {
    let directory = TempDir::new().unwrap();
    let (store, customers, actor) = authorized_service(&directory, 44, 51, MANAGER_RELATION);
    let created = customers
        .create(
            &mutation(actor.clone(), 4000, 10),
            &create_command("عميل محلي", "777123456"),
        )
        .unwrap();
    let original = customers.sync_batch(&actor.scope, 1).unwrap().remove(0);
    let mut server = customer_server();
    server.records.push(original.clone());
    let (mut engine, request, audit) = sync_engine(&store, &actor);
    download(
        &customers,
        &mut engine,
        &mut server,
        &actor,
        &request,
        &audit,
    )
    .unwrap();
    let edit = customers
        .update(
            &mutation(actor.clone(), 4001, 20),
            &UpdateCustomer {
                customer_id: created.customer.id,
                expected_revision: 1,
                name: CustomerName::parse("تعديل محلي محفوظ").unwrap(),
                phone: created.customer.phone,
                address: None,
                notes: None,
            },
        )
        .unwrap();
    server
        .records
        .push(competing_update(original, &edit.customer));
    download(
        &customers,
        &mut engine,
        &mut server,
        &actor,
        &request,
        &audit,
    )
    .unwrap();
    assert_eq!(contact(&customers, &actor, edit.customer.id), edit.customer);
    assert_eq!(outbox_count(&store), 1);
    assert_eq!(server.submitted, 0);
    drop(engine);
    let (mut engine, request, audit) = sync_engine(&store, &actor);
    server.submitted = 1; // The next submission receives a server conflict.
    CustomerSyncCycle {
        customers: &customers,
        engine: &mut engine,
        transport: &mut server,
        actor: &actor,
        server_scope: &actor.scope,
        request: &request,
        audit: &audit,
    }
    .run()
    .unwrap();
    let conflicted = contact(&customers, &actor, edit.customer.id);
    assert_eq!(conflicted.name, edit.customer.name);
    assert_eq!(conflicted.sync_state, CustomerSyncState::Conflicted);
    drop(engine);
    let (mut engine, request, audit) = sync_engine(&store, &actor);
    download(
        &customers,
        &mut engine,
        &mut server,
        &actor,
        &request,
        &audit,
    )
    .unwrap();
    assert_eq!(contact(&customers, &actor, edit.customer.id), conflicted);
    assert_eq!(outbox_count(&store), 1);
}

#[test]
fn incoming_customer_history_rejects_foreign_branches_and_revoked_readers() {
    let source = TempDir::new().unwrap();
    let (_, writer, author) = authorized_service(&source, 42, 52, MANAGER_RELATION);
    let created = writer
        .create(
            &mutation(author.clone(), 5000, 10),
            &create_command("عميل فرع آخر", "777123456"),
        )
        .unwrap();
    let directory = TempDir::new().unwrap();
    let (store, customers, actor) = authorized_service(&directory, 43, 51, MANAGER_RELATION);
    let mut server = customer_server();
    let mut foreign = writer.sync_batch(&author.scope, 1).unwrap().remove(0);
    server.records.push(foreign.clone());
    let (mut engine, request, audit) = sync_engine(&store, &actor);
    assert!(
        download(
            &customers,
            &mut engine,
            &mut server,
            &actor,
            &request,
            &audit
        )
        .is_err()
    );
    assert!(engine.metadata().checkpoint.is_none());
    assert!(
        store
            .get_customer(&actor.scope, created.customer.id)
            .unwrap()
            .is_none()
    );
    foreign.scope = actor.scope.clone();
    server.records = vec![foreign];
    Connection::open(store.path())
        .unwrap()
        .execute(
            "DELETE FROM scope_relationships WHERE relation = ?1",
            [MANAGER_RELATION],
        )
        .unwrap();
    assert!(
        download(
            &customers,
            &mut engine,
            &mut server,
            &actor,
            &request,
            &audit
        )
        .is_err()
    );
    assert!(engine.metadata().checkpoint.is_none());
    assert!(
        store
            .get_customer(&actor.scope, created.customer.id)
            .unwrap()
            .is_none()
    );
}

fn contact(
    customers: &CustomerService,
    actor: &AuthorizationContext,
    customer_id: CustomerId,
) -> eitmad_contracts::customer::Customer {
    customers
        .get(
            actor,
            &GetCustomer {
                customer_id,
                branch_id: None,
            },
        )
        .unwrap()
}

fn customer_history(
    writer: &CustomerService,
    author: &AuthorizationContext,
    count: u128,
) -> Vec<ChangeRecord> {
    let mut server = customer_server();
    for index in 0..count {
        let context = mutation(author.clone(), 2000 + index, 10);
        let created = writer
            .create(&context, &create_command("عميل اختباري", "777123456"))
            .unwrap();
        server.records.push(
            sync_change(
                &context,
                &created.customer,
                None,
                ChangeId::new(Uuid::new_v4()),
            )
            .unwrap(),
        );
    }
    server.records
}

fn competing_update(
    original: ChangeRecord,
    customer: &eitmad_contracts::customer::Customer,
) -> ChangeRecord {
    let mut remote = original;
    remote.change_id = ChangeId::new(Uuid::new_v4());
    remote.idempotency_key = IdempotencyKey::new(Uuid::new_v4());
    remote.base_revision = Some(1);
    remote.revision = 2;
    let payload = CustomerSyncPayload {
        customer_id: customer.id,
        name: CustomerName::parse("تعديل جهاز آخر").unwrap(),
        phone: customer.phone.clone(),
        address: None,
        notes: None,
        revision: 2,
    };
    remote.payload.as_mut().unwrap().base64 =
        STANDARD.encode(serde_json::to_vec(&payload).unwrap());
    remote
}

fn customer_server() -> CustomerServer {
    CustomerServer {
        transport: SimulatedTransport::new(hello(), hello(), RetryPolicy::default()),
        records: vec![],
        submitted: 0,
    }
}

fn download(
    customers: &CustomerService,
    engine: &mut SyncEngine,
    server: &mut CustomerServer,
    actor: &AuthorizationContext,
    request: &AuthorizationRequest,
    audit: &BoundaryAuditContext,
) -> Result<(), crate::CustomerSyncError> {
    CustomerSyncCycle {
        customers,
        engine,
        transport: server,
        actor,
        server_scope: &actor.scope,
        request,
        audit,
    }
    .download()
}

fn outbox_count(store: &AuthorityStore) -> i64 {
    Connection::open(store.path())
        .unwrap()
        .query_row("SELECT count(*) FROM customer_sync_outbox", [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn sync_engine(
    store: &AuthorityStore,
    actor: &AuthorizationContext,
) -> (SyncEngine, AuthorizationRequest, BoundaryAuditContext) {
    let request = AuthorizationRequest {
        action: ActionId::parse("eitmad.action.sync.write.v1").unwrap(),
        object: ScopedObject {
            tenant_id: actor.tenant_id,
            workspace_id: None,
            kind: ObjectKind::parse("branch").unwrap(),
            id: ObjectId::new(actor.scope.id.value()),
        },
        attributes: std::collections::BTreeMap::new(),
    };
    let relation = RelationId::parse(MANAGER_RELATION).unwrap();
    let policy = RelationshipPolicy::new(
        vec![RelationshipTuple {
            subject: TupleSubject::Principal(RelationshipSubject {
                principal_id: actor.identity.principal_id,
                principal_kind: actor.identity.principal_kind,
            }),
            relation: relation.clone(),
            object: request.object.clone(),
            condition: None,
        }],
        vec![PermissionRule {
            action: request.action.clone(),
            object_kind: request.object.kind.clone(),
            relations: vec![relation],
            inherits_via: vec![],
        }],
    )
    .unwrap();
    let audit = BoundaryAuditContext {
        kind: BoundaryKind::Sync,
        operation: "eitmad.customer.sync-cycle.v1".into(),
        target: AuditTarget {
            kind: "branch".into(),
            identifiers: vec![],
        },
        occurred_at: UnixMillis(1000),
        correlation_id: CorrelationId::new(Uuid::new_v4()),
        causation_id: None,
        idempotency_key: None,
        extension_points: vec![],
    };
    let engine = SyncEngine::open(
        store.clone(),
        actor.scope.clone(),
        SyncMode::LocalFirst,
        SyncAuthorization::new(AuthorizationGate::new(policy, store.clone())),
        actor,
        &audit,
    )
    .unwrap();
    (engine, request, audit)
}

fn hello() -> PeerHello {
    PeerHello {
        peer_kind: PeerKind::Engine,
        product_version: serde_json::from_str::<ReleaseVersion>("\"0.0.0\"").unwrap(),
        protocols: vec![SupportedProtocol {
            major: 1,
            minimum_minor: eitmad_contracts::PROTOCOL_VERSION.minor,
            maximum_minor: eitmad_contracts::PROTOCOL_VERSION.minor,
        }],
        capabilities: vec![CapabilityId::parse("eitmad.capability.sync.v1").unwrap()],
        required_capabilities: vec![],
        schemas: vec![SchemaSupport {
            schema_id: SchemaId::parse(CUSTOMER_SCHEMA_ID).unwrap(),
            minimum_version: 1,
            maximum_version: 1,
            required: true,
        }],
    }
}

struct CustomerServer {
    transport: SimulatedTransport,
    records: Vec<ChangeRecord>,
    submitted: usize,
}

impl CustomerServer {
    fn submit(&mut self, change: &ChangeRecord) -> LocalChangeDisposition {
        self.submitted += 1;
        match self.submitted {
            1 => LocalChangeDisposition::Rejected {
                reason: ErrorCodeRef::parse("eitmad.error.customer-denied.v1").unwrap(),
            },
            2 => LocalChangeDisposition::Conflicted {
                conflict_id: ConflictId::new(Uuid::new_v4()),
            },
            index => {
                self.records.push(change.clone());
                if index % 2 == 0 {
                    LocalChangeDisposition::Applied {
                        authoritative_change: change.clone(),
                    }
                } else {
                    LocalChangeDisposition::Replayed {
                        authoritative_change: change.clone(),
                    }
                }
            }
        }
    }
}

impl SyncTransport for CustomerServer {
    fn kind(&self) -> TransportKind {
        self.transport.kind()
    }
    fn connect(&mut self, now: UnixMillis) -> Result<NegotiatedSession, TransportFailure> {
        self.transport.connect(now)
    }
    fn disconnect(&mut self, now: UnixMillis) {
        self.transport.disconnect(now);
    }
    fn send(
        &mut self,
        frame: &SyncTransportFrame,
        now: UnixMillis,
    ) -> Result<(), TransportFailure> {
        self.transport.send(frame, now)?;
        let SyncTransportPayload::Message(message) = &frame.payload else {
            panic!("expected sync message")
        };
        let response = match message.as_ref() {
            SyncMessage::SubmitLocal(submission) => SyncMessage::LocalResult(LocalChangeResult {
                submitted_change_id: submission.change.change_id,
                disposition: self.submit(&submission.change),
            }),
            SyncMessage::Pull(request) => {
                let start = request
                    .after
                    .map_or(0, |c| usize::try_from(c.value().as_u128()).unwrap());
                let end = (start + 50).min(self.records.len());
                SyncMessage::Changes(
                    ChangeBatch::new(
                        DeliveryId::new(Uuid::new_v4()),
                        IdempotencyKey::new(Uuid::new_v4()),
                        request.after,
                        Checkpoint::new(Uuid::from_u128(end as u128)),
                        self.records[start..end].to_vec(),
                        end < self.records.len(),
                    )
                    .unwrap(),
                )
            }
            SyncMessage::Acknowledge(ack) => SyncMessage::Acknowledge(ack.clone()),
            _ => panic!("unexpected sync message"),
        };
        let mut reply = frame.clone();
        reply.payload = SyncTransportPayload::Message(Box::new(response));
        self.transport.inject_incoming(reply);
        Ok(())
    }
    fn receive(&mut self, now: UnixMillis) -> Result<ReceiveOutcome, TransportFailure> {
        self.transport.receive(now)
    }
    fn cancel(
        &mut self,
        stream: SyncStreamId,
        correlation: CorrelationId,
        reason: SyncCancellationReason,
        now: UnixMillis,
    ) -> Result<(), TransportFailure> {
        self.transport.cancel(stream, correlation, reason, now)
    }
    fn health(&self) -> &ConnectionHealth {
        self.transport.health()
    }
    fn negotiated_session(&self) -> Option<&NegotiatedSession> {
        self.transport.negotiated_session()
    }
}
