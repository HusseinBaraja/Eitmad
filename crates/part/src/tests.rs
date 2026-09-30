use super::*;
use eitmad_authorization::{MANAGER_RELATION, RECEPTIONIST_RELATION};
use eitmad_contracts::{
    authorization::{RelationId, RelationshipSubject},
    commands::GrantScopeRelationship,
    identity::{
        AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId, TenantId,
    },
    material::{
        Material, MaterialUnit, SaveMaterial, SaveMaterialCategory, SaveMaterialUnit, UnitDimension,
    },
    transport::{CorrelationId, IdempotencyKey, UnixMillis},
};
use eitmad_material::MaterialService;
use tempfile::TempDir;

/// Creates two materials in distinct dimensions and an independently specified approved composition.
fn fixtures(
    store: &AuthorityStore,
    service: &PartService,
    manager: &AuthorizationContext,
) -> (SavePart, Vec<Material>, Vec<MaterialUnit>) {
    let materials = MaterialService::new(store.clone(), AuthorizationService::new(store.clone()));
    let category = materials
        .save_category(
            &mutation(manager.clone(), 10),
            &SaveMaterialCategory {
                id: None,
                expected_revision: None,
                name: "أخشاب".into(),
                archived: false,
            },
        )
        .unwrap();
    let part_category = service
        .save_category(
            &mutation(manager.clone(), 11),
            &SavePartCategory {
                id: None,
                expected_revision: None,
                name: "خزانة ملابس".into(),
                archived: false,
            },
        )
        .unwrap();
    let mut records = Vec::new();
    let mut units = Vec::new();
    for (key, name, symbol, dimension, cost) in [
        (12, "MDF 18mm", "m²", UnitDimension::Area, 7250),
        (14, "Edge Band", "m", UnitDimension::Length, 250),
    ] {
        let unit = materials
            .save_unit(
                &mutation(manager.clone(), key),
                &SaveMaterialUnit {
                    id: None,
                    expected_revision: None,
                    name: symbol.into(),
                    symbol: symbol.into(),
                    dimension,
                    numerator: 1,
                    denominator: 1,
                    archived: false,
                },
            )
            .unwrap();
        records.push(
            materials
                .save_material(
                    &mutation(manager.clone(), key + 1),
                    &SaveMaterial {
                        id: None,
                        expected_revision: None,
                        name: name.into(),
                        category_id: category.id,
                        unit_id: unit.id,
                        current_cost_yer: cost,
                        archived: false,
                    },
                )
                .unwrap(),
        );
        units.push(unit);
    }
    let usages = records
        .iter()
        .zip(&units)
        .zip(["1.2", "3"])
        .map(|((m, u), q)| PartUsage {
            material_id: m.id,
            material_revision: m.revision,
            unit_id: u.id,
            unit_revision: u.revision,
            quantity: MaterialQuantity::parse(q.into()).unwrap(),
        })
        .collect();
    (
        SavePart {
            id: None,
            expected_revision: None,
            name: "جانب خزانة".into(),
            category_id: part_category.id,
            description: "جزء تجريبي".into(),
            usages,
            archived: false,
        },
        records,
        units,
    )
}

/// Protects restart durability and immutable identity and costs across exact retries.
#[test]
fn multi_material_save_reopens_and_exact_retry_preserves_composition() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let (input, _, _) = fixtures(&store, &service, &manager);
    let context = mutation(manager.clone(), 20);
    let saved = service.save(&context, &input).unwrap();
    assert_eq!(saved.cost.total_cost_yer, 9450);
    assert_eq!(service.save(&context, &input).unwrap(), saved);
    let mut different_retry = input.clone();
    different_retry.name = "طلب مختلف".into();
    assert_eq!(
        service.save(&context, &different_retry),
        Err(PartError::Invalid)
    );
    let reopened = AuthorityStore::open(dir.path()).unwrap();
    let service = PartService::new(reopened.clone(), AuthorizationService::new(reopened));
    let page = service
        .list(
            &manager,
            &ListParts {
                term: "خزانه".into(),
                after: None,
                limit: 1,
            },
        )
        .unwrap();
    assert_eq!(page.items[0].part, saved);
    assert_eq!(page.items[0].current_cost, saved.cost);
    assert_eq!(
        service
            .composition(
                &manager,
                &GetPartComposition {
                    reference: saved.composition.clone()
                }
            )
            .unwrap(),
        saved
    );
    let connection =
        rusqlite::Connection::open(dir.path().join(eitmad_storage::DATABASE_FILE_NAME)).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM part_material_usages", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM mutation_audit WHERE operation='eitmad.part.save.v1'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

