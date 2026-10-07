use super::*;
use crate::{
    QUOTATION_DRAFT_SCHEMA, QuotationDraftError, QuotationDraftService, QuotationDraftSyncCycle,
    QuotationDraftSyncError,
};
use eitmad_authorization::{
    AuthorizationGate, BoundaryAuditContext, BoundaryKind, RelationshipPolicy,
};
use eitmad_contracts::{
    authorization::{
        ActionId, AuthorizationRequest, ObjectId, ObjectKind, PermissionRule, RelationshipTuple,
        ScopedObject, TupleSubject,
    },
    quotation_draft::*,
    sync::{
        ChangeBatch, ChangeRecord, Checkpoint, ConflictId, DeliveryId, LocalChangeDisposition,
        LocalChangeResult, SyncMessage, SyncMode,
    },
    sync_transport::{
        SyncCancellationReason, SyncStreamId, SyncTransportFrame, SyncTransportPayload,
    },
    transport::{CapabilityId, SchemaId},
    updates::ReleaseVersion,
    versioning::{NegotiatedSession, PeerHello, PeerKind, SchemaSupport, SupportedProtocol},
};
use eitmad_sync::{
    ConnectionHealth, ReceiveOutcome, RetryPolicy, SimulatedTransport, SyncAuthorization,
    SyncEngine, SyncEngineError, SyncTransport, TransportFailure, TransportKind,
};
use std::collections::BTreeMap;

/// Uses the existing simulated protocol transport and real `SQLite` authority.
struct DraftSyncFixture {
    _directory: TempDir,
    store: AuthorityStore,
    actor: AuthorizationContext,
    input: EvaluateQuotation,
    drafts: QuotationDraftService,
    engine: SyncEngine,
    request: AuthorizationRequest,
    audit: BoundaryAuditContext,
    server: DraftServer,
    catalog: eitmad_contracts::identity::ScopeRef,
}

impl DraftSyncFixture {
    fn new() -> Self {
        let (directory, store, _, actor, input, _) = fixture_evaluation();
        let request = AuthorizationRequest {
            action: ActionId::parse("eitmad.action.sync.write.v1").unwrap(),
            object: ScopedObject {
                tenant_id: actor.tenant_id,
                workspace_id: actor.workspace_id,
                kind: ObjectKind::parse("branch").unwrap(),
                id: ObjectId::new(actor.scope.id.value()),
            },
            attributes: BTreeMap::new(),
        };
        let relation = RelationId::parse(RECEPTIONIST_RELATION).unwrap();
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
            operation: "eitmad.quotation-draft.sync.v1".into(),
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
        let engine = SyncEngine::open_domain(
            store.clone(),
            actor.scope.clone(),
            (
                SyncMode::LocalFirst,
                SchemaId::parse(QUOTATION_DRAFT_SCHEMA).unwrap(),
            ),
            SyncAuthorization::new(AuthorizationGate::new(policy, store.clone())),
            &actor,
            &audit,
        )
        .unwrap();
        let catalog = eitmad_contracts::identity::ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(actor.tenant_id.value()),
        };
        let hello = PeerHello {
            peer_kind: PeerKind::Engine,
            product_version: serde_json::from_str::<ReleaseVersion>("\"0.0.0\"").unwrap(),
            protocols: vec![SupportedProtocol {
                major: 1,
                minimum_minor: 19,
                maximum_minor: 19,
            }],
            capabilities: vec![CapabilityId::parse("eitmad.capability.sync.v1").unwrap()],
            required_capabilities: vec![],
            schemas: vec![SchemaSupport {
                schema_id: SchemaId::parse(QUOTATION_DRAFT_SCHEMA).unwrap(),
                minimum_version: 1,
                maximum_version: 1,
                required: true,
            }],
        };
        Self {
            _directory: directory,
            drafts: QuotationDraftService::new(
                store.clone(),
                AuthorizationService::new(store.clone()),
            ),
            store,
            actor,
            input,
            engine,
            request,
            audit,
            catalog,
            server: DraftServer {
                transport: SimulatedTransport::new(hello.clone(), hello, RetryPolicy::default()),
                records: vec![],
                submitted: vec![],
                messages: vec![],
                exception: None,
            },
        }
    }

    fn run(&mut self) -> Result<(), QuotationDraftSyncError> {
        QuotationDraftSyncCycle {
            drafts: &self.drafts,
            engine: &mut self.engine,
            transport: &mut self.server,
            actor: &self.actor,
            server_scope: &self.actor.scope,
            server_catalog_scope: &self.catalog,
            request: &self.request,
            audit: &self.audit,
        }
        .run()
    }

    fn create(&self) -> QuotationDraft {
        self.drafts
            .create(
                &mutation(self.actor.clone(), 1000),
                &CreateQuotationDraft {
                    intent: self.input.clone(),
                },
            )
            .unwrap()
    }

    fn update(&self, draft: &QuotationDraft) -> QuotationDraft {
        let mut intent = draft.snapshot.intent.clone();
        intent.discount_basis_points = 100;
        self.drafts
            .update(
                &mutation(self.actor.clone(), 1001),
                &UpdateQuotationDraft {
                    draft_id: draft.snapshot.id,
                    expected_revision: draft.snapshot.revision,
                    intent,
                },
            )
            .unwrap()
    }

    fn get(&self, id: QuotationDraftId) -> QuotationDraft {
        self.drafts
            .get(&self.actor, &GetQuotationDraft { draft_id: id })
            .unwrap()
    }
}

