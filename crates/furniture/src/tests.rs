use super::*;
use eitmad_authorization::{MANAGER_RELATION, RECEPTIONIST_RELATION};
use eitmad_contracts::{
    authorization::{RelationId, RelationshipSubject},
    commands::GrantScopeRelationship,
    furniture::*,
    identity::{
        AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId, TenantId,
    },
    material::{
        Material, MaterialQuantity, MaterialUnit, SaveMaterial, SaveMaterialCategory,
        SaveMaterialUnit, UnitDimension,
    },
    part::{PartUsage, SavePart, SavePartCategory},
    transport::{CorrelationId, IdempotencyKey, UnixMillis},
};
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
    FurnitureService,
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
        FurnitureService::new(store, auth),
        manager,
        receptionist,
    )
}

use eitmad_material::MaterialService;
use eitmad_part::PartService;
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

fn fixture(
    store: &AuthorityStore,
    service: &FurnitureService,
    manager: &AuthorizationContext,
) -> SaveFurniture {
    let parts = PartService::new(store.clone(), AuthorizationService::new(store.clone()));
    let (input, _, _) = fixtures(store, &parts, manager);
    let part = parts.save(&mutation(manager.clone(), 20), &input).unwrap();
    let category = service
        .save_category(
            &mutation(manager.clone(), 21),
            &SaveFurnitureCategory {
                id: None,
                expected_revision: None,
                name: "غرف النوم".into(),
                archived: false,
            },
        )
        .unwrap();
    SaveFurniture {
        image: None,
        id: None,
        expected_revision: None,
        name: "خزانة اختبار".into(),
        category_id: category.id,
        description: "تعريف اختبار".into(),
        notes: "داخلي".into(),
        parts: vec![FurniturePart {
            reference: part.composition,
            quantity: 2,
        }],
        variants: vec![FurnitureVariant {
            id: FurnitureVariantId::new(Uuid::from_u128(900)),
            name: "صغير".into(),
            dimensions: dims(1200),
            customization: Some(FurnitureCustomization {
                minimum: dims(1000),
                maximum: dims(2000),
            }),
            selling_price_yer: 25000,
            archived: false,
            color_ids: vec![Uuid::from_u128(901)],
            handle_ids: vec![],
        }],
        colors: vec![FurnitureOption {
            id: Uuid::from_u128(901),
            name: "أبيض".into(),
            visual: "#FFFFFF".into(),
            price_adjustment_yer: 500,
            archived: false,
        }],
        handles: vec![FurnitureOption {
            id: Uuid::from_u128(902),
            name: "قياسي".into(),
            visual: "Standard".into(),
            price_adjustment_yer: 1000,
            archived: false,
        }],
        state: FurnitureState::Active,
        confirm_below_cost: false,
    }
}
fn dims(width: u32) -> FurnitureDimensions {
    FurnitureDimensions {
        width_mm: width,
        height_mm: 2000,
        depth_mm: 550,
    }
}
fn query(p: &Furniture, new: bool) -> GetFurnitureRevision {
    GetFurnitureRevision {
        reference: FurnitureReference {
            scope: p.scope.clone(),
            furniture_id: p.id,
            variant_id: p.variants[0].id,
            revision: p.revision,
            schema_version: 1,
        },
        for_new_work: new,
    }
}
fn list() -> ListFurnitures {
    ListFurnitures {
        term: String::new(),
        after: None,
        limit: 100,
        selectable_only: false,
    }
}
#[test]
fn definition_restarts_retries_and_preserves_snapshots_and_archive() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let mut input = fixture(&store, &service, &manager);
    let context = mutation(manager.clone(), 30);
    let saved = service.save(&context, &input).unwrap();
    assert_eq!(saved.parts_cost_yer, 18900);
    assert_eq!(service.save(&context, &input).unwrap(), saved);
    let reopened = AuthorityStore::open(dir.path()).unwrap();
    let service = FurnitureService::new(reopened.clone(), AuthorizationService::new(reopened));
    assert_eq!(
        service.list(&manager, &list()).unwrap().items,
        vec![saved.clone()]
    );
    input.id = Some(saved.id);
    input.expected_revision = Some(1);
    input.state = FurnitureState::Draft;
    input.name = "اسم جديد".into();
    input.variants[0].selling_price_yer = 0;
    let draft = service
        .save(&mutation(manager.clone(), 31), &input)
        .unwrap();
    assert_eq!(draft.revision, 2);
    assert_eq!(
        service.revision(&manager, &query(&saved, false)).unwrap(),
        saved
    );
    assert_eq!(
        service.revision(&manager, &query(&draft, true)),
        Err(FurnitureError::InvalidReference)
    );
    assert!(matches!(
        service.save(&mutation(manager.clone(), 32), &input),
        Err(FurnitureError::RevisionConflict {
            expected: Some(1),
            actual: Some(2)
        })
    ));
    input.expected_revision = Some(2);
    input.state = FurnitureState::Archived;
    let archived = service
        .save(&mutation(manager.clone(), 33), &input)
        .unwrap();
    assert_eq!(archived.state, FurnitureState::Archived);
    assert!(
        service
            .list(
                &manager,
                &ListFurnitures {
                    selectable_only: true,
                    ..list()
                }
            )
            .unwrap()
            .items
            .is_empty()
    );
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    assert!(
        connection
            .execute("UPDATE furniture_revisions SET revision=99", [])
            .is_err()
    );
    assert!(
        connection
            .execute("DELETE FROM furniture_revisions", [])
            .is_err()
    );
}
#[test]
fn invalid_relationships_quantities_bounds_options_and_money_are_rejected() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let input = fixture(&store, &service, &manager);
    let mut cases = Vec::new();
    let mut bad = input.clone();
    bad.parts[0].quantity = 0;
    cases.push(bad);
    let mut bad = input.clone();
    bad.parts.push(bad.parts[0].clone());
    cases.push(bad);
    let mut bad = input.clone();
    bad.parts[0].reference.scope.id =
        eitmad_contracts::identity::ScopeId::new(Uuid::from_u128(999));
    cases.push(bad);
    let mut bad = input.clone();
    bad.parts[0].reference.revision = 99;
    cases.push(bad);
    let mut bad = input.clone();
    bad.variants[0].dimensions.width_mm = 0;
    cases.push(bad);
    let mut bad = input.clone();
    bad.variants[0]
        .customization
        .as_mut()
        .unwrap()
        .maximum
        .width_mm = 1000;
    cases.push(bad);
    let mut bad = input.clone();
    bad.variants[0].color_ids = vec![Uuid::from_u128(999)];
    cases.push(bad);
    let mut bad = input.clone();
    bad.colors[0].archived = true;
    cases.push(bad);
    let mut bad = input.clone();
    bad.colors[0].price_adjustment_yer = -1;
    cases.push(bad);
    let mut bad = input.clone();
    bad.variants[0].selling_price_yer = i64::MAX;
    cases.push(bad);
    let mut bad = input.clone();
    bad.variants[0].selling_price_yer = 1;
    cases.push(bad);
    for (i, bad) in cases.iter().enumerate() {
        assert!(
            service
                .save(
                    &mutation(manager.clone(), 100 + u128::try_from(i).unwrap()),
                    bad
                )
                .is_err()
        );
    }
    assert!(service.list(&manager, &list()).unwrap().items.is_empty());
    let mut confirmed = input;
    confirmed.variants[0].selling_price_yer = 1;
    confirmed.confirm_below_cost = true;
    assert!(service.save(&mutation(manager, 200), &confirmed).is_ok());
}
#[test]
fn selection_checks_customization_option_compatibility_and_stale_revisions() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let mut input = fixture(&store, &service, &manager);
    let saved = service
        .save(&mutation(manager.clone(), 30), &input)
        .unwrap();
    let mut selection = CheckFurnitureSelection {
        reference: query(&saved, true).reference,
        dimensions: dims(1500),
        color_id: Some(input.colors[0].id),
        handle_id: Some(input.handles[0].id),
        quantity: 3,
    };
    let checked = service.selection(&manager, &selection).unwrap();
    assert_eq!(checked.unit_price_yer, 26500);
    assert_eq!(checked.total_yer, 79500);
    selection.dimensions.width_mm = 999;
    assert_eq!(
        service.selection(&manager, &selection),
        Err(FurnitureError::Invalid)
    );
    selection.dimensions.width_mm = 1500;
    selection.color_id = Some(Uuid::from_u128(999));
    assert_eq!(
        service.selection(&manager, &selection),
        Err(FurnitureError::InvalidReference)
    );
    selection.color_id = Some(input.colors[0].id);
    input.id = Some(saved.id);
    input.expected_revision = Some(1);
    input.variants[0].customization = None;
    let fixed = service
        .save(&mutation(manager.clone(), 31), &input)
        .unwrap();
    assert_eq!(
        service.selection(&manager, &selection),
        Err(FurnitureError::InvalidReference)
    );
    selection.reference = query(&fixed, true).reference;
    assert_eq!(
        service.selection(&manager, &selection),
        Err(FurnitureError::Invalid)
    );
    selection.dimensions = dims(1200);
    assert!(service.selection(&manager, &selection).is_ok());
}
#[test]
fn manager_only_writes_reads_and_audit_failure_roll_back() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, receptionist) = setup(&dir);
    let input = fixture(&store, &service, &manager);
    assert_eq!(
        service.save(&mutation(receptionist.clone(), 30), &input),
        Err(FurnitureError::Denied)
    );
    assert_eq!(
        service.list(&receptionist, &list()),
        Err(FurnitureError::Denied)
    );
    assert_eq!(
        service.review(&receptionist, &input),
        Err(FurnitureError::Denied)
    );
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_furniture_audit BEFORE INSERT ON mutation_audit WHEN NEW.operation='eitmad.furniture.save.v1' BEGIN SELECT RAISE(ABORT,'test audit failure'); END;").unwrap();
    assert_eq!(
        service.save(&mutation(manager.clone(), 31), &input),
        Err(FurnitureError::Unavailable)
    );
    assert!(service.list(&manager, &list()).unwrap().items.is_empty());
    connection
        .execute_batch("DROP TRIGGER fail_furniture_audit;")
        .unwrap();
    assert_eq!(
        service
            .save(&mutation(manager, 31), &input)
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn part_changes_preserve_composition_and_archived_option_identities() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&dir);
    let mut input = fixture(&store, &service, &manager);
    let saved = service
        .save(&mutation(manager.clone(), 30), &input)
        .unwrap();
    let parts = PartService::new(store.clone(), AuthorizationService::new(store.clone()));
    let part = store
        .read_furnitures(|tx| tx.part(&manager.scope, input.parts[0].reference.part_id.value()))
        .unwrap()
        .unwrap();
    let revised = parts
        .save(
            &mutation(manager.clone(), 31),
            &SavePart {
                id: Some(part.id),
                expected_revision: Some(part.revision),
                name: part.name,
                category_id: part.category_id,
                description: part.description,
                usages: part.cost.rows.into_iter().map(|row| row.usage).collect(),
                archived: true,
            },
        )
        .unwrap();
    assert_eq!(revised.revision, 2);
    let mut new_definition = input.clone();
    new_definition.variants[0].id = FurnitureVariantId::new(Uuid::from_u128(950));
    new_definition.colors[0].id = Uuid::from_u128(951);
    new_definition.variants[0].color_ids = vec![new_definition.colors[0].id];
    new_definition.handles[0].id = Uuid::from_u128(952);
    assert_eq!(
        service.save(&mutation(manager.clone(), 32), &new_definition),
        Err(FurnitureError::InvalidReference)
    );
    input.id = Some(saved.id);
    input.expected_revision = Some(saved.revision);
    input.parts[0].quantity = 3;
    input.variants[0].selling_price_yer = 50000;
    input.colors.clear();
    input.variants[0].color_ids.clear();
    let updated = service
        .save(&mutation(manager.clone(), 33), &input)
        .unwrap();
    assert_eq!(updated.parts_cost_yer, 28350);
    assert_eq!(updated.parts[0].reference, saved.parts[0].reference);
    assert!(updated.colors[0].archived);
    assert_eq!(
        service.revision(&manager, &query(&saved, false)).unwrap(),
        saved
    );
    input.expected_revision = Some(updated.revision);
    let mut cross_kind = input.clone();
    cross_kind.handles.push(FurnitureOption {
        visual: "Brass".into(),
        ..saved.colors[0].clone()
    });
    assert_eq!(
        service.save(&mutation(manager.clone(), 34), &cross_kind),
        Err(FurnitureError::InvalidReference)
    );
    input.colors = saved.colors;
    assert_eq!(
        service.save(&mutation(manager, 35), &input),
        Err(FurnitureError::InvalidReference)
    );
}

