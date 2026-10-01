use super::*;
use eitmad_authorization::{MANAGER_RELATION, RECEPTIONIST_RELATION};
use eitmad_contracts::{
    authorization::{RelationId, RelationshipSubject},
    commands::GrantScopeRelationship,
    identity::{
        AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId, TenantId,
    },
    product::{ProductReference, ProductVariantId, SaveProductVariant},
    transport::{CorrelationId, IdempotencyKey, UnixMillis},
};
use tempfile::TempDir;
/// Creates a deterministic synthetic user and organization authorization context.
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
/// Creates deterministic audit and retry metadata for a synthetic operation.
fn mutation(actor: AuthorizationContext, key: u128) -> MutationContext {
    MutationContext {
        authorization: actor,
        correlation_id: CorrelationId::new(Uuid::from_u128(key + 10_000)),
        causation_id: None,
        idempotency_key: IdempotencyKey::new(Uuid::from_u128(key)),
        occurred_at: UnixMillis(i64::try_from(key).unwrap()),
    }
}
/// Bootstraps isolated authority storage with Manager and Receptionist relationships.
fn setup(
    dir: &TempDir,
) -> (
    AuthorityStore,
    ProductService,
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
        ProductService::new(store, auth),
        manager,
        receptionist,
    )
}

/// Creates a synthetic active category and an unsaved product with fixed supplier options.
fn fixture(service: &ProductService, manager: &AuthorizationContext) -> SaveProduct {
    let category = service
        .save_category(
            &mutation(manager.clone(), 10),
            &SaveProductCategory {
                id: None,
                expected_revision: None,
                name: "مراتب".into(),
                archived: false,
            },
        )
        .unwrap();
    SaveProduct {
        id: None,
        expected_revision: None,
        name: "مرتبة طبية".into(),
        category_id: category.id,
        description: "منتج جاهز".into(),
        notes: "ملاحظة داخلية".into(),
        variants: vec![
            SaveProductVariant {
                id: ProductVariantId::new(Uuid::from_u128(900)),
                name: "مفرد".into(),
                purchase_cost_yer: 55000,
                archived: false,
            },
            SaveProductVariant {
                id: ProductVariantId::new(Uuid::from_u128(901)),
                name: "مزدوج".into(),
                purchase_cost_yer: 80000,
                archived: false,
            },
        ],
        archived: false,
    }
}
/// Builds an exact historical option reference with an optional new-work eligibility check.
fn reference(p: &Product, variant: usize, new: bool) -> GetProductRevision {
    GetProductRevision {
        reference: ProductReference {
            scope: p.scope.clone(),
            product_id: p.id,
            variant_id: p.variants[variant].id,
            revision: p.revision,
            schema_version: 1,
        },
        for_new_work: new,
    }
}
/// Builds a bounded product-list request without search or selection restrictions.
fn list() -> ListProducts {
    ListProducts {
        term: String::new(),
        after: None,
        limit: 100,
        selectable_only: false,
    }
}
/// Verifies product reads see committed state without reserving the writer lock.
#[test]
fn product_queries_read_committed_data_while_a_writer_holds_the_database() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let input = fixture(&service, &manager);
    let product = service
        .save(&mutation(manager.clone(), 20), &input)
        .unwrap();
    let mut writer = rusqlite::Connection::open(store.path()).unwrap();
    let _transaction = writer
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    assert_eq!(
        service.list(&manager, &list()).unwrap().items,
        vec![product.clone()]
    );
    assert_eq!(
        service
            .revision(&manager, &reference(&product, 0, true))
            .unwrap(),
        product
    );
    assert_eq!(
        service
            .categories(
                &manager,
                &ListProductCategories {
                    after: None,
                    limit: 100
                }
            )
            .unwrap()
            .items[0]
            .id,
        input.category_id
    );
}