/// Protects historical snapshots and rejects saving a review made against stale material revisions.
#[test]
fn current_cost_changes_never_rewrite_historical_composition_or_accept_stale_review() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let (mut input, records, _) = fixtures(&store, &service, &manager);
    let saved = service
        .save(&mutation(manager.clone(), 20), &input)
        .unwrap();
    let materials = MaterialService::new(store.clone(), AuthorizationService::new(store));
    let old = &records[0];
    let changed = materials
        .save_material(
            &mutation(manager.clone(), 21),
            &SaveMaterial {
                id: Some(old.id),
                expected_revision: Some(old.revision),
                name: old.name.clone(),
                category_id: old.category_id,
                unit_id: old.unit_id,
                current_cost_yer: 8000,
                archived: false,
            },
        )
        .unwrap();
    let current = service
        .list(
            &manager,
            &ListParts {
                term: String::new(),
                after: None,
                limit: 100,
            },
        )
        .unwrap()
        .items
        .remove(0);
    assert_eq!(current.current_cost.total_cost_yer, 10350);
    assert_eq!(current.part.cost.total_cost_yer, 9450);
    assert_eq!(
        service
            .composition(
                &manager,
                &GetPartComposition {
                    reference: saved.composition.clone()
                }
            )
            .unwrap()
            .cost
            .total_cost_yer,
        9450
    );
    input.id = Some(saved.id);
    input.expected_revision = Some(1);
    assert!(matches!(
        service.save(&mutation(manager.clone(), 22), &input),
        Err(PartError::RevisionConflict { .. })
    ));
    input.usages[0].material_revision = changed.revision;
    let updated = service
        .save(&mutation(manager.clone(), 23), &input)
        .unwrap();
    assert_eq!(updated.revision, 2);
    assert_eq!(updated.cost.total_cost_yer, 10350);
    assert_eq!(
        service
            .composition(
                &manager,
                &GetPartComposition {
                    reference: saved.composition
                }
            )
            .unwrap()
            .cost
            .total_cost_yer,
        9450
    );
}