#[test]
fn image_replacement_retains_historical_furniture_references() {
    use eitmad_contracts::catalog_image::{CatalogImageKind, ImportCatalogImage};
    let directory = TempDir::new().unwrap();
    let (store, service, manager, _) = setup(&directory);
    let images = eitmad_catalog_image::CatalogImageService::new(
        store.clone(),
        AuthorizationService::new(store.clone()),
    );
    let mut input = fixture(&store, &service, &manager);
    let path = directory.path().join("synthetic.png");
    let mut refs = Vec::new();
    for (key, width) in [(71, 7), (72, 11)] {
        image::DynamicImage::new_rgb8(width, 9)
            .save_with_format(&path, image::ImageFormat::Png)
            .unwrap();
        refs.push(
            images
                .import(
                    &mutation(manager.clone(), key),
                    &ImportCatalogImage {
                        kind: CatalogImageKind::Furniture,
                        source_path: path.to_str().unwrap().into(),
                    },
                    eitmad_contracts::transport::UnixMillis(i64::MAX),
                )
                .unwrap(),
        );
    }
    input.image = Some(Box::new(refs[0].clone()));
    let first = service
        .save(&mutation(manager.clone(), 73), &input)
        .unwrap();
    input.id = Some(first.id);
    input.expected_revision = Some(first.revision);
    input.image = Some(Box::new(refs[1].clone()));
    let second = service
        .save(&mutation(manager.clone(), 74), &input)
        .unwrap();
    assert_eq!(
        service
            .revision(&manager, &query(&first, false))
            .unwrap()
            .image,
        first.image
    );
    assert_eq!(second.image.as_deref(), Some(&refs[1]));
    assert!(
        store
            .catalog_image(&manager.scope, &refs[0])
            .unwrap()
            .is_some()
    );
}
