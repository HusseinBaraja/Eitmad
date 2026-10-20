use super::*;
use eitmad_contracts::{customer::*, identity::UserId, sync::ReconcileResolution};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct RecoveryDirectory {
    page: CustomerConflictPage,
    original: ReconcileResolution<ConfirmCustomerResolution>,
    receipt: Option<ChangeRecord>,
    unavailable: Mutex<bool>,
    recovered: Mutex<usize>,
}
impl CustomerDirectory for RecoveryDirectory {
    fn conflicts(
        &self,
        _: &AuthorizationContext,
        _: &GetCustomerConflicts,
    ) -> Result<CustomerConflictPage, CustomerError> {
        Ok(self.page.clone())
    }
    fn reconcile_resolution(
        &self,
        actor: &AuthorizationContext,
        input: &ReconcileResolution<ConfirmCustomerResolution>,
    ) -> Result<Option<ChangeRecord>, CustomerError> {
        assert_ne!(
            actor.identity.principal_id.value(),
            input.original_user.value()
        );
        assert_eq!(input, &self.original);
        if *self.unavailable.lock().unwrap() {
            return Err(CustomerError::Unavailable);
        }
        *self.recovered.lock().unwrap() += 1;
        Ok(self.receipt.clone())
    }
    fn resolve(
        &self,
        _: &AuthorizationContext,
        _: &ConfirmCustomerResolution,
    ) -> Result<ChangeRecord, CustomerError> {
        if *self.unavailable.lock().unwrap() {
            return Err(CustomerError::Unavailable);
        }
        self.receipt.clone().ok_or(CustomerError::Unavailable)
    }
    fn read(
        &self,
        _: &AuthorizationContext,
        _: &CustomerRead,
    ) -> Result<CustomerPage, CustomerError> {
        unreachable!()
    }
    fn watch(
        &self,
        _: &AuthorizationContext,
        _: &std::sync::atomic::AtomicBool,
        _: &mut dyn FnMut(CustomerChangeNotice),
    ) -> Result<(), CustomerError> {
        unreachable!()
    }
}

#[test]
fn resolution_rejects_unrelated_receipts_and_keeps_exact_retry_intent() {
    let fixture = RecoveryFixture::new(true);
    *fixture.server.unavailable.lock().unwrap() = false;
    let service = CustomerService::new(
        fixture.store.clone(),
        AuthorizationService::new(fixture.store.clone()),
    );
    let request = &fixture.server.original.request;
    let actor = &fixture.manager_b;
    service
        .clone()
        .with_directory(fixture.server.clone())
        .conflicts(
            actor,
            &GetCustomerConflicts {
                customer: request.command.customer.clone(),
                after: None,
            },
        )
        .unwrap();
    for field in ["scope", "key", "revision"] {
        let mut receipt = fixture.server.receipt.clone().unwrap();
        match field {
            "scope" => receipt.scope.kind = ScopeKind::parse("organization").unwrap(),
            "key" => receipt.idempotency_key = IdempotencyKey::new(Uuid::new_v4()),
            _ => receipt.revision += 1,
        }
        let directory = Arc::new(RecoveryDirectory {
            receipt: Some(receipt),
            page: fixture.server.page.clone(),
            original: fixture.server.original.clone(),
            unavailable: Mutex::new(false),
            recovered: Mutex::new(0),
        });
        let context = MutationContext {
            idempotency_key: request.idempotency_key,
            ..mutation(actor.clone(), 914, 5)
        };
        assert_eq!(
            service
                .clone()
                .with_directory(directory)
                .resolve(&context, &request.command),
            Err(CustomerError::Unavailable),
            "{field}"
        );
        assert_eq!(
            fixture
                .store
                .customer_resolution_intent(
                    actor,
                    request.command.customer.customer_id,
                    actor.scope.id
                )
                .unwrap(),
            Some(request.clone()),
            "{field}"
        );
    }
}

