use super::*;
use crate::{QuotationDraftError as E, QuotationDraftService};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_contracts::{quotation_approval::*, quotation_draft::*, sync::*};
use std::sync::Mutex;

struct Server {
    confirmed: ChangeRecord,
    fail: Mutex<bool>,
    receipt_exists: bool,
    reconciled: Mutex<Vec<ReconcileResolution<ConfirmQuotationDraftResolution>>>,
}
impl crate::DiscountApprovalServer for Server {
    fn reconcile_draft_resolution(
        &self,
        actor: &AuthorizationContext,
        input: &ReconcileResolution<ConfirmQuotationDraftResolution>,
        _: UnixMillis,
    ) -> Result<Option<ChangeRecord>, E> {
        assert_ne!(
            actor.identity.principal_id.value(),
            input.original_user.value()
        );
        if *self.fail.lock().unwrap() {
            return Err(E::Unavailable);
        }
        self.reconciled.lock().unwrap().push(input.clone());
        Ok(self.receipt_exists.then(|| self.confirmed.clone()))
    }

    fn draft_recovery(
        &self,
        _: &AuthorizationContext,
        _: QuotationDraftId,
        _: UnixMillis,
    ) -> Result<Vec<ChangeRecord>, E> {
        if *self.fail.lock().unwrap() {
            Err(E::Unavailable)
        } else {
            Ok(vec![self.confirmed.clone()])
        }
    }
    fn resolve_draft(
        &self,
        _: &AuthorizationContext,
        _: &ConfirmQuotationDraftResolution,
        _: UnixMillis,
    ) -> Result<ChangeRecord, E> {
        if *self.fail.lock().unwrap() {
            Err(E::Unavailable)
        } else {
            Ok(self.confirmed.clone())
        }
    }
    fn draft_review(
        &self,
        _: &AuthorizationContext,
        _: &GetQuotationDraftReview,
        _: UnixMillis,
    ) -> Result<QuotationDraftReview, E> {
        Ok(QuotationDraftReview {
            current: decode(&self.confirmed),
            items: vec![],
            next: None,
            pending_reviews: vec![],
            pending_resolution: None,
            can_resolve: true,
        })
    }
    fn transition(
        &self,
        _: &AuthorizationContext,
        _: &ConfirmDiscountApproval,
        _: UnixMillis,
    ) -> Result<Option<DiscountApproval>, crate::ApprovalError> {
        unreachable!()
    }
    fn list(
        &self,
        _: &AuthorizationContext,
        _: &ListDiscountApprovals,
        _: UnixMillis,
    ) -> Result<DiscountApprovalPage, crate::ApprovalError> {
        unreachable!()
    }
    fn watch(
        &self,
        _: &AuthorizationContext,
        _: &std::sync::atomic::AtomicBool,
        _: &mut dyn FnMut(DiscountApprovalNotice),
    ) -> Result<(), crate::ApprovalError> {
        unreachable!()
    }
}
fn decode(change: &ChangeRecord) -> QuotationDraftSnapshot {
    serde_json::from_slice(
        &STANDARD
            .decode(&change.payload.as_ref().unwrap().base64)
            .unwrap(),
    )
    .unwrap()
}
fn authorize_manager(
    store: &AuthorityStore,
    branch: &AuthorizationContext,
) -> AuthorizationContext {
    let manager = AuthorizationContext {
        scope: branch.scope.clone(),
        ..actor(100, 50)
    };
    let owner = AuthorizationContext {
        scope: branch.scope.clone(),
        ..actor(99, 50)
    };
    AuthorizationService::new(store.clone())
        .grant_relationship(
            &mutation(owner, 4000),
            &GrantScopeRelationship {
                expected_policy_version: 2,
                subject: RelationshipSubject {
                    principal_id: manager.identity.principal_id,
                    principal_kind: PrincipalKind::User,
                },
                relation: RelationId::parse(MANAGER_RELATION).unwrap(),
            },
        )
        .unwrap();
    manager
}
struct ResolutionFixture {
    directory: TempDir,
    store: AuthorityStore,
    drafts: QuotationDraftService,
    writer: AuthorizationContext,
    manager: AuthorizationContext,
    pending: Vec<ChangeRecord>,
    later: QuotationDraft,
    server: Server,
    command: ResolveQuotationDraft,
    context: MutationContext,
    update: UpdateQuotationDraft,
}
fn receipt(
    pending: &ChangeRecord,
    submitted: &ChangeRecord,
    manager: &AuthorizationContext,
) -> ChangeRecord {
    let mut confirmed = pending.clone();
    let mut snapshot = decode(&confirmed);
    snapshot.revision = 3;
    confirmed.base_revision = Some(2);
    confirmed.revision = 3;
    confirmed.change_id = ChangeId::new(Uuid::new_v4());
    confirmed.idempotency_key = mutation(manager.clone(), 4201).idempotency_key;
    confirmed.payload.as_mut().unwrap().base64 =
        STANDARD.encode(serde_json::to_vec(&snapshot).unwrap());
    confirmed.merge = Some(MergeMetadata {
        strategy: MergeStrategy::DomainMerge,
        common_ancestor_revision: Some(1),
        source_changes: vec![pending.change_id, submitted.change_id],
        merged_at: UnixMillis(4200),
    });
    confirmed
}
impl ResolutionFixture {
    fn new() -> Self {
        let (directory, store, _, writer, input, _) = fixture_evaluation();
        let drafts =
            QuotationDraftService::new(store.clone(), AuthorizationService::new(store.clone()));
        let first = drafts
            .create(
                &mutation(writer.clone(), 4100),
                &CreateQuotationDraft {
                    intent: input.clone(),
                },
            )
            .unwrap();
        let submitted = drafts.sync_batch(&writer, 50).unwrap()[0].clone();
        drafts
            .project_confirmed(
                &writer,
                &submitted,
                mutation(writer.clone(), 4101).correlation_id,
            )
            .unwrap();
        let mut update = UpdateQuotationDraft {
            draft_id: first.snapshot.id,
            expected_revision: 1,
            intent: input,
        };
        update.intent.discount_basis_points = 200;
        drafts
            .update(&mutation(writer.clone(), 4102), &update)
            .unwrap();
        update.expected_revision = 2;
        update.intent.discount_basis_points = 300;
        let mut later = drafts
            .update(&mutation(writer.clone(), 4103), &update)
            .unwrap();
        for revision in 3..6 {
            update.expected_revision = revision;
            update.intent.discount_basis_points += 100;
            later = drafts
                .update(
                    &mutation(writer.clone(), 4103 + u128::from(revision)),
                    &update,
                )
                .unwrap();
        }
        let pending = drafts.sync_batch(&writer, 50).unwrap();
        drafts
            .mark_sync_exception(
                &writer,
                first.snapshot.id,
                QuotationDraftSyncState::Conflicted,
                None,
                mutation(writer.clone(), 4104).correlation_id,
                UnixMillis(4104),
            )
            .unwrap();
        let manager = authorize_manager(&store, &writer);
        let confirmed = receipt(&pending[0], &submitted, &manager);
        let server = Server {
            confirmed: confirmed.clone(),
            fail: Mutex::new(true),
            receipt_exists: true,
            reconciled: Mutex::new(vec![]),
        };
        let command = ResolveQuotationDraft {
            draft_id: first.snapshot.id,
            branch_id: writer.scope.id,
            expected_server_revision: 2,
            source: QuotationDraftResolutionSource::PendingReview(Box::new(pending[0].clone())),
            choice: QuotationDraftResolutionChoice::UseInput,
        };
        let context = mutation(manager.clone(), 4201);
        Self {
            directory,
            store,
            drafts,
            writer,
            manager,
            pending,
            later,
            server,
            command,
            context,
            update,
        }
    }
    fn uncertain(&self) {
        assert_eq!(
            self.drafts.resolve(
                &self.context,
                &self.command,
                &self.server,
                UnixMillis(i64::MAX)
            ),
            Err(E::Unavailable)
        );
        assert_eq!(
            self.store
                .get_quotation_draft(&self.writer.scope, self.command.draft_id)
                .unwrap()
                .unwrap()
                .snapshot,
            self.later.snapshot
        );
    }
    fn recover(&self, store: &AuthorityStore, drafts: &QuotationDraftService) {
        let page = drafts
            .review(
                &self.manager,
                &GetQuotationDraftReview {
                    draft_id: self.command.draft_id,
                    branch_id: self.writer.scope.id,
                    after: None,
                },
                &self.server,
                UnixMillis(i64::MAX),
            )
            .unwrap();
        assert_eq!(page.pending_resolution.unwrap().command, self.command);
        assert_eq!(page.pending_reviews.len(), 5);
        *self.server.fail.lock().unwrap() = false;
        let db = rusqlite::Connection::open(store.path()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_resolution_projection BEFORE INSERT ON mutation_audit WHEN NEW.operation='eitmad.quotation-draft.sync-project.v1' BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;").unwrap();
        assert_eq!(
            drafts.recover_pending(&self.server, UnixMillis(i64::MAX)),
            Err(crate::ApprovalError::Unavailable)
        );
        assert_eq!(
            store
                .quotation_draft_pending_reviews(&self.writer.scope, self.command.draft_id)
                .unwrap(),
            self.pending
        );
        db.execute_batch("DROP TRIGGER fail_resolution_projection;")
            .unwrap();
        assert_eq!(
            drafts
                .recover_pending(&self.server, UnixMillis(i64::MAX))
                .unwrap(),
            0
        );
        assert_eq!(
            drafts
                .resolve(
                    &self.context,
                    &self.command,
                    &self.server,
                    UnixMillis(i64::MAX)
                )
                .unwrap(),
            self.server.confirmed
        );
    }
    fn preserved(&self, store: &AuthorityStore, drafts: &QuotationDraftService) {
        assert!(drafts.sync_batch(&self.writer, 50).unwrap().is_empty());
        let held = store
            .quotation_draft_pending_reviews(&self.writer.scope, self.command.draft_id)
            .unwrap();
        assert_eq!(held, self.pending[1..]);
        assert_eq!(decode(held.last().unwrap()), self.later.snapshot);
        assert_eq!(
            drafts
                .get(
                    &self.writer,
                    &GetQuotationDraft {
                        draft_id: self.command.draft_id
                    }
                )
                .unwrap()
                .snapshot,
            decode(&self.server.confirmed)
        );
        assert!(
            !drafts
                .project_confirmed(
                    &self.writer,
                    &self.server.confirmed,
                    self.context.correlation_id
                )
                .unwrap()
        );
        assert_eq!(
            store
                .quotation_draft_pending_reviews(&self.writer.scope, self.command.draft_id)
                .unwrap(),
            held
        );
        let count:i64=rusqlite::Connection::open(store.path()).unwrap().query_row(
        "SELECT count(*) FROM mutation_audit WHERE operation='eitmad.quotation-draft.resolve.v1' AND outcome='\"succeeded\"'",[],|r|r.get(0)).unwrap();
        assert_eq!(count, 1);
        assert!(
            store
                .quotation_draft_resolution_intent(
                    &self.manager,
                    self.command.draft_id,
                    self.writer.scope.id
                )
                .unwrap()
                .is_none()
        );
    }
    fn edit_after_resolution(&mut self, drafts: &QuotationDraftService) {
        self.update.expected_revision = 3;
        let edited = drafts
            .update(&mutation(self.writer.clone(), 4203), &self.update)
            .unwrap();
        assert!(
            !drafts
                .project_confirmed(
                    &self.writer,
                    &self.server.confirmed,
                    self.context.correlation_id
                )
                .unwrap()
        );
        assert_eq!(
            drafts
                .get(
                    &self.writer,
                    &GetQuotationDraft {
                        draft_id: self.command.draft_id
                    }
                )
                .unwrap(),
            edited
        );
        assert_eq!(drafts.sync_batch(&self.writer, 50).unwrap().len(), 1);
    }
}
#[test]
fn resolution_restart_holds_later_work_preserves_prices_and_freezes_uncertain_decision() {
    let mut fixture = ResolutionFixture::new();
    fixture.uncertain();
    let reopened = AuthorityStore::open(fixture.directory.path()).unwrap();
    let drafts = QuotationDraftService::new(
        reopened.clone(),
        AuthorizationService::new(reopened.clone()),
    );
    fixture.recover(&reopened, &drafts);
    fixture.preserved(&reopened, &drafts);
    fixture.edit_after_resolution(&drafts);
}
#[test]
fn resolution_denies_receptionist_before_calling_the_server() {
    let (_directory, store, _, writer, input, _) = fixture_evaluation();
    let drafts =
        QuotationDraftService::new(store.clone(), AuthorizationService::new(store.clone()));
    let saved = drafts
        .create(
            &mutation(writer.clone(), 4300),
            &CreateQuotationDraft { intent: input },
        )
        .unwrap();
    let change = drafts.sync_batch(&writer, 50).unwrap().remove(0);
    let server = Server {
        confirmed: change,
        fail: Mutex::new(false),
        receipt_exists: true,
        reconciled: Mutex::new(vec![]),
    };
    let command = ResolveQuotationDraft {
        draft_id: saved.snapshot.id,
        branch_id: writer.scope.id,
        expected_server_revision: 1,
        source: QuotationDraftResolutionSource::Conflict(QuotationDraftConflictSource {
            conflict_id: ConflictId::new(Uuid::new_v4()),
        }),
        choice: QuotationDraftResolutionChoice::KeepCurrent,
    };
    assert_eq!(
        drafts.resolve(
            &mutation(writer, 4301),
            &command,
            &server,
            UnixMillis(i64::MAX)
        ),
        Err(E::Denied)
    );
    let count:i64=rusqlite::Connection::open(store.path()).unwrap().query_row(
        "SELECT count(*) FROM mutation_audit WHERE operation='eitmad.quotation-draft.resolve.v1' AND outcome='\"denied\"'",[],|r|r.get(0)).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn resolution_handover_after_restart_preserves_original_evidence_and_denies_removed_manager() {
    for applied in [false, true] {
        let mut fixture = ResolutionFixture::new();
        fixture.server.receipt_exists = applied;
        fixture.uncertain();
        let manager_b = replace_manager(&fixture);
        let db = rusqlite::Connection::open(fixture.store.path()).unwrap();
        let store = AuthorityStore::open(fixture.directory.path()).unwrap();
        let drafts =
            QuotationDraftService::new(store.clone(), AuthorizationService::new(store.clone()));
        let query = GetQuotationDraftReview {
            draft_id: fixture.command.draft_id,
            branch_id: fixture.writer.scope.id,
            after: None,
        };
        assert_eq!(
            drafts.review(
                &fixture.manager,
                &query,
                &fixture.server,
                UnixMillis(i64::MAX)
            ),
            Err(E::Denied)
        );
        assert_eq!(
            drafts.review(&manager_b, &query, &fixture.server, UnixMillis(i64::MAX)),
            Err(E::Unavailable)
        );
        assert!(
            store
                .pending_quotation_draft_resolution(&manager_b, query.draft_id, query.branch_id)
                .unwrap()
                .is_some()
        );
        *fixture.server.fail.lock().unwrap() = false;
        assert!(
            drafts
                .review(&manager_b, &query, &fixture.server, UnixMillis(i64::MAX))
                .unwrap()
                .pending_resolution
                .is_none()
        );
        drafts
            .review(&manager_b, &query, &fixture.server, UnixMillis(i64::MAX))
            .unwrap();
        let recovered = fixture.server.reconciled.lock().unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(
            recovered[0].original_user.value(),
            fixture.manager.identity.principal_id.value()
        );
        assert_eq!(recovered[0].request.command, fixture.command);
        assert_eq!(
            recovered[0].request.idempotency_key,
            fixture.context.idempotency_key
        );
        let (original, request, receipt, by): (String,Vec<u8>,Option<Vec<u8>>,String) = db.query_row("SELECT principal_id,request_json,receipt_json,reconciled_by FROM resolution_reconciliations", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        assert_eq!(
            original,
            fixture.manager.identity.principal_id.value().to_string()
        );
        assert_eq!(
            serde_json::from_slice::<ConfirmQuotationDraftResolution>(&request).unwrap(),
            recovered[0].request
        );
        assert_eq!(
            receipt.map(|v| serde_json::from_slice::<ChangeRecord>(&v).unwrap()),
            applied.then_some(fixture.server.confirmed)
        );
        assert_eq!(by, manager_b.identity.principal_id.value().to_string());
        let next = ConfirmQuotationDraftResolution {
            scope: manager_b.scope.clone(),
            idempotency_key: mutation(manager_b.clone(), 4501).idempotency_key,
            command: fixture.command.clone(),
        };
        store
            .stage_quotation_draft_resolution(
                &manager_b,
                &next,
                &crate::drafts::audit(
                    &mutation(manager_b.clone(), 4501),
                    "eitmad.quotation-draft.resolution-request.v1",
                    Some(query.draft_id),
                ),
            )
            .unwrap();
    }
}

fn replace_manager(fixture: &ResolutionFixture) -> AuthorizationContext {
    let manager_b = AuthorizationContext {
        scope: fixture.writer.scope.clone(),
        ..actor(101, 50)
    };
    let owner = AuthorizationContext {
        scope: fixture.writer.scope.clone(),
        ..actor(99, 50)
    };
    AuthorizationService::new(fixture.store.clone())
        .grant_relationship(
            &mutation(owner, 4500),
            &GrantScopeRelationship {
                expected_policy_version: 3,
                subject: RelationshipSubject {
                    principal_id: manager_b.identity.principal_id,
                    principal_kind: PrincipalKind::User,
                },
                relation: RelationId::parse(MANAGER_RELATION).unwrap(),
            },
        )
        .unwrap();
    let db = rusqlite::Connection::open(fixture.store.path()).unwrap();
    db.execute(
        "DELETE FROM scope_relationships WHERE principal_id=?1 AND relation=?2",
        [
            fixture.manager.identity.principal_id.value().to_string(),
            MANAGER_RELATION.to_string(),
        ],
    )
    .unwrap();
    manager_b
}
