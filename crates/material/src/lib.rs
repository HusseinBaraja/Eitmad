//! Durable raw material definitions and versioned reference administration.

use eitmad_authorization::{
    AuthorizationError, AuthorizationService, MATERIAL_READ_PERMISSION,
    MATERIAL_UNIT_MANAGE_PERMISSION, MATERIAL_WRITE_PERMISSION, MutationContext,
};
use eitmad_contracts::{
    events::Event,
    identity::AuthorizationContext,
    material::{
        ListMaterials, Material, MaterialCategory, MaterialCategoryId, MaterialChangeNotice,
        MaterialId, MaterialPage, MaterialRecordKind, MaterialReferences, MaterialUnit,
        MaterialUnitId, SaveMaterial, SaveMaterialCategory, SaveMaterialUnit,
    },
};
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use eitmad_storage::{
    AuthorityStore, DurableIdempotency, DurablePublication, MaterialCommit, MaterialCommitOutcome,
    MaterialRecord,
};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest as _, Sha256};
use unicode_normalization::{UnicodeNormalization as _, char::is_combining_mark};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialError {
    Denied,
    Invalid,
    InvalidReference,
    NotFound,
    RevisionConflict {
        expected: Option<u64>,
        actual: Option<u64>,
    },
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct MaterialService {
    store: AuthorityStore,
    authorization: AuthorizationService,
}

impl MaterialService {
    #[must_use]
    pub const fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self {
            store,
            authorization,
        }
    }

    /// Saves one category without changing IDs held by materials.
    /// # Errors
    /// Rejects unauthorized, invalid, or stale writes.
    pub fn save_category(
        &self,
        context: &MutationContext,
        command: &SaveMaterialCategory,
    ) -> Result<MaterialCategory, MaterialError> {
        const OP: &str = "eitmad.material-category.save.v1";
        self.authorize(
            context,
            MATERIAL_WRITE_PERMISSION,
            OP,
            command.id.map(MaterialCategoryId::value),
        )?;
        validate_name(&command.name, 120)?;
        validate_identity(
            command.id.as_ref(),
            command.expected_revision,
            command.archived,
        )?;
        let id = command
            .id
            .unwrap_or_else(|| MaterialCategoryId::new(Uuid::new_v4()));
        let record = MaterialCategory {
            id,
            scope: context.authorization.scope.clone(),
            name: command.name.clone(),
            archived: command.archived,
            revision: next_revision(command.expected_revision)?,
            updated_at: context.occurred_at,
        };
        self.commit(
            context,
            OP,
            command.expected_revision,
            MaterialRecord::Category(&record),
            &normalize_search(&record.name),
            command,
        )
    }

    /// Saves a unit; a referenced unit's dimension and conversion cannot change.
    /// # Errors
    /// Rejects unauthorized, invalid, stale, or reference-breaking writes.
    pub fn save_unit(
        &self,
        context: &MutationContext,
        command: &SaveMaterialUnit,
    ) -> Result<MaterialUnit, MaterialError> {
        const OP: &str = "eitmad.material-unit.save.v1";
        self.authorize(
            context,
            MATERIAL_UNIT_MANAGE_PERMISSION,
            OP,
            command.id.map(MaterialUnitId::value),
        )?;
        validate_name(&command.name, 120)?;
        validate_name(&command.symbol, 24)?;
        validate_identity(
            command.id.as_ref(),
            command.expected_revision,
            command.archived,
        )?;
        if command.numerator == 0 || command.denominator == 0 {
            return Err(MaterialError::Invalid);
        }
        let id = command
            .id
            .unwrap_or_else(|| MaterialUnitId::new(Uuid::new_v4()));
        let record = MaterialUnit {
            id,
            scope: context.authorization.scope.clone(),
            name: command.name.clone(),
            symbol: command.symbol.clone(),
            dimension: command.dimension,
            numerator: command.numerator,
            denominator: command.denominator,
            archived: command.archived,
            revision: next_revision(command.expected_revision)?,
            updated_at: context.occurred_at,
        };
        self.commit(
            context,
            OP,
            command.expected_revision,
            MaterialRecord::Unit(&record),
            &normalize_search(&record.name),
            command,
        )
    }

    /// Saves a definition and its whole-YER cost; no stock movement is created.
    /// # Errors
    /// Rejects unauthorized, invalid, stale, or inactive new references.
    pub fn save_material(
        &self,
        context: &MutationContext,
        command: &SaveMaterial,
    ) -> Result<Material, MaterialError> {
        const OP: &str = "eitmad.material.save.v1";
        self.authorize(
            context,
            MATERIAL_WRITE_PERMISSION,
            OP,
            command.id.map(MaterialId::value),
        )?;
        validate_name(&command.name, 160)?;
        validate_identity(
            command.id.as_ref(),
            command.expected_revision,
            command.archived,
        )?;
        if command.current_cost_yer < 0 {
            return Err(MaterialError::Invalid);
        }
        let id = command
            .id
            .unwrap_or_else(|| MaterialId::new(Uuid::new_v4()));
        let record = Material {
            id,
            scope: context.authorization.scope.clone(),
            name: command.name.clone(),
            category_id: command.category_id,
            unit_id: command.unit_id,
            current_cost_yer: command.current_cost_yer,
            archived: command.archived,
            revision: next_revision(command.expected_revision)?,
            updated_at: context.occurred_at,
        };
        self.commit(
            context,
            OP,
            command.expected_revision,
            MaterialRecord::Material(&record),
            &normalize_search(&record.name),
            command,
        )
    }

    /// Reads all category and unit records in one exact organization scope.
    /// # Errors
    /// Rejects unauthorized reads and unavailable storage.
    pub fn references(
        &self,
        context: &AuthorizationContext,
    ) -> Result<MaterialReferences, MaterialError> {
        self.authorize_read(context)?;
        self.store
            .material_references(&context.scope)
            .map_err(|_| MaterialError::Unavailable)
    }

    /// Searches stored names using an Arabic-normalized index without changing display text.
    /// # Errors
    /// Rejects unauthorized or oversized queries and unavailable storage.
    pub fn list(
        &self,
        context: &AuthorizationContext,
        query: &ListMaterials,
    ) -> Result<MaterialPage, MaterialError> {
        self.authorize_read(context)?;
        if query.term.len() > 256 || query.term.chars().any(|c| c.is_control() || unsafe_bidi(c)) {
            return Err(MaterialError::Invalid);
        }
        self.store
            .list_materials(
                &context.scope,
                &normalize_search(&query.term),
                query.after,
                query.limit(),
            )
            .map_err(|_| MaterialError::Unavailable)
    }

    fn authorize_read(&self, context: &AuthorizationContext) -> Result<(), MaterialError> {
        if context.scope.kind.as_str() != "organization" {
            return Err(MaterialError::Denied);
        }
        self.authorization
            .authorize(context, MATERIAL_READ_PERMISSION)
            .map_err(map_auth)
    }

    fn authorize(
        &self,
        context: &MutationContext,
        permission: &str,
        operation: &str,
        id: Option<Uuid>,
    ) -> Result<(), MaterialError> {
        if context.authorization.scope.kind.as_str() != "organization" {
            return Err(MaterialError::Denied);
        }
        match self
            .authorization
            .authorize(&context.authorization, permission)
        {
            Ok(()) => Ok(()),
            Err(AuthorizationError::Denied) => {
                let audit = audit(context, operation, id).with_outcome(
                    AuditOutcome::Denied,
                    Some("eitmad.error.authorization-denied.v1".to_owned()),
                );
                self.store
                    .append_audit(&audit)
                    .map_err(|_| MaterialError::Unavailable)?;
                Err(MaterialError::Denied)
            }
            Err(value) => Err(map_auth(value)),
        }
    }

    fn commit<C: Serialize, R: DeserializeOwned>(
        &self,
        context: &MutationContext,
        operation: &str,
        expected: Option<u64>,
        record: MaterialRecord<'_>,
        normalized: &str,
        command: &C,
    ) -> Result<R, MaterialError> {
        let hash: [u8; 32] = Sha256::digest(
            serde_json::to_vec(&(operation, command)).map_err(|_| MaterialError::Unavailable)?,
        )
        .into();
        let idempotency = DurableIdempotency {
            key: context.idempotency_key,
            request_hash: hash,
            response_json: Vec::new(),
        };
        let kind = match record {
            MaterialRecord::Material(_) => MaterialRecordKind::Material,
            MaterialRecord::Category(_) => MaterialRecordKind::Category,
            MaterialRecord::Unit(_) => MaterialRecordKind::Unit,
        };
        let publication = DurablePublication {
            event: Event::MaterialChanged(MaterialChangeNotice {
                scope: context.authorization.scope.clone(),
                kind,
                id: record_id(record),
                revision: record_revision(record),
                changed_at: context.occurred_at,
            }),
            policy_changed: false,
        };
        let audit = audit(context, operation, Some(record_id(record)));
        match self.store.commit_material(&MaterialCommit {
            record,
            normalized_name: normalized,
            expected_revision: expected,
            operation,
            idempotency: &idempotency,
            audit: &audit,
            publication: &publication,
        }) {
            Ok(MaterialCommitOutcome::Committed) => {
                serde_json::from_slice(&record_json(record)).map_err(|_| MaterialError::Unavailable)
            }
            Ok(MaterialCommitOutcome::Replayed(json)) => {
                serde_json::from_slice(&json).map_err(|_| MaterialError::Unavailable)
            }
            Ok(MaterialCommitOutcome::RevisionConflict(actual)) => {
                Err(MaterialError::RevisionConflict { expected, actual })
            }
            Ok(
                MaterialCommitOutcome::InvalidReference
                | MaterialCommitOutcome::ReferencedUnitDefinition,
            ) => Err(MaterialError::InvalidReference),
            Ok(
                MaterialCommitOutcome::DuplicateName | MaterialCommitOutcome::IdempotencyMismatch,
            ) => Err(MaterialError::Invalid),
            Err(_) => Err(MaterialError::Unavailable),
        }
    }
}

