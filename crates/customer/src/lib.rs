//! Branch-scoped customer contact capability.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_authorization::{AuthorizationError, AuthorizationService, MutationContext};
pub use eitmad_authorization::{CUSTOMER_READ_PERMISSION, CUSTOMER_WRITE_PERMISSION};
use eitmad_contracts::{
    commands::{CreateCustomer, UpdateCustomer},
    customer::{
        Customer, CustomerChangeNotice, CustomerId, CustomerMutationResult, CustomerPage,
        CustomerStatus, CustomerSyncState, GetCustomer, SearchCustomers,
    },
    events::Event,
    identity::{AuthorizationContext, ScopeRef},
    sync::{ChangeId, ChangeOperation, ChangeRecord, EncodedDomainPayload, RecordId},
    transport::SchemaId,
};
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use eitmad_storage::{
    AuthorityStore, CustomerCommit, CustomerCommitOutcome, DurableIdempotency, DurablePublication,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use unicode_normalization::{UnicodeNormalization as _, char::is_combining_mark};
use uuid::Uuid;

pub const CUSTOMER_SCHEMA_ID: &str = "eitmad.schema.customer.v1";

const BRANCH_SCOPE: &str = "branch";
const CREATE_OPERATION: &str = "eitmad.customer.create.v1";
const UPDATE_OPERATION: &str = "eitmad.customer.update.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomerError {
    Denied,
    UnsupportedScope,
    NotFound,
    RevisionConflict {
        expected_revision: Option<u64>,
        actual_revision: Option<u64>,
    },
    IdempotencyMismatch,
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct CustomerService {
    store: AuthorityStore,
    authorization: AuthorizationService,
}

impl CustomerService {
    #[must_use]
    pub const fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self {
            store,
            authorization,
        }
    }

    /// Creates one stable customer identity in the authorized branch scope.
    ///
    /// # Errors
    ///
    /// Returns a domain error for denial, scope mismatch, idempotency mismatch,
    /// or unavailable durable state.
    pub fn create(
        &self,
        context: &MutationContext,
        command: &CreateCustomer,
    ) -> Result<CustomerMutationResult, CustomerError> {
        self.authorize_mutation(context, CREATE_OPERATION, None)?;
        let customer = Customer {
            id: CustomerId::new(Uuid::new_v4()),
            scope: context.authorization.scope.clone(),
            name: command.name.clone(),
            phone: command.phone.clone(),
            address: command.address.clone(),
            notes: command.notes.clone(),
            status: CustomerStatus::Active,
            revision: 1,
            updated_at: context.occurred_at,
            sync_state: CustomerSyncState::Pending,
        };
        self.commit(
            context,
            CREATE_OPERATION,
            None,
            &customer,
            request_hash(CREATE_OPERATION, command)?,
        )
    }

    /// Updates customer contact fields with an optimistic revision check.
    ///
    /// # Errors
    ///
    /// Returns a domain error for denial, scope mismatch, missing state,
    /// revision conflict, idempotency mismatch, or unavailable durable state.
    pub fn update(
        &self,
        context: &MutationContext,
        command: &UpdateCustomer,
    ) -> Result<CustomerMutationResult, CustomerError> {
        self.authorize_mutation(context, UPDATE_OPERATION, Some(command.customer_id))?;
        let current = self
            .store
            .get_customer(&context.authorization.scope, command.customer_id)
            .map_err(|_| CustomerError::Unavailable)?;
        let Some(current) = current else {
            self.store
                .append_audit(
                    &audit_record(context, UPDATE_OPERATION, command.customer_id).with_outcome(
                        AuditOutcome::Failed,
                        Some("eitmad.error.customer-not-found.v1".to_owned()),
                    ),
                )
                .map_err(|_| CustomerError::Unavailable)?;
            return Err(CustomerError::NotFound);
        };
        let revision = command
            .expected_revision
            .checked_add(1)
            .ok_or(CustomerError::Unavailable)?;
        let customer = Customer {
            id: current.id,
            scope: current.scope,
            name: command.name.clone(),
            phone: command.phone.clone(),
            address: command.address.clone(),
            notes: command.notes.clone(),
            status: current.status,
            revision,
            updated_at: context.occurred_at,
            sync_state: CustomerSyncState::Pending,
        };
        self.commit(
            context,
            UPDATE_OPERATION,
            Some(command.expected_revision),
            &customer,
            request_hash(UPDATE_OPERATION, command)?,
        )
    }

    /// Gets one customer from the exact authorized branch scope.
    ///
    /// # Errors
    ///
    /// Returns a domain error for denial, missing state, or unavailable storage.
    pub fn get(
        &self,
        context: &AuthorizationContext,
        query: &GetCustomer,
    ) -> Result<Customer, CustomerError> {
        self.authorize_read(context)?;
        self.store
            .get_customer(&context.scope, query.customer_id)
            .map_err(|_| CustomerError::Unavailable)?
            .ok_or(CustomerError::NotFound)
    }

    /// Searches customer names and phones in one exact authorized branch scope.
    ///
    /// # Errors
    ///
    /// Returns a domain error for denial, scope mismatch, or unavailable storage.
    pub fn search(
        &self,
        context: &AuthorizationContext,
        query: &SearchCustomers,
    ) -> Result<CustomerPage, CustomerError> {
        self.authorize_read(context)?;
        let normalized_name = normalize_arabic_name(query.term.as_str());
        let normalized_phone = normalize_phone_search(query.term.as_str());
        self.store
            .search_customers(
                &context.scope,
                &normalized_name,
                normalized_phone.as_deref(),
                query.after,
                query.limit(),
            )
            .map_err(|_| CustomerError::Unavailable)
    }

    /// Loads one bounded internal customer sync batch.
    ///
    /// # Errors
    ///
    /// Returns an unavailable error for invalid limits or malformed durable work.
    pub fn sync_batch(
        &self,
        scope: &ScopeRef,
        limit: u32,
    ) -> Result<Vec<ChangeRecord>, CustomerError> {
        validate_scope(scope)?;
        self.store
            .customer_sync_batch(scope, limit)
            .map_err(|_| CustomerError::Unavailable)
    }

    /// Confirms one exact delivered customer change.
    ///
    /// # Errors
    ///
    /// Returns an unavailable error when the scoped change is absent.
    pub fn confirm_sync(&self, scope: &ScopeRef, change_id: ChangeId) -> Result<(), CustomerError> {
        validate_scope(scope)?;
        self.store
            .confirm_customer_sync(scope, change_id)
            .map_err(|_| CustomerError::Unavailable)
    }

    fn commit(
        &self,
        context: &MutationContext,
        operation: &'static str,
        expected_revision: Option<u64>,
        customer: &Customer,
        request_hash: [u8; 32],
    ) -> Result<CustomerMutationResult, CustomerError> {
        let normalized_name = normalize_arabic_name(customer.name.as_str());
        let normalized_phone = normalize_phone(customer.phone.as_str());
        let change_id = ChangeId::new(Uuid::new_v4());
        let change = sync_change(context, customer, expected_revision, change_id)?;
        let idempotency = DurableIdempotency {
            key: context.idempotency_key,
            request_hash,
            response_json: Vec::new(),
        };
        let publication = DurablePublication {
            event: Event::CustomerChanged(CustomerChangeNotice {
                customer_id: customer.id,
                scope: customer.scope.clone(),
                revision: customer.revision,
                changed_at: customer.updated_at,
                change_id,
            }),
            policy_changed: false,
        };
        let audit = audit_record(context, operation, customer.id);
        match self.store.commit_customer(&CustomerCommit {
            customer,
            normalized_name: &normalized_name,
            normalized_phone: &normalized_phone,
            expected_revision,
            operation,
            idempotency: &idempotency,
            audit: &audit,
            publication: &publication,
            change: &change,
            conflict_error_code: "eitmad.error.customer-revision-conflict.v1",
        }) {
            Ok(CustomerCommitOutcome::Committed(result)) => Ok(result),
            Ok(CustomerCommitOutcome::Replayed { response_json }) => {
                serde_json::from_slice(&response_json).map_err(|_| CustomerError::Unavailable)
            }
            Ok(CustomerCommitOutcome::RevisionConflict { actual_revision }) => {
                Err(CustomerError::RevisionConflict {
                    expected_revision,
                    actual_revision,
                })
            }
            Ok(CustomerCommitOutcome::IdempotencyMismatch) => {
                Err(CustomerError::IdempotencyMismatch)
            }
            Err(_) => Err(CustomerError::Unavailable),
        }
    }

    fn authorize_mutation(
        &self,
        context: &MutationContext,
        operation: &str,
        customer_id: Option<CustomerId>,
    ) -> Result<(), CustomerError> {
        validate_scope(&context.authorization.scope)?;
        match self
            .authorization
            .authorize(&context.authorization, CUSTOMER_WRITE_PERMISSION)
        {
            Ok(()) => Ok(()),
            Err(AuthorizationError::Denied) => {
                let id = customer_id.unwrap_or_else(|| CustomerId::new(Uuid::new_v4()));
                self.store
                    .append_audit(&audit_record(context, operation, id).with_outcome(
                        AuditOutcome::Denied,
                        Some("eitmad.error.authorization-denied.v1".to_owned()),
                    ))
                    .map_err(|_| CustomerError::Unavailable)?;
                Err(CustomerError::Denied)
            }
            Err(AuthorizationError::UnsupportedScope) => Err(CustomerError::UnsupportedScope),
            Err(_) => Err(CustomerError::Unavailable),
        }
    }

    fn authorize_read(&self, context: &AuthorizationContext) -> Result<(), CustomerError> {
        validate_scope(&context.scope)?;
        self.authorization
            .authorize(context, CUSTOMER_READ_PERMISSION)
            .map_err(map_authorization_error)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomerSyncPayload<'a> {
    customer_id: CustomerId,
    name: &'a eitmad_contracts::customer::CustomerName,
    phone: &'a eitmad_contracts::customer::CustomerPhone,
    address: &'a Option<eitmad_contracts::customer::CustomerAddress>,
    notes: &'a Option<eitmad_contracts::customer::CustomerNotes>,
    revision: u64,
}

fn sync_change(
    context: &MutationContext,
    customer: &Customer,
    base_revision: Option<u64>,
    change_id: ChangeId,
) -> Result<ChangeRecord, CustomerError> {
    let payload = serde_json::to_vec(&CustomerSyncPayload {
        customer_id: customer.id,
        name: &customer.name,
        phone: &customer.phone,
        address: &customer.address,
        notes: &customer.notes,
        revision: customer.revision,
    })
    .map_err(|_| CustomerError::Unavailable)?;
    Ok(ChangeRecord {
        change_id,
        record_id: RecordId::new(customer.id.value()),
        scope: customer.scope.clone(),
        operation: ChangeOperation::Upsert,
        base_revision,
        revision: customer.revision,
        changed_at: customer.updated_at,
        idempotency_key: context.idempotency_key,
        payload: Some(EncodedDomainPayload {
            schema_id: SchemaId::parse(CUSTOMER_SCHEMA_ID)
                .expect("static customer schema ID is valid"),
            schema_version: 1,
            base64: STANDARD.encode(payload),
        }),
        merge: None,
    })
}

fn request_hash(operation: &str, command: &impl Serialize) -> Result<[u8; 32], CustomerError> {
    let encoded =
        serde_json::to_vec(&(operation, command)).map_err(|_| CustomerError::Unavailable)?;
    Ok(Sha256::digest(encoded).into())
}

fn audit_record(
    context: &MutationContext,
    operation: &str,
    customer_id: CustomerId,
) -> MutationAuditRecord {
    let mut record = MutationAuditRecord::from_authorization(
        &context.authorization,
        context.occurred_at,
        context.correlation_id,
        operation,
        AuditTarget {
            kind: "customer".to_owned(),
            identifiers: vec![customer_id.value().to_string()],
        },
    );
    record.causation_id = context.causation_id;
    record.idempotency_key = Some(context.idempotency_key);
    record.changed_identifiers = vec![
        "name".to_owned(),
        "phone".to_owned(),
        "address".to_owned(),
        "notes".to_owned(),
    ];
    record
}

fn validate_scope(scope: &ScopeRef) -> Result<(), CustomerError> {
    (scope.kind.as_str() == BRANCH_SCOPE)
        .then_some(())
        .ok_or(CustomerError::UnsupportedScope)
}

fn map_authorization_error(error: AuthorizationError) -> CustomerError {
    match error {
        AuthorizationError::Denied => CustomerError::Denied,
        AuthorizationError::UnsupportedScope => CustomerError::UnsupportedScope,
        _ => CustomerError::Unavailable,
    }
}

/// Normalizes customer names for search without changing stored text.
#[must_use]
pub fn normalize_arabic_name(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut pending_space = false;
    for character in value.nfd() {
        if is_combining_mark(character) || character == '\u{0640}' {
            continue;
        }
        if character.is_whitespace() {
            pending_space = !normalized.is_empty();
            continue;
        }
        if matches!(character, '\u{200c}' | '\u{200d}') {
            continue;
        }
        if pending_space {
            normalized.push(' ');
            pending_space = false;
        }
        let mapped = match character {
            '\u{0622}' | '\u{0623}' | '\u{0625}' | '\u{0671}' => '\u{0627}',
            '\u{0649}' | '\u{06cc}' => '\u{064a}',
            '\u{0629}' => '\u{0647}',
            '\u{06a9}' => '\u{0643}',
            '\u{0660}'..='\u{0669}' => {
                char::from_u32(u32::from('0') + u32::from(character) - 0x0660)
                    .expect("Arabic digit maps to ASCII")
            }
            '\u{06f0}'..='\u{06f9}' => {
                char::from_u32(u32::from('0') + u32::from(character) - 0x06f0)
                    .expect("Persian digit maps to ASCII")
            }
            _ => character,
        };
        for lowercase in mapped.to_lowercase() {
            normalized.push(lowercase);
        }
    }
    normalized
}

/// Normalizes a validated phone for search without changing its display value.
#[must_use]
pub fn normalize_phone(value: &str) -> String {
    normalize_phone_search(value).expect("validated phone uses the accepted search alphabet")
}

fn normalize_phone_search(value: &str) -> Option<String> {
    let mut normalized = String::with_capacity(value.len());
    for (index, character) in value.chars().enumerate() {
        match character {
            '+' if index == 0 => normalized.push('+'),
            '0'..='9' => normalized.push(character),
            '\u{0660}'..='\u{0669}' => normalized.push(
                char::from_u32(u32::from('0') + u32::from(character) - 0x0660)
                    .expect("Arabic digit maps to ASCII"),
            ),
            '\u{06f0}'..='\u{06f9}' => normalized.push(
                char::from_u32(u32::from('0') + u32::from(character) - 0x06f0)
                    .expect("Persian digit maps to ASCII"),
            ),
            ' ' | '-' | '(' | ')' => {}
            _ => return None,
        }
    }
    (!normalized.is_empty() && normalized != "+").then_some(normalized)
}

#[cfg(test)]
mod tests {
    use eitmad_authorization::{MANAGER_RELATION, RECEPTIONIST_RELATION};
    use eitmad_contracts::{
        authorization::{RelationId, RelationshipSubject},
        commands::GrantScopeRelationship,
        customer::{
            CustomerAddress, CustomerName, CustomerNotes, CustomerPhone, CustomerSearchTerm,
        },
        identity::{
            AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId,
            TenantId,
        },
        transport::{CorrelationId, IdempotencyKey, UnixMillis},
    };
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::*;

    fn authorization(principal: u128, branch: u128) -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::from_u128(principal + 100)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::from_u128(principal)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(Uuid::from_u128(1)),
            workspace_id: None,
            scope: ScopeRef {
                kind: ScopeKind::parse("branch").unwrap(),
                id: ScopeId::new(Uuid::from_u128(branch)),
            },
        }
    }

    fn mutation(authorization: AuthorizationContext, key: u128, time: i64) -> MutationContext {
        MutationContext {
            authorization,
            correlation_id: CorrelationId::new(Uuid::from_u128(key + 1_000)),
            causation_id: None,
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(key)),
            occurred_at: UnixMillis(time),
        }
    }

    fn authorized_service(
        directory: &TempDir,
        principal: u128,
        branch: u128,
        relation: &str,
    ) -> (AuthorityStore, CustomerService, AuthorizationContext) {
        let store = AuthorityStore::open(directory.path()).unwrap();
        let authorization = authorization(principal, branch);
        let authorization_service = AuthorizationService::new(store.clone());
        authorization_service
            .bootstrap_owner(
                &mutation(authorization.clone(), branch + 10_000, 1),
                &RelationshipSubject {
                    principal_id: authorization.identity.principal_id,
                    principal_kind: PrincipalKind::User,
                },
            )
            .unwrap();
        authorization_service
            .grant_relationship(
                &mutation(authorization.clone(), branch + 20_000, 2),
                &GrantScopeRelationship {
                    expected_policy_version: 1,
                    subject: RelationshipSubject {
                        principal_id: authorization.identity.principal_id,
                        principal_kind: PrincipalKind::User,
                    },
                    relation: RelationId::parse(relation).unwrap(),
                },
            )
            .unwrap();
        (
            store.clone(),
            CustomerService::new(store, authorization_service),
            authorization,
        )
    }

    fn create_command(name: &str, phone: &str) -> CreateCustomer {
        CreateCustomer {
            name: CustomerName::parse(name).unwrap(),
            phone: CustomerPhone::parse(phone).unwrap(),
            address: Some(CustomerAddress::parse("عدن، المنصورة").unwrap()),
            notes: Some(CustomerNotes::parse("الاتصال قبل التسليم").unwrap()),
        }
    }

    #[test]
    fn persists_after_restart_and_preserves_entered_text() {
        let directory = TempDir::new().unwrap();
        let (_store, service, authorization) =
            authorized_service(&directory, 2, 10, RECEPTIONIST_RELATION);
        let created = service
            .create(
                &mutation(authorization.clone(), 100, 10),
                &create_command("إعـتماد القيسي", "+٩٦٧ (٧٧٧) ١٢٣-٤٥٦"),
            )
            .unwrap();
        drop(service);

        let reopened = AuthorityStore::open(directory.path()).unwrap();
        let service = CustomerService::new(reopened.clone(), AuthorizationService::new(reopened));
        let customer = service
            .get(
                &authorization,
                &GetCustomer {
                    customer_id: created.customer.id,
                },
            )
            .unwrap();
        assert_eq!(customer.name.as_str(), "إعـتماد القيسي");
        assert_eq!(customer.phone.as_str(), "+٩٦٧ (٧٧٧) ١٢٣-٤٥٦");
    }

    #[test]
    fn search_normalizes_arabic_names_and_phones_and_pages_stably() {
        let directory = TempDir::new().unwrap();
        let (_store, service, authorization) =
            authorized_service(&directory, 3, 20, RECEPTIONIST_RELATION);
        for (key, name, phone) in [
            (200, "إعـتماد القيسي", "+٩٦٧ ٧٧٧ ١٢٣ ٤٥٦"),
            (201, "اعتماد أحمد", "+967-733-000-001"),
            (202, "سالم علي", "+967 711 000 002"),
        ] {
            service
                .create(
                    &mutation(authorization.clone(), key, key as i64),
                    &create_command(name, phone),
                )
                .unwrap();
        }

        let first = service
            .search(
                &authorization,
                &SearchCustomers::new(CustomerSearchTerm::parse("اِعتماد").unwrap(), None, 1)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(first.items.len(), 1);
        assert!(first.next.is_some());
        let second = service
            .search(
                &authorization,
                &SearchCustomers::new(CustomerSearchTerm::parse("اِعتماد").unwrap(), first.next, 1)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(second.items.len(), 1);
        assert!(second.next.is_none());

        let phone = service
            .search(
                &authorization,
                &SearchCustomers::new(
                    CustomerSearchTerm::parse("+967 (777) 123-456").unwrap(),
                    None,
                    10,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(phone.items.len(), 1);
        assert_eq!(phone.items[0].name.as_str(), "إعـتماد القيسي");
    }

    #[test]
    fn denies_unrelated_principal_and_isolates_branch_scope() {
        let directory = TempDir::new().unwrap();
        let (_store, service, branch_a) =
            authorized_service(&directory, 4, 30, RECEPTIONIST_RELATION);
        service
            .create(
                &mutation(branch_a.clone(), 300, 3),
                &create_command("عميل الفرع أ", "777 000 001"),
            )
            .unwrap();

        let denied = authorization(99, 30);
        assert_eq!(
            service.search(
                &denied,
                &SearchCustomers::new(CustomerSearchTerm::parse("").unwrap(), None, 10).unwrap()
            ),
            Err(CustomerError::Denied)
        );
        assert_eq!(
            service.create(
                &mutation(denied, 301, 4),
                &create_command("بيانات سرية", "777 000 009"),
            ),
            Err(CustomerError::Denied)
        );
        assert_eq!(
            service
                .search(
                    &branch_a,
                    &SearchCustomers::new(CustomerSearchTerm::parse("").unwrap(), None, 10)
                        .unwrap(),
                )
                .unwrap()
                .items
                .len(),
            1
        );

        let (_other_store, other_service, branch_b) =
            authorized_service(&directory, 4, 31, RECEPTIONIST_RELATION);
        assert!(
            other_service
                .search(
                    &branch_b,
                    &SearchCustomers::new(CustomerSearchTerm::parse("").unwrap(), None, 10)
                        .unwrap()
                )
                .unwrap()
                .items
                .is_empty()
        );
    }

    #[test]
    fn exact_retry_returns_one_stable_identity_and_duplicate_phone_is_advisory() {
        let directory = TempDir::new().unwrap();
        let (store, service, authorization) =
            authorized_service(&directory, 5, 40, RECEPTIONIST_RELATION);
        let context = mutation(authorization.clone(), 400, 4);
        let command = create_command("العميل الأول", "٧٧٧ ٠٠٠ ٠٠١");
        let first = service.create(&context, &command).unwrap();
        let replay = service.create(&context, &command).unwrap();
        assert_eq!(replay, first);
        let publication = store
            .pending_publication(&authorization.scope, context.idempotency_key)
            .unwrap()
            .unwrap();
        assert!(matches!(publication.event, Event::CustomerChanged(_)));
        let sync = service.sync_batch(&authorization.scope, 10).unwrap();
        assert_eq!(sync.len(), 1);
        service
            .confirm_sync(&authorization.scope, sync[0].change_id)
            .unwrap();
        assert!(
            service
                .sync_batch(&authorization.scope, 10)
                .unwrap()
                .is_empty()
        );

        let second = service
            .create(
                &mutation(authorization.clone(), 401, 5),
                &create_command("العميل الثاني", "777-000-001"),
            )
            .unwrap();
        assert_ne!(first.customer.id, second.customer.id);
        assert_eq!(second.potential_duplicate_ids, vec![first.customer.id]);
        assert_eq!(
            service
                .search(
                    &authorization,
                    &SearchCustomers::new(CustomerSearchTerm::parse("").unwrap(), None, 10)
                        .unwrap()
                )
                .unwrap()
                .items
                .len(),
            2
        );
    }

    #[test]
    fn audit_failure_rolls_back_customer_and_concurrent_edit_conflicts() {
        let directory = TempDir::new().unwrap();
        let (store, service, authorization) =
            authorized_service(&directory, 6, 50, MANAGER_RELATION);
        Connection::open(store.path())
            .unwrap()
            .execute_batch("DROP TABLE mutation_audit")
            .unwrap();
        let failed = service.create(
            &mutation(authorization.clone(), 500, 5),
            &create_command("لن يحفظ", "777000001"),
        );
        assert_eq!(failed, Err(CustomerError::Unavailable));
        assert!(
            store
                .search_customers(&authorization.scope, "", None, None, 10)
                .unwrap()
                .items
                .is_empty()
        );

        let directory = TempDir::new().unwrap();
        let (_store, service, authorization) =
            authorized_service(&directory, 7, 60, MANAGER_RELATION);
        let created = service
            .create(
                &mutation(authorization.clone(), 600, 6),
                &create_command("عميل", "777000001"),
            )
            .unwrap();
        let update = UpdateCustomer {
            customer_id: created.customer.id,
            expected_revision: 1,
            name: CustomerName::parse("عميل محدث").unwrap(),
            phone: CustomerPhone::parse("777000002").unwrap(),
            address: None,
            notes: None,
        };
        service
            .update(&mutation(authorization.clone(), 601, 7), &update)
            .unwrap();
        assert_eq!(
            service.update(&mutation(authorization, 602, 8), &update),
            Err(CustomerError::RevisionConflict {
                expected_revision: Some(1),
                actual_revision: Some(2),
            })
        );
    }

    #[test]
    fn normalization_profiles_are_explicit_and_non_destructive() {
        assert_eq!(
            normalize_arabic_name(" إِعـتماد  کاظم یحیى "),
            "اعتماد كاظم يحيي"
        );
        assert_eq!(normalize_phone("+۹۶۷ (٧٧٧) 123-٤٥٦"), "+967777123456");
    }
}