/// Protects exact conversion, aggregate rounding, quantity validation, and money overflow rejection.
#[test]
fn exact_conversion_and_single_total_rounding_reject_loss_and_overflow() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let (mut input, _, units) = fixtures(&store, &service, &manager);
    for usage in &mut input.usages {
        usage.quantity = MaterialQuantity::parse("0.001".into()).unwrap();
    }
    // 7.25 + 0.25 = 7.5; round the aggregate once to 8, not 7.
    assert_eq!(
        service
            .cost(
                &manager,
                &CalculatePartCost {
                    part_id: None,
                    usages: input.usages.clone()
                }
            )
            .unwrap()
            .total_cost_yer,
        8
    );
    let materials = MaterialService::new(store.clone(), AuthorizationService::new(store));
    let cm = materials
        .save_unit(
            &mutation(manager.clone(), 24),
            &SaveMaterialUnit {
                id: None,
                expected_revision: None,
                name: "سنتيمتر".into(),
                symbol: "سم".into(),
                dimension: UnitDimension::Length,
                numerator: 1,
                denominator: 100,
                archived: false,
            },
        )
        .unwrap();
    input.usages.remove(0);
    input.usages[0].unit_id = cm.id;
    input.usages[0].unit_revision = cm.revision;
    input.usages[0].quantity = MaterialQuantity::parse("300".into()).unwrap();
    service
        .save(&mutation(manager.clone(), 25), &input)
        .unwrap();
    assert_eq!(
        materials.save_unit(
            &mutation(manager.clone(), 26),
            &SaveMaterialUnit {
                id: Some(cm.id),
                expected_revision: Some(cm.revision),
                name: cm.name.clone(),
                symbol: cm.symbol.clone(),
                dimension: cm.dimension,
                numerator: 2,
                denominator: 100,
                archived: false,
            }
        ),
        Err(eitmad_material::MaterialError::InvalidReference)
    );
    assert_eq!(
        service
            .cost(
                &manager,
                &CalculatePartCost {
                    part_id: None,
                    usages: input.usages.clone()
                }
            )
            .unwrap()
            .total_cost_yer,
        750
    );
    input.usages[0].quantity =
        MaterialQuantity::parse("99999999999999999999999999999999".into()).unwrap();
    assert_eq!(
        service.cost(
            &manager,
            &CalculatePartCost {
                part_id: None,
                usages: input.usages.clone()
            }
        ),
        Err(PartError::Invalid)
    );
    for q in ["0", "-1", "1e2", "1.0000001"] {
        assert!(MaterialQuantity::parse(q.into()).is_err());
    }
    input.usages[0].unit_id = units[0].id;
    assert_eq!(
        service.cost(
            &manager,
            &CalculatePartCost {
                part_id: None,
                usages: input.usages
            }
        ),
        Err(PartError::InvalidReference)
    );
}

/// Protects Rust permission, scope, reference, and duplicate-material enforcement.
#[test]
fn permission_scope_reference_and_duplicate_validation_is_authoritative() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, receptionist) = setup(&dir);
    let (mut input, _, _) = fixtures(&store, &service, &manager);
    assert_eq!(
        service.save(&mutation(receptionist.clone(), 20), &input),
        Err(PartError::Denied)
    );
    assert_eq!(
        service.list(
            &receptionist,
            &ListParts {
                term: String::new(),
                after: None,
                limit: 100
            }
        ),
        Err(PartError::Denied)
    );
    input.usages.push(input.usages[0].clone());
    assert_eq!(
        service.save(&mutation(manager.clone(), 21), &input),
        Err(PartError::Invalid)
    );
    input.usages.pop();
    input.category_id = PartCategoryId::new(Uuid::new_v4());
    assert_eq!(
        service.save(&mutation(manager.clone(), 22), &input),
        Err(PartError::InvalidReference)
    );
    let mut foreign = manager.clone();
    foreign.scope.id = ScopeId::new(Uuid::new_v4());
    assert_eq!(
        service.save(&mutation(foreign, 23), &input),
        Err(PartError::Denied)
    );
    assert_eq!(
        service.list(
            &manager,
            &ListParts {
                term: String::new(),
                after: None,
                limit: 101
            }
        ),
        Err(PartError::Invalid)
    );
}