fn record_id(record: MaterialRecord<'_>) -> Uuid {
    match record {
        MaterialRecord::Material(x) => x.id.value(),
        MaterialRecord::Category(x) => x.id.value(),
        MaterialRecord::Unit(x) => x.id.value(),
    }
}
fn record_revision(record: MaterialRecord<'_>) -> u64 {
    match record {
        MaterialRecord::Material(x) => x.revision,
        MaterialRecord::Category(x) => x.revision,
        MaterialRecord::Unit(x) => x.revision,
    }
}
fn record_json(record: MaterialRecord<'_>) -> Vec<u8> {
    match record {
        MaterialRecord::Material(x) => serde_json::to_vec(x),
        MaterialRecord::Category(x) => serde_json::to_vec(x),
        MaterialRecord::Unit(x) => serde_json::to_vec(x),
    }
    .unwrap_or_default()
}

fn audit(context: &MutationContext, operation: &str, id: Option<Uuid>) -> MutationAuditRecord {
    let mut record = MutationAuditRecord::from_authorization(
        &context.authorization,
        context.occurred_at,
        context.correlation_id,
        operation,
        AuditTarget {
            kind: "material-definition".to_owned(),
            identifiers: id.map(|value| vec![value.to_string()]).unwrap_or_default(),
        },
    );
    record.causation_id = context.causation_id;
    record.idempotency_key = Some(context.idempotency_key);
    record.changed_identifiers = vec!["definition".to_owned(), "archived".to_owned()];
    record
}