/// Replies on the same stream, with deterministic server history checkpoints.
struct DraftServer {
    transport: SimulatedTransport,
    records: Vec<ChangeRecord>,
    submitted: Vec<ChangeRecord>,
    messages: Vec<SyncMessage>,
    exception: Option<LocalChangeDisposition>,
}
impl SyncTransport for DraftServer {
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
        self.messages.push(*message.clone());
        let response = match message.as_ref() {
            SyncMessage::SubmitLocal(submission) => {
                self.submitted.push(submission.change.clone());
                let disposition = self.exception.clone().unwrap_or_else(|| {
                    if !self.records.contains(&submission.change) {
                        self.records.push(submission.change.clone());
                    }
                    LocalChangeDisposition::Applied {
                        authoritative_change: submission.change.clone(),
                    }
                });
                SyncMessage::LocalResult(LocalChangeResult {
                    submitted_change_id: submission.change.change_id,
                    disposition,
                })
            }
            SyncMessage::Pull(request) => {
                let start = request
                    .after
                    .map_or(0, |c| usize::try_from(c.value().as_u128()).unwrap());
                SyncMessage::Changes(
                    ChangeBatch::new(
                        DeliveryId::new(Uuid::new_v4()),
                        IdempotencyKey::new(Uuid::new_v4()),
                        request.after,
                        Checkpoint::new(Uuid::from_u128(self.records.len() as u128)),
                        self.records[start..].to_vec(),
                        false,
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

#[test]
fn quotation_draft_projection_failure_replays_before_a_later_local_edit() {
    // Cover both an absent engine record and an engine one revision behind.
    for existing_revision in [false, true] {
        let mut f = DraftSyncFixture::new();
        let created = f.create();
        let incoming = if existing_revision {
            f.run().unwrap();
            let updated = f.update(&created);
            f.drafts
                .sync_batch(&f.actor, 50)
                .unwrap()
                .into_iter()
                .find(|c| c.revision == updated.snapshot.revision)
                .unwrap()
        } else {
            f.drafts.sync_batch(&f.actor, 50).unwrap().remove(0)
        };
        f.drafts
            .project_confirmed(&f.actor, &incoming, f.audit.correlation_id)
            .unwrap();
        f.server.records.push(incoming);
        // Projection has committed, but checkpoint persistence still fails.
        let db = rusqlite::Connection::open(f.store.path()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_draft_checkpoint BEFORE UPDATE ON sync_scopes WHEN NEW.schema_id='eitmad.schema.quotation-draft.v1' AND json_extract(CAST(NEW.state_json AS TEXT),'$.metadata.checkpoint') != json_extract(CAST(OLD.state_json AS TEXT),'$.metadata.checkpoint') OR NEW.schema_id='eitmad.schema.quotation-draft.v1' AND json_extract(CAST(NEW.state_json AS TEXT),'$.metadata.checkpoint') IS NOT NULL AND json_extract(CAST(OLD.state_json AS TEXT),'$.metadata.checkpoint') IS NULL BEGIN SELECT RAISE(ABORT,'synthetic checkpoint failure'); END;").unwrap();
        let result = f.run();
        assert!(
            matches!(
                result,
                Err(QuotationDraftSyncError::Engine(
                    SyncEngineError::StorageUnavailable
                ))
            ),
            "{result:?}"
        );
        db.execute_batch("DROP TRIGGER fail_draft_checkpoint")
            .unwrap();
        let confirmed = f.get(created.snapshot.id);
        assert_eq!(confirmed.sync_state, QuotationDraftSyncState::Confirmed);
        let edit = f
            .drafts
            .update(
                &mutation(f.actor.clone(), 1002),
                &UpdateQuotationDraft {
                    draft_id: confirmed.snapshot.id,
                    expected_revision: confirmed.snapshot.revision,
                    intent: confirmed.snapshot.intent.clone(),
                },
            )
            .unwrap();
        f.engine = SyncEngine::open_domain(
            f.store.clone(),
            f.actor.scope.clone(),
            (
                SyncMode::LocalFirst,
                SchemaId::parse(QUOTATION_DRAFT_SCHEMA).unwrap(),
            ),
            sync_authorization(&f),
            &f.actor,
            &f.audit,
        )
        .unwrap();
        f.server.messages.clear();
        f.run().unwrap();
        assert!(matches!(
            f.server.messages.first(),
            Some(SyncMessage::Pull(_))
        ));
        assert_eq!(
            f.server.submitted.last().unwrap().revision,
            edit.snapshot.revision
        );
        assert_eq!(f.get(created.snapshot.id).snapshot, edit.snapshot);
        assert_eq!(
            f.get(created.snapshot.id).sync_state,
            QuotationDraftSyncState::Confirmed
        );
        assert!(f.engine.pending_changes().is_empty());
        assert!(f.drafts.sync_batch(&f.actor, 50).unwrap().is_empty());
    }
}

fn sync_authorization(f: &DraftSyncFixture) -> SyncAuthorization {
    let relation = RelationId::parse(RECEPTIONIST_RELATION).unwrap();
    SyncAuthorization::new(AuthorizationGate::new(
        RelationshipPolicy::new(
            vec![RelationshipTuple {
                subject: TupleSubject::Principal(RelationshipSubject {
                    principal_id: f.actor.identity.principal_id,
                    principal_kind: f.actor.identity.principal_kind,
                }),
                relation: relation.clone(),
                object: f.request.object.clone(),
                condition: None,
            }],
            vec![PermissionRule {
                action: f.request.action.clone(),
                object_kind: f.request.object.kind.clone(),
                relations: vec![relation],
                inherits_via: vec![],
            }],
        )
        .unwrap(),
        f.store.clone(),
    ))
}

#[test]
fn quotation_draft_held_submission_recovers_after_exception_write_failure() {
    for state in [
        QuotationDraftSyncState::Rejected,
        QuotationDraftSyncState::Conflicted,
    ] {
        let mut f = DraftSyncFixture::new();
        let created = f.create();
        let edited = f.update(&created);
        let conflict_id = ConflictId::new(Uuid::from_u128(2000));
        f.server.exception = Some(if state == QuotationDraftSyncState::Conflicted {
            LocalChangeDisposition::Conflicted { conflict_id }
        } else {
            LocalChangeDisposition::Rejected {
                reason: eitmad_contracts::sync::ErrorCodeRef::parse(
                    "eitmad.error.quotation-draft-invalid.v1",
                )
                .unwrap(),
            }
        });
        let db = rusqlite::Connection::open(f.store.path()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_draft_exception BEFORE INSERT ON mutation_audit WHEN NEW.operation='eitmad.quotation-draft.sync-state.v1' BEGIN SELECT RAISE(ABORT,'synthetic status failure'); END;").unwrap();
        let result = f.run();
        assert!(
            matches!(
                result,
                Err(QuotationDraftSyncError::QuotationDraft(
                    QuotationDraftError::Unavailable
                ))
            ),
            "{result:?}"
        );
        assert_eq!(
            f.get(created.snapshot.id).sync_state,
            QuotationDraftSyncState::Pending
        );
        assert!(f.engine.pending_changes().is_empty());
        assert_eq!(f.drafts.sync_batch(&f.actor, 50).unwrap().len(), 2);
        db.execute_batch("DROP TRIGGER fail_draft_exception")
            .unwrap();
        f.engine = SyncEngine::open_domain(
            f.store.clone(),
            f.actor.scope.clone(),
            (
                SyncMode::LocalFirst,
                SchemaId::parse(QUOTATION_DRAFT_SCHEMA).unwrap(),
            ),
            sync_authorization(&f),
            &f.actor,
            &f.audit,
        )
        .unwrap();
        f.run().unwrap();
        assert_eq!(f.get(created.snapshot.id).snapshot, edited.snapshot);
        assert_eq!(f.get(created.snapshot.id).sync_state, state);
        assert_eq!(f.server.submitted.len(), 2);
        assert!(f.engine.pending_changes().is_empty());
        assert!(f.drafts.sync_batch(&f.actor, 50).unwrap().is_empty());
        assert!(
            f.server
                .messages
                .iter()
                .any(|m| matches!(m, SyncMessage::Pull(_)))
        );
        f.run().unwrap();
        assert_eq!(f.server.submitted.len(), 2);
    }
}

#[test]
fn quotation_draft_failure_audit_matches_denial_conflict_and_validation() {
    let f = DraftSyncFixture::new();
    let created = f.create();
    let mut foreign = f.input.clone();
    match &mut foreign.lines[0].configuration.selection.target {
        PriceTarget::Product(target) => target.scope.id = ScopeId::new(Uuid::new_v4()),
        PriceTarget::Furniture(target) => target.scope.id = ScopeId::new(Uuid::new_v4()),
    }
    let denied = mutation(f.actor.clone(), 1100);
    assert_eq!(
        f.drafts
            .create(&denied, &CreateQuotationDraft { intent: foreign }),
        Err(QuotationDraftError::Denied)
    );
    assert_failure_audit(
        &f.store,
        &denied,
        "denied",
        "eitmad.error.authorization-denied.v1",
    );

    let conflict = mutation(f.actor.clone(), 1101);
    assert_eq!(
        f.drafts.update(
            &conflict,
            &UpdateQuotationDraft {
                draft_id: created.snapshot.id,
                expected_revision: 0,
                intent: f.input.clone(),
            }
        ),
        Err(QuotationDraftError::Conflict {
            expected: Some(0),
            actual: Some(1)
        })
    );
    assert_failure_audit(
        &f.store,
        &conflict,
        "conflict",
        "eitmad.error.quotation-draft-conflict.v1",
    );

    f.drafts
        .mark_sync_exception(
            &f.actor,
            created.snapshot.id,
            QuotationDraftSyncState::Conflicted,
            Some(ConflictId::new(Uuid::new_v4())),
            f.audit.correlation_id,
            UnixMillis(1102),
        )
        .unwrap();
    let unresolved = mutation(f.actor.clone(), 1103);
    assert_eq!(
        f.drafts.update(
            &unresolved,
            &UpdateQuotationDraft {
                draft_id: created.snapshot.id,
                expected_revision: 1,
                intent: f.input.clone(),
            }
        ),
        Err(QuotationDraftError::UnresolvedConflict)
    );
    assert_failure_audit(
        &f.store,
        &unresolved,
        "conflict",
        "eitmad.error.quotation-draft-conflict.v1",
    );

    let invalid = mutation(f.actor.clone(), 1104);
    let mut intent = f.input.clone();
    intent.lines.clear();
    assert!(matches!(
        f.drafts.create(&invalid, &CreateQuotationDraft { intent }),
        Err(QuotationDraftError::Validation(_))
    ));
    assert_failure_audit(
        &f.store,
        &invalid,
        "invalid",
        "eitmad.error.quotation-draft-invalid.v1",
    );
}

fn assert_failure_audit(
    store: &AuthorityStore,
    context: &MutationContext,
    outcome: &str,
    code: &str,
) {
    let db = rusqlite::Connection::open(store.path()).unwrap();
    let actual: (String, String) = db.query_row(
        "SELECT outcome,json_extract(redacted_error,'$.code') FROM mutation_audit WHERE correlation_id=?1 AND operation IN ('eitmad.quotation-draft.create.v1','eitmad.quotation-draft.update.v1')",
        [context.correlation_id.value().to_string()], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(actual, (format!("\"{outcome}\""), code.to_owned()));
}