struct RecoveryFixture {
    directory: TempDir,
    store: AuthorityStore,
    manager_a: AuthorizationContext,
    manager_b: AuthorizationContext,
    server: Arc<RecoveryDirectory>,
}
impl RecoveryFixture {
    fn new(applied: bool) -> Self {
        let dir = TempDir::new().unwrap();
        let (store, service, manager_a) = authorized_service(&dir, 9, 90, MANAGER_RELATION);
        let customer = service
            .create(
                &mutation(manager_a.clone(), 910, 1),
                &create_command("عميل تجريبي", "777123456"),
            )
            .unwrap()
            .customer;
        let initial = service.sync_batch(&manager_a.scope, 10).unwrap().remove(0);
        let receipt = resolved_customer_change(&initial, &initial, &customer);
        let request = ConfirmCustomerResolution {
            scope: manager_a.scope.clone(),
            idempotency_key: receipt.idempotency_key,
            command: ResolveCustomerConflict {
                customer: GetCustomer {
                    customer_id: customer.id,
                    branch_id: Some(manager_a.scope.id),
                },
                source: CustomerResolutionSource::Conflict(CustomerConflictSource {
                    conflict_id: eitmad_contracts::sync::ConflictId::new(Uuid::new_v4()),
                }),
                expected_server_revision: receipt.base_revision.unwrap(),
                contact: CustomerContact {
                    name: customer.name.clone(),
                    phone: customer.phone.clone(),
                    address: None,
                    notes: None,
                },
            },
        };
        let server = Arc::new(RecoveryDirectory {
            page: CustomerConflictPage {
                current: customer.clone(),
                items: vec![],
                next: None,
                pending_reviews: vec![],
                pending_resolution: None,
            },
            original: ReconcileResolution {
                original_user: UserId::new(manager_a.identity.principal_id.value()),
                request: request.clone(),
            },
            receipt: applied.then_some(receipt.clone()),
            unavailable: Mutex::new(true),
            recovered: Mutex::new(0),
        });
        let context = MutationContext {
            idempotency_key: request.idempotency_key,
            ..mutation(manager_a.clone(), 911, 2)
        };
        assert_eq!(
            service
                .clone()
                .with_directory(server.clone())
                .resolve(&context, &request.command),
            Err(CustomerError::Unavailable)
        );
        let manager_b = authorization(10, 90);
        let auth = AuthorizationService::new(store.clone());
        auth.grant_relationship(
            &mutation(manager_a.clone(), 912, 3),
            &GrantScopeRelationship {
                expected_policy_version: 2,
                subject: RelationshipSubject {
                    principal_id: manager_b.identity.principal_id,
                    principal_kind: PrincipalKind::User,
                },
                relation: RelationId::parse(MANAGER_RELATION).unwrap(),
            },
        )
        .unwrap();
        let db = Connection::open(store.path()).unwrap();
        // Simulate imported server role removal in the authority database.
        db.execute(
            "DELETE FROM scope_relationships WHERE principal_id=?1 AND relation=?2",
            [
                manager_a.identity.principal_id.value().to_string(),
                MANAGER_RELATION.to_string(),
            ],
        )
        .unwrap();
        Self {
            directory: dir,
            store,
            manager_a,
            manager_b,
            server,
        }
    }
    fn assert_archived(&self) {
        let db = Connection::open(self.store.path()).unwrap();
        let manager_a = &self.manager_a;
        let manager_b = &self.manager_b;
        let request = &self.server.original.request;
        let server = &self.server;
        let (original, archived, saved, recovered_by): (String,Vec<u8>,Option<Vec<u8>>,String) = db.query_row("SELECT principal_id,request_json,receipt_json,reconciled_by FROM resolution_reconciliations", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        assert_eq!(
            original,
            manager_a.identity.principal_id.value().to_string()
        );
        assert_eq!(
            serde_json::from_slice::<ConfirmCustomerResolution>(&archived).unwrap(),
            *request
        );
        assert_eq!(
            saved.map(|v| serde_json::from_slice::<ChangeRecord>(&v).unwrap()),
            server.receipt
        );
        assert_eq!(
            recovered_by,
            manager_b.identity.principal_id.value().to_string()
        );
    }
}

#[test]
fn resolution_handover_after_restart_preserves_original_evidence_and_denies_removed_manager() {
    for applied in [false, true] {
        let fixture = RecoveryFixture::new(applied);
        let manager_a = &fixture.manager_a;
        let manager_b = &fixture.manager_b;
        let server = &fixture.server;
        let request = &server.original.request;
        let customer = &server.page.current;
        let reopened = AuthorityStore::open(fixture.directory.path()).unwrap();
        let service = CustomerService::new(
            reopened.clone(),
            AuthorizationService::new(reopened.clone()),
        )
        .with_directory(server.clone());
        let query = GetCustomerConflicts {
            customer: request.command.customer.clone(),
            after: None,
        };
        assert_eq!(
            service.conflicts(manager_a, &query),
            Err(CustomerError::Denied)
        );
        assert_eq!(
            service.conflicts(manager_b, &query),
            Err(CustomerError::Unavailable)
        );
        assert!(
            reopened
                .pending_customer_resolution(manager_b, customer.id, manager_b.scope.id)
                .unwrap()
                .is_some()
        );
        *server.unavailable.lock().unwrap() = false;
        assert!(
            service
                .conflicts(manager_b, &query)
                .unwrap()
                .pending_resolution
                .is_none()
        );
        assert!(
            reopened
                .pending_customer_resolution(manager_b, customer.id, manager_b.scope.id)
                .unwrap()
                .is_none()
        );
        service.conflicts(manager_b, &query).unwrap();
        assert_eq!(*server.recovered.lock().unwrap(), 1);
        fixture.assert_archived();
        let new_request = ConfirmCustomerResolution {
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            ..request.clone()
        };
        reopened
            .stage_customer_resolution(
                manager_b,
                &new_request,
                &audit_record(
                    &mutation(manager_b.clone(), 913, 4),
                    "eitmad.customer.resolution-request.v1",
                    customer.id,
                ),
            )
            .unwrap();
    }
}