/// Protects optimistic concurrency so only one update to an expected revision succeeds.
#[test]
fn concurrent_edit_has_one_winner_and_does_not_overwrite() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let (mut input, _, _) = fixtures(&store, &service, &manager);
    let saved = service
        .save(&mutation(manager.clone(), 20), &input)
        .unwrap();
    input.id = Some(saved.id);
    input.expected_revision = Some(1);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles = (0..2)
        .map(|n| {
            let service = service.clone();
            let manager = manager.clone();
            let barrier = barrier.clone();
            let mut input = input.clone();
            input.name = format!("تعديل {n}");
            std::thread::spawn(move || {
                barrier.wait();
                service.save(&mutation(manager, 30 + n), &input)
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(
                r,
                Err(PartError::RevisionConflict {
                    actual: Some(2),
                    ..
                })
            ))
            .count(),
        1
    );
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    let connection =
        rusqlite::Connection::open(dir.path().join(eitmad_storage::DATABASE_FILE_NAME)).unwrap();
    let conflict_evidence: String = connection.query_row("SELECT redacted_error FROM mutation_audit WHERE operation='eitmad.part.save.v1' AND outcome=?1", [serde_json::to_string(&AuditOutcome::Conflict).unwrap()], |r| r.get(0)).unwrap();
    assert!(conflict_evidence.contains("eitmad.error.part-revision-conflict.v1"));
    assert_eq!(
        service
            .list(
                &manager,
                &ListParts {
                    term: String::new(),
                    after: None,
                    limit: 100
                }
            )
            .unwrap()
            .items[0]
            .part
            .name,
        winner.name
    );
}

/// Protects atomic rollback when required audit storage fails.
#[test]
fn mandatory_audit_failure_rolls_back_part_composition_usage_and_publication() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let (input, _, _) = fixtures(&store, &service, &manager);
    let connection =
        rusqlite::Connection::open(dir.path().join(eitmad_storage::DATABASE_FILE_NAME)).unwrap();
    let before = connection
        .query_row("SELECT COUNT(*) FROM publication_outbox", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap();
    connection.execute_batch("CREATE TRIGGER block_part_audit BEFORE INSERT ON mutation_audit WHEN NEW.operation='eitmad.part.save.v1' BEGIN SELECT RAISE(ABORT,'blocked'); END;").unwrap();
    assert_eq!(
        service.save(&mutation(manager.clone(), 20), &input),
        Err(PartError::Unavailable)
    );
    for table in ["parts", "part_compositions", "part_material_usages"] {
        assert_eq!(
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM publication_outbox", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        before
    );
}
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
    PartService,
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
        PartService::new(store, auth),
        manager,
        receptionist,
    )
}

/// Protects pagination and retention of archived references only for existing compositions.
#[test]
fn pages_are_bounded_and_archived_materials_remain_only_in_existing_compositions() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let (mut input, records, _) = fixtures(&store, &service, &manager);
    let saved = service
        .save(&mutation(manager.clone(), 40), &input)
        .unwrap();
    service
        .save_category(
            &mutation(manager.clone(), 41),
            &SavePartCategory {
                id: None,
                expected_revision: None,
                name: "أبواب".into(),
                archived: false,
            },
        )
        .unwrap();
    let first = service
        .categories(
            &manager,
            &ListPartCategories {
                after: None,
                limit: 1,
            },
        )
        .unwrap();
    assert_eq!(first.items.len(), 1);
    let second = service
        .categories(
            &manager,
            &ListPartCategories {
                after: first.next,
                limit: 1,
            },
        )
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert_ne!(first.items[0].id, second.items[0].id);
    assert!(second.next.is_none());
    assert_eq!(
        service.categories(
            &manager,
            &ListPartCategories {
                after: None,
                limit: 0
            }
        ),
        Err(PartError::Invalid)
    );
    let materials = MaterialService::new(store.clone(), AuthorizationService::new(store));
    let old = &records[0];
    let archived = materials
        .save_material(
            &mutation(manager.clone(), 42),
            &SaveMaterial {
                id: Some(old.id),
                expected_revision: Some(old.revision),
                name: old.name.clone(),
                category_id: old.category_id,
                unit_id: old.unit_id,
                current_cost_yer: old.current_cost_yer,
                archived: true,
            },
        )
        .unwrap();
    input.usages[0].material_revision = archived.revision;
    assert_eq!(
        service.save(&mutation(manager.clone(), 43), &input),
        Err(PartError::InvalidReference)
    );
    input.id = Some(saved.id);
    input.expected_revision = Some(saved.revision);
    input.archived = true;
    assert!(
        service
            .save(&mutation(manager, 44), &input)
            .unwrap()
            .archived
    );
}