fn validate_identity<T>(
    id: Option<&T>,
    expected: Option<u64>,
    archived: bool,
) -> Result<(), MaterialError> {
    if id.is_some() != expected.is_some() || expected == Some(0) || id.is_none() && archived {
        Err(MaterialError::Invalid)
    } else {
        Ok(())
    }
}
fn next_revision(expected: Option<u64>) -> Result<u64, MaterialError> {
    expected
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(MaterialError::Invalid)
}
fn unsafe_bidi(c: char) -> bool {
    matches!(c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'|'\u{200e}'|'\u{200f}')
}
fn validate_name(value: &str, max: usize) -> Result<(), MaterialError> {
    if value.is_empty()
        || value.len() > max
        || value.trim() != value
        || value.chars().any(|c| c.is_control() || unsafe_bidi(c))
    {
        Err(MaterialError::Invalid)
    } else {
        Ok(())
    }
}
fn map_auth(value: AuthorizationError) -> MaterialError {
    match value {
        AuthorizationError::Denied | AuthorizationError::UnsupportedScope => MaterialError::Denied,
        _ => MaterialError::Unavailable,
    }
}

/// Returns a derived search form. Stored display text is never rewritten.
#[must_use]
pub fn normalize_search(value: &str) -> String {
    let mut result = String::new();
    let mut space = false;
    for c in value.nfd() {
        if is_combining_mark(c) || c == '\u{0640}' || matches!(c, '\u{200c}' | '\u{200d}') {
            continue;
        }
        if c.is_whitespace() {
            space = !result.is_empty();
            continue;
        }
        if space {
            result.push(' ');
            space = false;
        }
        result.push(match c {
            '\u{0622}' | '\u{0623}' | '\u{0625}' | '\u{0671}' => '\u{0627}',
            '\u{0649}' | '\u{06cc}' => '\u{064a}',
            '\u{0629}' => '\u{0647}',
            '\u{06a9}' => '\u{0643}',
            '\u{0660}'..='\u{0669}' => {
                char::from_u32(c as u32 - '\u{0660}' as u32 + '0' as u32).unwrap_or(c)
            }
            '\u{06f0}'..='\u{06f9}' => {
                char::from_u32(c as u32 - '\u{06f0}' as u32 + '0' as u32).unwrap_or(c)
            }
            _ => c.to_lowercase().next().unwrap_or(c),
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_authorization::{MANAGER_RELATION, RECEPTIONIST_RELATION};
    use eitmad_contracts::{
        authorization::{RelationId, RelationshipSubject},
        commands::GrantScopeRelationship,
        identity::{
            AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId,
            TenantId,
        },
        material::{MaterialQuantity, UnitDimension},
        transport::{CorrelationId, IdempotencyKey, UnixMillis},
    };
    use eitmad_storage::DATABASE_FILE_NAME;
    use tempfile::TempDir;

    fn actor(principal: u128, organization: u128) -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::from_u128(principal + 1_000)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::from_u128(principal)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(Uuid::from_u128(organization)),
            workspace_id: None,
            scope: eitmad_contracts::identity::ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(Uuid::from_u128(organization)),
            },
        }
    }
    fn mutation(actor: AuthorizationContext, key: u128) -> MutationContext {
        MutationContext {
            authorization: actor,
            correlation_id: CorrelationId::new(Uuid::from_u128(key + 10_000)),
            causation_id: None,
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(key)),
            occurred_at: UnixMillis(i64::try_from(key).unwrap()),
        }
    }
    fn setup(
        dir: &TempDir,
    ) -> (
        AuthorityStore,
        MaterialService,
        AuthorizationContext,
        AuthorizationContext,
    ) {
        let store = AuthorityStore::open(dir.path()).unwrap();
        let auth = AuthorizationService::new(store.clone());
        let owner = actor(99, 50);
        let manager = actor(100, 50);
        let receptionist = actor(101, 50);
        auth.bootstrap_owner(
            &mutation(owner.clone(), 1),
            &RelationshipSubject {
                principal_id: owner.identity.principal_id,
                principal_kind: PrincipalKind::User,
            },
        )
        .unwrap();
        for (key, subject, relation, version) in [
            (2, manager.identity.principal_id, MANAGER_RELATION, 1),
            (
                3,
                receptionist.identity.principal_id,
                RECEPTIONIST_RELATION,
                2,
            ),
        ] {
            auth.grant_relationship(
                &mutation(owner.clone(), key),
                &GrantScopeRelationship {
                    expected_policy_version: version,
                    subject: RelationshipSubject {
                        principal_id: subject,
                        principal_kind: PrincipalKind::User,
                    },
                    relation: RelationId::parse(relation).unwrap(),
                },
            )
            .unwrap();
        }
        (
            store.clone(),
            MaterialService::new(store, auth),
            manager,
            receptionist,
        )
    }
    fn category(name: &str) -> SaveMaterialCategory {
        SaveMaterialCategory {
            id: None,
            expected_revision: None,
            name: name.to_owned(),
            archived: false,
        }
    }
    fn unit() -> SaveMaterialUnit {
        SaveMaterialUnit {
            id: None,
            expected_revision: None,
            name: "متر".to_owned(),
            symbol: "م".to_owned(),
            dimension: UnitDimension::Length,
            numerator: 1,
            denominator: 1,
            archived: false,
        }
    }

    #[test]
    fn manager_edits_survive_restart_and_arabic_search_uses_reference_names() {
        let dir = TempDir::new().unwrap();
        let (_store, service, manager, _) = setup(&dir);
        let category = service
            .save_category(&mutation(manager.clone(), 10), &category("أخشاب طبيعية"))
            .unwrap();
        let unit = service
            .save_unit(&mutation(manager.clone(), 11), &unit())
            .unwrap();
        let material = service
            .save_material(
                &mutation(manager.clone(), 12),
                &SaveMaterial {
                    id: None,
                    expected_revision: None,
                    name: "خشب زان".to_owned(),
                    category_id: category.id,
                    unit_id: unit.id,
                    current_cost_yer: 8_000,
                    archived: false,
                },
            )
            .unwrap();
        let edited = service
            .save_material(
                &mutation(manager.clone(), 13),
                &SaveMaterial {
                    id: Some(material.id),
                    expected_revision: Some(1),
                    name: "خشب زان".to_owned(),
                    category_id: category.id,
                    unit_id: unit.id,
                    current_cost_yer: 8_500,
                    archived: false,
                },
            )
            .unwrap();
        assert_eq!(edited.revision, 2);
        drop(service);
        let reopened = AuthorityStore::open(dir.path()).unwrap();
        let service = MaterialService::new(reopened.clone(), AuthorizationService::new(reopened));
        let page = service
            .list(
                &manager,
                &ListMaterials::new("اخشاب".to_owned(), None, 100).unwrap(),
            )
            .unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].current_cost_yer, 8_500);
        assert_eq!(page.items[0].id, material.id);
        assert_eq!(service.references(&manager).unwrap().units[0].id, unit.id);
    }

    #[test]
    fn reference_archive_preserves_links_and_stale_writes_fail() {
        let dir = TempDir::new().unwrap();
        let (_store, service, manager, _) = setup(&dir);
        let category = service
            .save_category(&mutation(manager.clone(), 20), &category("ألواح"))
            .unwrap();
        let unit = service
            .save_unit(&mutation(manager.clone(), 21), &unit())
            .unwrap();
        let material = service
            .save_material(
                &mutation(manager.clone(), 22),
                &SaveMaterial {
                    id: None,
                    expected_revision: None,
                    name: "لوح".to_owned(),
                    category_id: category.id,
                    unit_id: unit.id,
                    current_cost_yer: 10,
                    archived: false,
                },
            )
            .unwrap();
        service
            .save_category(
                &mutation(manager.clone(), 23),
                &SaveMaterialCategory {
                    id: Some(category.id),
                    expected_revision: Some(1),
                    name: "ألواح".to_owned(),
                    archived: true,
                },
            )
            .unwrap();
        let current = service
            .list(
                &manager,
                &ListMaterials::new(String::new(), None, 100).unwrap(),
            )
            .unwrap();
        assert_eq!(current.items[0].category_id, category.id);
        assert_eq!(
            service.save_material(
                &mutation(manager.clone(), 24),
                &SaveMaterial {
                    id: None,
                    expected_revision: None,
                    name: "لوح جديد".to_owned(),
                    category_id: category.id,
                    unit_id: unit.id,
                    current_cost_yer: 12,
                    archived: false
                }
            ),
            Err(MaterialError::InvalidReference)
        );
        assert_eq!(
            service.save_material(
                &mutation(manager.clone(), 25),
                &SaveMaterial {
                    id: Some(material.id),
                    expected_revision: Some(0),
                    name: "لوح".to_owned(),
                    category_id: category.id,
                    unit_id: unit.id,
                    current_cost_yer: 11,
                    archived: false
                }
            ),
            Err(MaterialError::Invalid)
        );
        assert!(matches!(
            service.save_material(
                &mutation(manager.clone(), 26),
                &SaveMaterial {
                    id: Some(material.id),
                    expected_revision: Some(2),
                    name: "لوح".to_owned(),
                    category_id: category.id,
                    unit_id: unit.id,
                    current_cost_yer: 11,
                    archived: false
                }
            ),
            Err(MaterialError::RevisionConflict { .. })
        ));
    }

    #[test]
    fn referenced_unit_definition_cannot_change() {
        let dir = TempDir::new().unwrap();
        let (_store, service, manager, _) = setup(&dir);
        let category = service
            .save_category(&mutation(manager.clone(), 40), &category("ألواح"))
            .unwrap();
        let unit = service
            .save_unit(&mutation(manager.clone(), 41), &unit())
            .unwrap();
        service
            .save_material(
                &mutation(manager.clone(), 42),
                &SaveMaterial {
                    id: None,
                    expected_revision: None,
                    name: "لوح".to_owned(),
                    category_id: category.id,
                    unit_id: unit.id,
                    current_cost_yer: 10,
                    archived: false,
                },
            )
            .unwrap();
        assert_eq!(
            service.save_unit(
                &mutation(manager, 43),
                &SaveMaterialUnit {
                    id: Some(unit.id),
                    expected_revision: Some(1),
                    name: unit.name,
                    symbol: unit.symbol,
                    dimension: UnitDimension::Area,
                    numerator: 1,
                    denominator: 1,
                    archived: false,
                }
            ),
            Err(MaterialError::InvalidReference)
        );
    }

    #[test]
    fn receptionist_cannot_mutate_and_cost_quantity_rules_reject_loss() {
        let dir = TempDir::new().unwrap();
        let (_store, service, manager, receptionist) = setup(&dir);
        assert_eq!(
            service.save_category(&mutation(receptionist.clone(), 31), &category("ممنوع")),
            Err(MaterialError::Denied)
        );
        assert_eq!(
            service.references(&receptionist),
            Err(MaterialError::Denied)
        );
        let category = service
            .save_category(&mutation(manager.clone(), 32), &category("أقمشة"))
            .unwrap();
        let unit = service
            .save_unit(&mutation(manager.clone(), 33), &unit())
            .unwrap();
        assert_eq!(
            service.save_material(
                &mutation(manager.clone(), 34),
                &SaveMaterial {
                    id: None,
                    expected_revision: None,
                    name: "قماش".to_owned(),
                    category_id: category.id,
                    unit_id: unit.id,
                    current_cost_yer: -1,
                    archived: false
                }
            ),
            Err(MaterialError::Invalid)
        );
        assert!(MaterialQuantity::parse("1.123456".to_owned()).is_ok());
        assert!(MaterialQuantity::parse("1.1234567".to_owned()).is_err());
        assert!(MaterialQuantity::parse("1e-2".to_owned()).is_err());
    }

    #[test]
    fn audit_failure_rolls_back_definition_and_event() {
        let dir = TempDir::new().unwrap();
        let (_store, service, manager, _) = setup(&dir);
        let connection = rusqlite::Connection::open(dir.path().join(DATABASE_FILE_NAME)).unwrap();
        let before: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM publication_outbox WHERE scope_kind='organization'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        connection.execute_batch("CREATE TRIGGER block_material_audit BEFORE INSERT ON mutation_audit
            WHEN NEW.operation='eitmad.material-category.save.v1' BEGIN SELECT RAISE(ABORT,'blocked'); END;").unwrap();
        assert_eq!(
            service.save_category(&mutation(manager.clone(), 40), &category("اختبار")),
            Err(MaterialError::Unavailable)
        );
        assert!(service.references(&manager).unwrap().categories.is_empty());
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM publication_outbox WHERE scope_kind='organization'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, before);
    }
}
