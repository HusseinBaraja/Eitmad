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

#[test]
fn customer_sync_counts_confirmations_while_the_next_batch_stays_full() {
    let directory = TempDir::new().unwrap();
    let (store, customers, actor) = authorized_service(&directory, 41, 51, MANAGER_RELATION);
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