/// Verifies restart persistence, exact retry, immutable history, and retained option references.
#[test]
fn restart_retry_and_history_preserve_fixed_supplier_references() {
    let dir = TempDir::new().unwrap();
    let (_, service, manager, _) = setup(&dir);
    let mut input = fixture(&service, &manager);
    let context = mutation(manager.clone(), 20);
    let first = service.save(&context, &input).unwrap();
    assert_eq!(service.save(&context, &input).unwrap(), first);
    input.name = "طلب مختلف".into();
    assert_eq!(service.save(&context, &input), Err(ProductError::Invalid));
    input.id = Some(first.id);
    input.expected_revision = Some(1);
    input.name = "مرتبة حديثة".into();
    input.variants[0].name = "مفرد جديد".into();
    input.variants[0].purchase_cost_yer = 60000;
    input.variants.pop();
    let changed = service
        .save(&mutation(manager.clone(), 21), &input)
        .unwrap();
    assert_eq!(changed.variants[1].id, first.variants[1].id);
    assert!(changed.variants[1].archived);
    let store = AuthorityStore::open(dir.path()).unwrap();
    let service = ProductService::new(store.clone(), AuthorizationService::new(store));
    assert_eq!(
        service
            .revision(&manager, &reference(&first, 1, false))
            .unwrap(),
        first
    );
    assert_eq!(service.list(&manager, &list()).unwrap().items[0], changed);
    assert_eq!(
        service.revision(&manager, &reference(&first, 0, true)),
        Err(ProductError::InvalidReference)
    );
    assert_eq!(
        service.revision(&manager, &reference(&changed, 1, true)),
        Err(ProductError::InvalidReference)
    );
    assert!(
        service
            .revision(&manager, &reference(&changed, 0, true))
            .is_ok()
    );
    let page = service
        .list(
            &manager,
            &ListProducts {
                selectable_only: true,
                ..list()
            },
        )
        .unwrap();
    assert_eq!(page.items[0].variants.len(), 1);
    input.expected_revision = Some(changed.revision);
    input.archived = true;
    let archived = service
        .save(&mutation(manager.clone(), 22), &input)
        .unwrap();
    assert!(
        service
            .list(
                &manager,
                &ListProducts {
                    selectable_only: true,
                    ..list()
                }
            )
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        service.revision(&manager, &reference(&archived, 0, true)),
        Err(ProductError::InvalidReference)
    );
    assert_eq!(
        service
            .revision(&manager, &reference(&first, 0, false))
            .unwrap(),
        first
    );
}
/// Verifies scoped access, cost redaction, and denial after relationship revocation.
#[test]
fn denied_changes_cost_redaction_cross_scope_and_revocation() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, receptionist) = setup(&dir);
    let input = fixture(&service, &manager);
    let saved = service
        .save(&mutation(manager.clone(), 20), &input)
        .unwrap();
    assert_eq!(
        service.save(&mutation(receptionist.clone(), 21), &input),
        Err(ProductError::Denied)
    );
    assert_eq!(
        service.save_category(
            &mutation(receptionist.clone(), 22),
            &SaveProductCategory {
                id: None,
                expected_revision: None,
                name: "أخرى".into(),
                archived: false
            }
        ),
        Err(ProductError::Denied)
    );
    for value in [
        service
            .list(&receptionist, &list())
            .unwrap()
            .items
            .remove(0),
        service
            .revision(&receptionist, &reference(&saved, 0, false))
            .unwrap(),
    ] {
        assert!(value.variants.iter().all(|v| v.purchase_cost_yer.is_none()));
        assert!(value.notes.is_empty());
        assert!(
            !serde_json::to_string(&value)
                .unwrap()
                .contains("purchaseCostYer")
        );
    }
    let mut other = manager.clone();
    other.scope = actor(100, 51).scope;
    other.tenant_id = actor(100, 51).tenant_id;
    assert_eq!(service.list(&other, &list()), Err(ProductError::Denied));
    let mut cross = reference(&saved, 0, false);
    cross.reference.scope = other.scope;
    assert_eq!(
        service.revision(&manager, &cross),
        Err(ProductError::InvalidReference)
    );
    let mut bad = input.clone();
    bad.category_id = ProductCategoryId::new(Uuid::new_v4());
    bad.variants[0].id = ProductVariantId::new(Uuid::new_v4());
    bad.variants[1].id = ProductVariantId::new(Uuid::new_v4());
    assert_eq!(
        service.save(&mutation(manager.clone(), 23), &bad),
        Err(ProductError::InvalidReference)
    );

    // Revoke using the authorization API so persisted policy changes take effect.
    let auth = AuthorizationService::new(store);
    let owner = actor(99, 50);
    let relationships = auth
        .list_relationships(
            &owner,
            &eitmad_contracts::queries::ListScopeRelationships::new(None, 100).unwrap(),
        )
        .unwrap();
    let target = relationships
        .relationships
        .iter()
        .find(|r| r.subject.principal_id == manager.identity.principal_id)
        .unwrap();
    auth.revoke_relationship(
        &mutation(owner, 24),
        &eitmad_contracts::commands::RevokeScopeRelationship {
            expected_policy_version: 3,
            relationship_id: target.relationship_id,
        },
    )
    .unwrap();
    assert_eq!(service.list(&manager, &list()), Err(ProductError::Denied));
}
/// Verifies stale revisions, invalid bounds, option ownership, and archived-category restrictions.
#[test]
fn revisions_bounds_variant_ownership_and_category_archive_are_enforced() {
    let dir = TempDir::new().unwrap();
    let (_, service, manager, _) = setup(&dir);
    let mut input = fixture(&service, &manager);
    let first = service
        .save(&mutation(manager.clone(), 20), &input)
        .unwrap();
    assert_eq!(
        service.save(&mutation(manager.clone(), 21), &input),
        Err(ProductError::InvalidReference)
    );
    input.id = Some(first.id);
    input.expected_revision = Some(1);
    let second = service
        .save(&mutation(manager.clone(), 22), &input)
        .unwrap();
    assert!(matches!(
        service.save(&mutation(manager.clone(), 23), &input),
        Err(ProductError::RevisionConflict {
            actual: Some(2),
            ..
        })
    ));
    assert_eq!(
        service.list(&manager, &ListProducts { limit: 0, ..list() }),
        Err(ProductError::Invalid)
    );
    assert_eq!(
        service.list(
            &manager,
            &ListProducts {
                limit: 101,
                ..list()
            }
        ),
        Err(ProductError::Invalid)
    );
    let category = service
        .categories(
            &manager,
            &ListProductCategories {
                after: None,
                limit: 1,
            },
        )
        .unwrap()
        .items
        .remove(0);
    service
        .save_category(
            &mutation(manager.clone(), 24),
            &SaveProductCategory {
                id: Some(category.id),
                expected_revision: Some(1),
                name: "مراتب مؤرشفة".into(),
                archived: true,
            },
        )
        .unwrap();
    assert!(
        service
            .list(
                &manager,
                &ListProducts {
                    selectable_only: true,
                    ..list()
                }
            )
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        service.revision(&manager, &reference(&second, 0, true)),
        Err(ProductError::InvalidReference)
    );
    assert_eq!(
        service
            .revision(&manager, &reference(&first, 0, false))
            .unwrap()
            .category_name,
        "مراتب"
    );
    input.expected_revision = Some(2);
    input.variants[0].archived = true;
    let archived = service
        .save(&mutation(manager.clone(), 25), &input)
        .unwrap();
    input.expected_revision = Some(archived.revision);
    input.variants[0].archived = false;
    assert_eq!(
        service.save(&mutation(manager.clone(), 26), &input),
        Err(ProductError::InvalidReference)
    );
}
/// Verifies an audit failure rolls back the definition, history, retry result, and publication.
#[test]
fn audit_failure_rolls_back_definition_history_retry_and_event() {
    let dir = TempDir::new().unwrap();
    let (_, service, manager, _) = setup(&dir);
    let input = fixture(&service, &manager);
    let conn =
        rusqlite::Connection::open(dir.path().join(eitmad_storage::DATABASE_FILE_NAME)).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM publication_outbox", [], |r| r.get(0))
        .unwrap();
    conn.execute_batch("CREATE TRIGGER block_product_audit BEFORE INSERT ON mutation_audit WHEN NEW.operation='eitmad.product.save.v1' BEGIN SELECT RAISE(ABORT,'blocked'); END;").unwrap();
    assert_eq!(
        service.save(&mutation(manager, 20), &input),
        Err(ProductError::Unavailable)
    );
    for table in ["products", "product_revisions", "product_variants"] {
        assert_eq!(
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM publication_outbox", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        count
    );
}
