use super::*;
mod quotation;
use eitmad_authorization::{MANAGER_RELATION, RECEPTIONIST_RELATION};
use eitmad_contracts::{
    authorization::{RelationId, RelationshipSubject},
    commands::GrantScopeRelationship,
    identity::{
        AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId, TenantId,
    },
    product::{ProductVariantId, SaveProduct, SaveProductCategory, SaveProductVariant},
    transport::{CorrelationId, IdempotencyKey, UnixMillis},
};
use eitmad_product::ProductService;
use tempfile::TempDir;
use uuid::Uuid;

fn project_sales(
    store: &AuthorityStore,
    reader: &AuthorizationContext,
    entries: Vec<(
        u128,
        u64,
        Option<eitmad_contracts::catalog_revision::CatalogEntry>,
    )>,
) {
    use eitmad_contracts::sync::{ChangeId, ChangeOperation, ChangeRecord, Checkpoint, RecordId};
    let records: Vec<_> = entries
        .into_iter()
        .map(|(id, revision, entry)| {
            (
                ChangeRecord {
                    change_id: ChangeId::new(Uuid::new_v4()),
                    record_id: RecordId::new(Uuid::from_u128(id)),
                    scope: reader.scope.clone(),
                    operation: if entry.is_some() {
                        ChangeOperation::Upsert
                    } else {
                        ChangeOperation::Tombstone
                    },
                    base_revision: None,
                    revision,
                    changed_at: UnixMillis(100),
                    idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                    payload: None,
                    merge: None,
                },
                entry,
            )
        })
        .collect();
    let evidence = MutationAuditRecord::from_authorization(
        reader,
        UnixMillis(100),
        CorrelationId::new(Uuid::new_v4()),
        "eitmad.catalog.sync.project.v1",
        AuditTarget {
            kind: "catalog-sync".into(),
            identifiers: vec![],
        },
    );
    store
        .project_catalog_page(
            reader,
            &eitmad_contracts::transport::SchemaId::parse("eitmad.schema.catalog-public.v1")
                .unwrap(),
            Checkpoint::new(Uuid::new_v4()),
            &eitmad_storage::CatalogSyncProjection {
                private: &[],
                public: &records,
                normalize_name: eitmad_material::normalize_search,
            },
            &evidence,
        )
        .unwrap();
}

fn published_product_entry() -> eitmad_contracts::catalog_revision::CatalogEntry {
    use eitmad_contracts::catalog_revision::CatalogRevision;
    let manager_dir = TempDir::new().unwrap();
    let (manager_store, products, manager, _) = setup(&manager_dir);
    let saved = products
        .save(
            &mutation(manager.clone(), 20),
            &fixture(&products, &manager),
        )
        .unwrap();
    let manager_pricing = PricingService::new(
        manager_store.clone(),
        AuthorizationService::new(manager_store),
    )
    .with_confirmation(Arc::new(Confirmed::default()));
    let receipt = manager_pricing
        .publish(
            &mutation(manager, 30),
            &publish_input(target(&saved)),
            UnixMillis(i64::MAX),
        )
        .unwrap();
    public_entry(&CatalogRevision::Product(Box::new(saved)), &receipt).unwrap()
}

#[test]
fn sales_catalog_separate_client_search_pages_prices_and_withdrawal_are_public_only() {
    use eitmad_contracts::sales_catalog::{
        CheckSalesConfiguration, GetSalesCatalogItem, ListSalesCatalog,
    };
    let entry = published_product_entry();
    let reader_dir = TempDir::new().unwrap();
    let (reader_store, _, _, reader) = setup(&reader_dir);
    // No private Product or category is copied to this client.
    project_sales(&reader_store, &reader, vec![(1, 1, Some(entry.clone()))]);
    let service = PricingService::new(
        reader_store.clone(),
        AuthorizationService::new(reader_store.clone()),
    );
    let query = ListSalesCatalog {
        term: "مرتبه طبيه".into(),
        category: Some("مراتب".into()),
        after: None,
        limit: 1,
    };
    let page = service.sales_catalog(&reader, &query).unwrap();
    assert_eq!(page.items, vec![entry.clone()]);
    assert_eq!(page.next, None);
    assert!(!page.server_available);
    let details = service
        .sales_catalog_item(
            &reader,
            &GetSalesCatalogItem {
                target: entry.price.target.clone(),
            },
        )
        .unwrap();
    assert_eq!(details.variants, vec![entry.clone()]);
    let mut input = CheckSalesConfiguration {
        selection: PriceSelection {
            target: entry.price.target.clone(),
            price_revision: 1,
            color_id: None,
            handle_id: None,
            quantity: 2,
        },
        dimensions: None,
    };
    let checked = service.sales_configuration(&reader, &input).unwrap();
    assert_eq!(checked.price.unit_price_yer, 70000);
    assert_eq!(checked.price.total_yer, 140_000);
    for payload in [
        serde_json::to_string(&page).unwrap(),
        serde_json::to_string(&details).unwrap(),
        serde_json::to_string(&checked).unwrap(),
    ] {
        for forbidden in ["cost", "margin", "parts", "notes", "55000", "ملاحظة داخلية"]
        {
            assert!(!payload.contains(forbidden), "{forbidden}");
        }
    }
    assert_eq!(
        service.sales_catalog(&actor(777, 50), &query),
        Err(PricingError::Denied)
    );
    let mut foreign = input.clone();
    if let PriceTarget::Product(r) = &mut foreign.selection.target {
        r.scope = actor(1, 51).scope;
    }
    assert_eq!(
        service.sales_configuration(&reader, &foreign),
        Err(PricingError::Reference)
    );
    let mut changed = entry.clone();
    changed.price.revision = 2;
    changed.price.selling_price_yer = 71000;
    project_sales(&reader_store, &reader, vec![(1, 2, Some(changed.clone()))]);
    assert!(matches!(
        service.sales_configuration(&reader, &input),
        Err(PricingError::Conflict {
            actual: Some(2),
            ..
        })
    ));
    input.selection.price_revision = 2;
    if let PriceTarget::Product(r) = &mut changed.price.target {
        r.revision += 1;
    }
    changed.price.revision = 3;
    project_sales(&reader_store, &reader, vec![(1, 3, Some(changed))]);
    assert_eq!(
        service.sales_configuration(&reader, &input),
        Err(PricingError::Reference)
    );
    project_sales(&reader_store, &reader, vec![(1, 4, None)]);
    assert!(
        service
            .sales_catalog(&reader, &query)
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        service.sales_configuration(&reader, &input),
        Err(PricingError::Reference)
    );
}

#[test]
fn sales_catalog_sparse_pages_cross_storage_batches_without_gaps() {
    use eitmad_contracts::sales_catalog::ListSalesCatalog;
    let entry = published_product_entry();
    let reader_dir = TempDir::new().unwrap();
    let (reader_store, _, _, reader) = setup(&reader_dir);
    let service = PricingService::new(
        reader_store.clone(),
        AuthorizationService::new(reader_store.clone()),
    );
    // Sparse matches cross multiple storage batches without dropping or repeating a row.
    let records = (2..=206u128)
        .map(|id| {
            let mut e = entry.clone();
            if let PriceTarget::Product(r) = &mut e.price.target {
                r.product_id = eitmad_contracts::product::ProductId::new(Uuid::from_u128(id));
            }
            e.name = if id % 50 == 0 {
                "منتج مطابق"
            } else {
                "منتج آخر"
            }
            .into();
            (id, 1, Some(e))
        })
        .collect();
    project_sales(&reader_store, &reader, records);
    let mut query = ListSalesCatalog {
        term: "مطابق".into(),
        category: None,
        after: None,
        limit: 2,
    };
    let first = service.sales_catalog(&reader, &query).unwrap();
    assert_eq!(first.items.len(), 2);
    assert_eq!(first.next, Some(Uuid::from_u128(100)));
    query.after = first.next;
    let second = service.sales_catalog(&reader, &query).unwrap();
    assert_eq!(second.items.len(), 2);
    assert_eq!(second.next, None);
    query.limit = 101;
    assert_eq!(
        service.sales_catalog(&reader, &query),
        Err(PricingError::Invalid)
    );
}

fn published_furniture_entry() -> eitmad_contracts::catalog_revision::CatalogEntry {
    use eitmad_contracts::{
        catalog_revision::CatalogRevision,
        furniture::{FurnitureCustomization, FurnitureDimensions},
    };
    let manager_dir = TempDir::new().unwrap();
    let (store, _, manager, _) = setup(&manager_dir);
    let furniture_service =
        FurnitureService::new(store.clone(), AuthorizationService::new(store.clone()));
    let mut definition = furniture_fixture(&store, &furniture_service, &manager);
    let d = definition.variants[0].dimensions.clone();
    definition.variants[0].customization = Some(FurnitureCustomization {
        minimum: d.clone(),
        maximum: FurnitureDimensions {
            width_mm: d.width_mm + 100,
            height_mm: d.height_mm,
            depth_mm: d.depth_mm,
        },
    });
    let saved = furniture_service
        .save(&mutation(manager.clone(), 24), &definition)
        .unwrap();
    let target = PriceTarget::Furniture(FurnitureReference {
        scope: manager.scope.clone(),
        furniture_id: saved.id,
        variant_id: saved.variants[0].id,
        revision: saved.revision,
        schema_version: 1,
    });
    let manager_pricing = PricingService::new(store.clone(), AuthorizationService::new(store))
        .with_confirmation(Arc::new(Confirmed::default()));
    let receipt = manager_pricing
        .publish(
            &mutation(manager, 30),
            &PublishPrice {
                target,
                expected_revision: None,
                selling_price_yer: 250_000,
                confirm_below_cost: false,
            },
            UnixMillis(i64::MAX),
        )
        .unwrap();
    public_entry(&CatalogRevision::Furniture(Box::new(saved)), &receipt).unwrap()
}

#[test]
fn sales_catalog_furniture_checks_dimensions_compatible_options_quantity_and_overflow() {
    use eitmad_contracts::sales_catalog::CheckSalesConfiguration;
    let entry = published_furniture_entry();
    let d = entry.dimensions.clone().unwrap();
    let reader_dir = TempDir::new().unwrap();
    let (reader_store, _, _, reader) = setup(&reader_dir);
    project_sales(&reader_store, &reader, vec![(1, 1, Some(entry.clone()))]);
    let service = PricingService::new(
        reader_store.clone(),
        AuthorizationService::new(reader_store.clone()),
    );
    let mut input = CheckSalesConfiguration {
        selection: PriceSelection {
            target: entry.price.target.clone(),
            price_revision: 1,
            color_id: entry.colors.first().map(|o| o.id),
            handle_id: entry.handles.first().map(|o| o.id),
            quantity: 2,
        },
        dimensions: Some(d.clone()),
    };
    let accepted = service.sales_configuration(&reader, &input).unwrap();
    assert_eq!(accepted.dimensions, Some(d.clone()));
    input.dimensions.as_mut().unwrap().width_mm += 100;
    assert!(service.sales_configuration(&reader, &input).is_ok());
    input.dimensions.as_mut().unwrap().width_mm += 1;
    assert_eq!(
        service.sales_configuration(&reader, &input),
        Err(PricingError::Invalid)
    );
    input.dimensions = Some(d);
    input.selection.color_id = Some(Uuid::new_v4());
    assert_eq!(
        service.sales_configuration(&reader, &input),
        Err(PricingError::Reference)
    );
    input.selection.color_id = None;
    assert_eq!(
        service.sales_configuration(&reader, &input),
        Err(PricingError::Reference)
    );
    input.selection.color_id = entry.colors.first().map(|o| o.id);
    input.selection.quantity = 0;
    assert_eq!(
        service.sales_configuration(&reader, &input),
        Err(PricingError::Invalid)
    );
    input.selection.quantity = 2;
    let mut fixed = entry.clone();
    fixed.customization = None;
    fixed.price.revision = 2;
    project_sales(&reader_store, &reader, vec![(1, 2, Some(fixed.clone()))]);
    input.selection.price_revision = 2;
    input.dimensions.as_mut().unwrap().width_mm += 1;
    assert_eq!(
        service.sales_configuration(&reader, &input),
        Err(PricingError::Invalid)
    );
    input.dimensions = entry.dimensions.clone();
    fixed.price.revision = 3;
    fixed.price.selling_price_yer = i64::MAX;
    project_sales(&reader_store, &reader, vec![(1, 3, Some(fixed))]);
    input.selection.price_revision = 3;
    assert_eq!(
        service.sales_configuration(&reader, &input),
        Err(PricingError::Invalid)
    );
}
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
        image: None,
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

#[derive(Default)]
struct Confirmed {
    catalog: std::sync::Mutex<
        std::collections::BTreeMap<
            (&'static str, Uuid, u64),
            eitmad_contracts::catalog_revision::CatalogRevision,
        >,
    >,
    deny_status: std::sync::atomic::AtomicBool,
    records: std::sync::Mutex<Vec<PublishedPrice>>,
    receipts: std::sync::Mutex<Vec<(ConfirmPrice, PublishedPrice)>>,
}
impl PriceConfirmation for Confirmed {
    fn synchronize_catalog(
        &self,
        _: &AuthorizationContext,
        input: &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions,
        _: UnixMillis,
    ) -> Result<(), PricingError> {
        let mut catalog = self.catalog.lock().unwrap();
        let mut staged = catalog.clone();
        for record in &input.records {
            let (kind, id, revision, scope) = record.identity();
            if scope != &input.scope {
                return Err(PricingError::Denied);
            }
            if let Some(existing) = staged.get(&(kind, id, revision)) {
                if existing != record {
                    return Err(PricingError::Reference);
                }
                continue;
            }
            validate_catalog_revision(record, &staged)?;
            staged.insert((kind, id, revision), record.clone());
            if staged
                .get(&(kind, id, 0))
                .is_none_or(|v| v.identity().2 < revision)
            {
                staged.insert((kind, id, 0), record.clone());
            }
        }
        *catalog = staged;
        Ok(())
    }
    fn status(
        &self,
        _: &AuthorizationContext,
        request: &ConfirmPrice,
        _: UnixMillis,
    ) -> Result<Option<PublishedPrice>, PricingError> {
        if self.deny_status.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(PricingError::Denied);
        }
        Ok(self
            .receipts
            .lock()
            .unwrap()
            .iter()
            .find(|(intent, _)| intent == request)
            .map(|(_, price)| price.clone()))
    }

    fn read(
        &self,
        _: &AuthorizationContext,
        _: &ReadPublishedPrices,
        _: UnixMillis,
    ) -> Result<PublishedPricePage, PricingError> {
        Ok(PublishedPricePage {
            items: self.records.lock().unwrap().clone(),
            next: None,
        })
    }
    fn confirm(
        &self,
        _: &AuthorizationContext,
        input: &ConfirmPrice,
        _: UnixMillis,
    ) -> Result<PublishedPrice, PricingError> {
        let mut receipts = self.receipts.lock().unwrap();
        if let Some((request, result)) = receipts
            .iter()
            .find(|(r, _)| r.idempotency_key == input.idempotency_key)
        {
            return if request == input {
                Ok(result.clone())
            } else {
                Err(PricingError::Invalid)
            };
        }
        let mut records = self.records.lock().unwrap();
        let catalog = self.catalog.lock().unwrap();
        let (kind, id, _) = input.command.target.identity();
        let definition = catalog.get(&(kind, id, 0)).ok_or(PricingError::Reference)?;
        validate_server_proposal(input, definition)?;
        let actual = records
            .iter()
            .rev()
            .find(|p| p.target.identity() == input.command.target.identity())
            .map(|p| p.revision);
        if actual != input.command.expected_revision {
            return Err(PricingError::Conflict {
                expected: input.command.expected_revision,
                actual,
            });
        }
        let value = PublishedPrice {
            target: input.command.target.clone(),
            currency: "YER".into(),
            selling_price_yer: input.command.selling_price_yer,
            colors: input.colors.clone(),
            handles: input.handles.clone(),
            revision: actual.unwrap_or(0) + 1,
            confirmed_at: UnixMillis(1000),
        };
        records.push(value.clone());
        receipts.push((input.clone(), value.clone()));
        Ok(value)
    }
}
fn target(product: &Product) -> PriceTarget {
    PriceTarget::Product(ProductReference {
        scope: product.scope.clone(),
        product_id: product.id,
        variant_id: product.variants[0].id,
        revision: product.revision,
        schema_version: 1,
    })
}
fn prices() -> ListPrices {
    ListPrices {
        term: String::new(),
        after: None,
        limit: 100,
    }
}
fn publish_input(target: PriceTarget) -> PublishPrice {
    PublishPrice {
        target,
        expected_revision: None,
        selling_price_yer: 70_000,
        confirm_below_cost: false,
    }
}

#[test]
fn approved_discount_examples_round_once_and_enforce_the_exact_threshold() {
    for (lines, bps, subtotal, discount_yer, total, approval) in [
        (vec![200_000, 85_000], 500, 285_000, 14_250, 270_750, false),
        (vec![100_000], 501, 100_000, 5_010, 94_990, true),
        (vec![1, 1], 2500, 2, 1, 1, true),
        (vec![1010], 500, 1010, 51, 959, false),
        (vec![10_001], 500, 10_001, 500, 9_501, false),
        (vec![i64::MAX], 10_000, i64::MAX, i64::MAX, 0, true),
    ] {
        assert_eq!(
            discount(&CalculateDiscount {
                line_totals_yer: lines,
                discount_basis_points: bps
            })
            .unwrap(),
            DiscountTotal {
                subtotal_yer: subtotal,
                discount_yer,
                total_yer: total,
                approval_required: approval
            }
        );
    }
    for input in [
        CalculateDiscount {
            line_totals_yer: vec![-1],
            discount_basis_points: 500,
        },
        CalculateDiscount {
            line_totals_yer: vec![i64::MAX, 1],
            discount_basis_points: 0,
        },
        CalculateDiscount {
            line_totals_yer: vec![100],
            discount_basis_points: 10001,
        },
    ] {
        assert_eq!(discount(&input), Err(PricingError::Invalid));
    }
}

#[test]
fn manager_publication_is_durable_audited_idempotent_and_stale_updates_conflict() {
    let dir = TempDir::new().unwrap();
    let (store, products, manager, receptionist) = setup(&dir);
    let saved = products
        .save(
            &mutation(manager.clone(), 20),
            &fixture(&products, &manager),
        )
        .unwrap();
    let server = Arc::new(Confirmed::default());
    let service = PricingService::new(store.clone(), AuthorizationService::new(store.clone()))
        .with_confirmation(server);
    let input = publish_input(target(&saved));
    let mut fractional = serde_json::to_value(&input).unwrap();
    fractional["sellingPriceYer"] = serde_json::json!(1.5);
    assert!(serde_json::from_value::<PublishPrice>(fractional).is_err());
    let published = service
        .publish(&mutation(manager.clone(), 30), &input, UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(published.revision, 1);
    assert_eq!(published.selling_price_yer, 70_000);
    assert_eq!(
        service
            .publish(&mutation(manager.clone(), 30), &input, UnixMillis(i64::MAX))
            .unwrap(),
        published
    );
    let reopened = PricingService::new(
        AuthorityStore::open(dir.path()).unwrap(),
        AuthorizationService::new(store.clone()),
    );
    let row = &reopened.list(&manager, &prices()).unwrap().items[0];
    assert_eq!(row.cost_yer, Some(55_000));
    assert_eq!(row.margin_yer, Some(15_000));
    assert_eq!(
        reopened.list(&receptionist, &prices()).unwrap().items.len(),
        1
    );
    assert_eq!(
        service.publish(&mutation(manager.clone(), 31), &input, UnixMillis(i64::MAX)),
        Err(PricingError::Conflict {
            expected: None,
            actual: Some(1)
        })
    );
    let db = rusqlite::Connection::open(dir.path().join("eitmad.sqlite3")).unwrap();
    let audits:u32=db.query_row("SELECT COUNT(*) FROM mutation_audit WHERE operation='eitmad.pricing.publish.v1' AND resulting_revision=1",[],|r|r.get(0)).unwrap();
    assert_eq!(audits, 1);
    assert!(
        db.execute("UPDATE pricing_revisions SET revision=2", [])
            .is_err()
    );
    assert!(db.execute("DELETE FROM pricing_revisions", []).is_err());
}

#[test]
fn receptionist_payload_and_direct_queries_never_include_cost_or_margin() {
    let dir = TempDir::new().unwrap();
    let (store, products, manager, receptionist) = setup(&dir);
    let saved = products
        .save(
            &mutation(manager.clone(), 20),
            &fixture(&products, &manager),
        )
        .unwrap();
    let service = PricingService::new(store.clone(), AuthorizationService::new(store))
        .with_confirmation(Arc::new(Confirmed::default()));
    assert!(
        service
            .list(&receptionist, &prices())
            .unwrap()
            .items
            .is_empty()
    );
    let input = publish_input(target(&saved));
    service
        .publish(&mutation(manager.clone(), 30), &input, UnixMillis(i64::MAX))
        .unwrap();
    let page = service.list(&receptionist, &prices()).unwrap();
    let json = serde_json::to_string(&page).unwrap();
    for field in [
        "costYer",
        "marginYer",
        "purchaseCostYer",
        "partsCostYer",
        "notes",
        "parts",
        "55000",
        "ملاحظة داخلية",
    ] {
        assert!(!json.contains(field), "leaked {field}");
    }
    assert!(!page.can_manage);
    assert!(!page.can_read_costs);
    assert_eq!(
        service.review(
            &receptionist,
            &ReviewPrice {
                target: input.target.clone(),
                selling_price_yer: 70_000
            }
        ),
        Err(PricingError::Denied)
    );
    assert_eq!(
        service.publish(
            &mutation(receptionist.clone(), 31),
            &input,
            UnixMillis(i64::MAX)
        ),
        Err(PricingError::Denied)
    );
    let selection = service
        .selection(
            &receptionist,
            &PriceSelection {
                target: input.target,
                price_revision: 1,
                color_id: None,
                handle_id: None,
                quantity: 3,
            },
        )
        .unwrap();
    assert_eq!(selection.total_yer, 210_000);
    let json = serde_json::to_string(&selection).unwrap();
    assert!(!json.contains("cost"));
    assert!(!json.contains("margin"));
    assert_eq!(
        service.list(&actor(100, 60), &prices()),
        Err(PricingError::Denied)
    );
}

#[test]
fn below_cost_requires_explicit_confirmation_and_offline_never_publishes() {
    let dir = TempDir::new().unwrap();
    let (store, products, manager, receptionist) = setup(&dir);
    let saved = products
        .save(
            &mutation(manager.clone(), 20),
            &fixture(&products, &manager),
        )
        .unwrap();
    let service = PricingService::new(store.clone(), AuthorizationService::new(store.clone()));
    let mut input = publish_input(target(&saved));
    input.selling_price_yer = 50_000;
    assert_eq!(
        service.publish(&mutation(manager.clone(), 30), &input, UnixMillis(i64::MAX)),
        Err(PricingError::BelowCost)
    );
    input.confirm_below_cost = true;
    assert_eq!(
        service.publish(&mutation(manager.clone(), 31), &input, UnixMillis(i64::MAX)),
        Err(PricingError::Unconfirmed)
    );
    assert!(
        service
            .list(&receptionist, &prices())
            .unwrap()
            .items
            .is_empty()
    );
    let reopened = PricingService::new(
        AuthorityStore::open(dir.path()).unwrap(),
        AuthorizationService::new(store),
    )
    .with_confirmation(Arc::new(Confirmed::default()));
    // A restarted shell can use a new key; Rust retries the original durable server intent.
    let price = reopened
        .publish(&mutation(manager.clone(), 32), &input, UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(price.revision, 1);
    let db = rusqlite::Connection::open(dir.path().join("eitmad.sqlite3")).unwrap();
    let confirmation:String=db.query_row("SELECT changed_identifiers FROM mutation_audit WHERE resulting_revision=1 AND operation='eitmad.pricing.publish.v1'",[],|r|r.get(0)).unwrap();
    assert!(confirmation.contains("below-cost-confirmed"));
}

#[test]
fn cost_changes_preserve_price_and_old_price_snapshots_but_require_new_publication() {
    let dir = TempDir::new().unwrap();
    let (store, products, manager, receptionist) = setup(&dir);
    let mut draft = fixture(&products, &manager);
    let saved = products
        .save(&mutation(manager.clone(), 20), &draft)
        .unwrap();
    let service = PricingService::new(store.clone(), AuthorizationService::new(store))
        .with_confirmation(Arc::new(Confirmed::default()));
    let input = publish_input(target(&saved));
    let old = service
        .publish(&mutation(manager.clone(), 30), &input, UnixMillis(i64::MAX))
        .unwrap();
    draft.id = Some(saved.id);
    draft.expected_revision = Some(1);
    draft.variants[0].purchase_cost_yer = 60_000;
    let updated = products
        .save(&mutation(manager.clone(), 40), &draft)
        .unwrap();
    let rows = service.list(&manager, &prices()).unwrap();
    let row = &rows.items[0];
    assert!(row.publication_required);
    assert_eq!(row.published.as_ref().unwrap().selling_price_yer, 70_000);
    assert_eq!(row.margin_yer, Some(10_000));
    assert!(
        service
            .list(&receptionist, &prices())
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        service.selection(
            &receptionist,
            &PriceSelection {
                target: input.target,
                price_revision: 1,
                color_id: None,
                handle_id: None,
                quantity: 1
            }
        ),
        Err(PricingError::Reference)
    );
    let next = PublishPrice {
        target: target(&updated),
        expected_revision: Some(1),
        selling_price_yer: 75_000,
        confirm_below_cost: false,
    };
    service
        .publish(&mutation(manager, 41), &next, UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(old.selling_price_yer, 70_000);
    assert_eq!(old.revision, 1);
}

use eitmad_contracts::{
    furniture::*,
    material::{
        Material, MaterialQuantity, MaterialUnit, SaveMaterial, SaveMaterialCategory,
        SaveMaterialUnit, UnitDimension,
    },
    part::{PartUsage, SavePart, SavePartCategory},
};
use eitmad_furniture::FurnitureService;
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

fn furniture_fixture(
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

#[test]
fn manufactured_cost_options_and_selling_prices_use_part_revisions_and_exact_whole_rials() {
    let dir = TempDir::new().unwrap();
    let (store, _, manager, receptionist) = setup(&dir);
    let furniture = FurnitureService::new(store.clone(), AuthorizationService::new(store.clone()));
    let draft = furniture_fixture(&store, &furniture, &manager);
    let saved = furniture
        .save(&mutation(manager.clone(), 22), &draft)
        .unwrap();
    let pricing = PricingService::new(store.clone(), AuthorizationService::new(store.clone()))
        .with_confirmation(Arc::new(Confirmed::default()));
    let target = PriceTarget::Furniture(FurnitureReference {
        scope: saved.scope.clone(),
        furniture_id: saved.id,
        variant_id: saved.variants[0].id,
        revision: saved.revision,
        schema_version: 1,
    });
    let review = pricing
        .review(
            &manager,
            &ReviewPrice {
                target: target.clone(),
                selling_price_yer: 25_000,
            },
        )
        .unwrap();
    assert_eq!(review.cost_yer, 18_900);
    assert_eq!(review.margin_yer, 6_100);
    let mut input = publish_input(target.clone());
    input.selling_price_yer = 25_000;
    pricing
        .publish(&mutation(manager.clone(), 30), &input, UnixMillis(i64::MAX))
        .unwrap();
    let mut selection = PriceSelection {
        target: target.clone(),
        price_revision: 1,
        color_id: Some(Uuid::from_u128(901)),
        handle_id: Some(Uuid::from_u128(902)),
        quantity: 2,
    };
    let price = pricing.selection(&receptionist, &selection).unwrap();
    assert_eq!(price.unit_price_yer, 26_500);
    assert_eq!(price.total_yer, 53_000);
    let json = serde_json::to_string(&price).unwrap();
    for forbidden in ["parts", "cost", "margin", "notes"] {
        assert!(!json.contains(forbidden));
    }
    selection.handle_id = Some(Uuid::from_u128(999));
    assert_eq!(
        pricing.selection(&receptionist, &selection),
        Err(PricingError::Reference)
    );
    selection.handle_id = None;
    selection.quantity = 0;
    assert_eq!(
        pricing.selection(&receptionist, &selection),
        Err(PricingError::Reference)
    );
    selection.quantity = 2;
    input.selling_price_yer = i64::MAX;
    input.expected_revision = Some(1);
    assert_eq!(
        pricing.publish(&mutation(manager.clone(), 31), &input, UnixMillis(i64::MAX)),
        Err(PricingError::Invalid)
    );
    input.target = target;
    input.selling_price_yer = i64::MAX - 1500;
    pricing
        .publish(&mutation(manager, 32), &input, UnixMillis(i64::MAX))
        .unwrap();
    selection.price_revision = 2;
    assert_eq!(
        pricing.selection(&receptionist, &selection),
        Err(PricingError::Invalid)
    );
}

#[test]
fn server_catalog_rejects_forged_product_cost_and_changed_immutable_revision() {
    use eitmad_contracts::catalog_revision::{CatalogRevision, SynchronizeCatalogRevisions};
    let dir = TempDir::new().unwrap();
    let (store, products, manager, _) = setup(&dir);
    let product = products
        .save(
            &mutation(manager.clone(), 20),
            &fixture(&products, &manager),
        )
        .unwrap();
    let server = Arc::new(Confirmed::default());
    let service = PricingService::new(store.clone(), AuthorizationService::new(store))
        .with_confirmation(server.clone());
    let command = publish_input(target(&product));
    service
        .publish(
            &mutation(manager.clone(), 30),
            &command,
            UnixMillis(i64::MAX),
        )
        .unwrap();
    let mut proposal = server.receipts.lock().unwrap()[0].0.clone();
    proposal.idempotency_key = IdempotencyKey::new(Uuid::from_u128(31));
    proposal.command.expected_revision = Some(1);
    proposal.command.selling_price_yer = 50_000;
    proposal.cost_yer = 0;
    assert_eq!(
        server.confirm(&manager, &proposal, UnixMillis(i64::MAX)),
        Err(PricingError::BelowCost)
    );
    proposal.command.confirm_below_cost = true;
    assert_eq!(
        server.confirm(&manager, &proposal, UnixMillis(i64::MAX)),
        Err(PricingError::Reference)
    );
    proposal.cost_yer = 55_000;
    assert_eq!(
        server
            .confirm(&manager, &proposal, UnixMillis(i64::MAX))
            .unwrap()
            .revision,
        2
    );
    let mut altered = product;
    altered.variants[0].purchase_cost_yer = Some(0);
    assert_eq!(
        server.synchronize_catalog(
            &manager,
            &SynchronizeCatalogRevisions {
                scope: manager.scope.clone(),
                records: vec![CatalogRevision::Product(Box::new(altered))],
            },
            UnixMillis(i64::MAX)
        ),
        Err(PricingError::Reference)
    );
    assert_eq!(server.receipts.lock().unwrap().len(), 2);
}

#[test]
fn server_catalog_recalculates_part_and_furniture_costs_and_rejects_foreign_dependencies() {
    use eitmad_contracts::catalog_revision::CatalogRevision;
    let dir = TempDir::new().unwrap();
    let (store, _, manager, _) = setup(&dir);
    let furniture = FurnitureService::new(store.clone(), AuthorizationService::new(store.clone()));
    let draft = furniture_fixture(&store, &furniture, &manager);
    let saved = furniture
        .save(&mutation(manager.clone(), 22), &draft)
        .unwrap();
    let service = PricingService::new(store.clone(), AuthorizationService::new(store));
    let target = PriceTarget::Furniture(FurnitureReference {
        scope: saved.scope.clone(),
        furniture_id: saved.id,
        variant_id: saved.variants[0].id,
        revision: saved.revision,
        schema_version: 1,
    });
    let records = service.catalog_revisions(&target).unwrap();
    let mut known = std::collections::BTreeMap::new();
    for record in records {
        validate_catalog_revision(&record, &known).unwrap();
        if let CatalogRevision::Part(p) = &record {
            assert_eq!(p.cost.total_cost_yer, 9450);
            let mut altered = p.clone();
            altered.cost.total_cost_yer = 0;
            assert_eq!(
                validate_catalog_revision(&CatalogRevision::Part(altered), &known),
                Err(PricingError::Invalid)
            );
            let mut altered = p.clone();
            altered.cost.rows[0].material.current_cost_yer = 0;
            assert_eq!(
                validate_catalog_revision(&CatalogRevision::Part(altered), &known),
                Err(PricingError::Reference)
            );
            let mut altered = p.clone();
            altered.cost.rows[0].cost_yer -= 1;
            assert_eq!(
                validate_catalog_revision(&CatalogRevision::Part(altered), &known),
                Err(PricingError::Invalid)
            );
        }
        if let CatalogRevision::Furniture(f) = &record {
            assert_eq!(publication_basis(&record, &target).unwrap().0, 18_900);
            let mut altered = f.clone();
            altered.parts_cost_yer = 0;
            assert_eq!(
                validate_catalog_revision(&CatalogRevision::Furniture(altered), &known),
                Err(PricingError::Invalid)
            );
            let mut altered = f.clone();
            altered.parts[0].reference.scope.id = ScopeId::new(Uuid::from_u128(999));
            assert_eq!(
                validate_catalog_revision(&CatalogRevision::Furniture(altered), &known),
                Err(PricingError::Reference)
            );
            let mut stale = target.clone();
            if let PriceTarget::Furniture(r) = &mut stale {
                r.revision += 1;
            }
            assert_eq!(
                publication_basis(&record, &stale),
                Err(PricingError::Reference)
            );
        }
        let (kind, id, revision, _) = record.identity();
        known.insert((kind, id, revision), record.clone());
        known.insert((kind, id, 0), record);
    }
}

#[test]
fn server_catalog_requires_exact_furniture_part_references() {
    use eitmad_contracts::catalog_revision::CatalogRevision;
    use eitmad_contracts::part::PartId;
    let dir = TempDir::new().unwrap();
    let (store, _, manager, _) = setup(&dir);
    let furniture = FurnitureService::new(store.clone(), AuthorizationService::new(store.clone()));
    let draft = furniture_fixture(&store, &furniture, &manager);
    let saved = furniture
        .save(&mutation(manager.clone(), 22), &draft)
        .unwrap();
    let service = PricingService::new(store.clone(), AuthorizationService::new(store));
    let target = PriceTarget::Furniture(FurnitureReference {
        scope: saved.scope.clone(),
        furniture_id: saved.id,
        variant_id: saved.variants[0].id,
        revision: saved.revision,
        schema_version: 1,
    });
    let mut known = std::collections::BTreeMap::new();
    for record in service.catalog_revisions(&target).unwrap() {
        validate_catalog_revision(&record, &known).unwrap();
        let (kind, id, revision, _) = record.identity();
        known.insert((kind, id, revision), record.clone());
        known.insert((kind, id, 0), record);
    }
    let mut altered = saved.clone();
    altered.parts[0].reference.revision = 0;
    let record = CatalogRevision::Furniture(Box::new(altered));
    assert_eq!(
        validate_catalog_revision(&record, &known),
        Err(PricingError::Reference)
    );
    assert_eq!(catalog_dependencies(&record), Err(PricingError::Reference));

    let reference = &saved.parts[0].reference;
    let key = ("part", reference.part_id.value(), reference.revision);
    let CatalogRevision::Part(part) = known.get(&key).unwrap() else {
        panic!("fixture must contain the referenced Part");
    };
    let part = part.clone();
    for mismatch in 0..4 {
        let mut altered = part.clone();
        match mismatch {
            0 => altered.id = PartId::new(Uuid::from_u128(999)),
            1 => altered.revision += 1,
            2 => altered.scope.id = ScopeId::new(Uuid::from_u128(999)),
            _ => altered.composition.revision += 1,
        }
        known.insert(key, CatalogRevision::Part(altered));
        assert_eq!(
            validate_catalog_revision(&CatalogRevision::Furniture(Box::new(saved.clone())), &known),
            Err(PricingError::Reference)
        );
    }
}

#[test]
fn bounded_variant_pages_and_cache_first_retry_complete_without_duplicate_revisions() {
    let dir = TempDir::new().unwrap();
    let (store, products, manager, _) = setup(&dir);
    let saved = products
        .save(
            &mutation(manager.clone(), 20),
            &fixture(&products, &manager),
        )
        .unwrap();
    let server = Arc::new(Confirmed::default());
    let service = PricingService::new(store.clone(), AuthorizationService::new(store.clone()))
        .with_confirmation(server.clone());
    let first = service
        .list(
            &manager,
            &ListPrices {
                limit: 1,
                ..prices()
            },
        )
        .unwrap();
    assert_eq!(first.items.len(), 1);
    let second = service
        .list(
            &manager,
            &ListPrices {
                after: first.next,
                limit: 1,
                ..prices()
            },
        )
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert_ne!(first.items[0].target, second.items[0].target);
    assert!(second.next.is_none());
    let input = publish_input(target(&saved));
    // Simulate a lost response after the server commit, then a cache refresh before retry.
    let hash: [u8; 32] =
        Sha256::digest(serde_json::to_vec(&(manager.identity.principal_id, &input)).unwrap())
            .into();
    let context = mutation(manager.clone(), 30);
    let prepared = service.prepare(&context, &input, &hash).unwrap();
    server
        .synchronize_catalog(
            &manager,
            &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions {
                scope: manager.scope.clone(),
                records: service.catalog_revisions(&input.target).unwrap(),
            },
            UnixMillis(i64::MAX),
        )
        .unwrap();
    let confirmed = server
        .confirm(&manager, &prepared, UnixMillis(i64::MAX))
        .unwrap();
    server
        .deny_status
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        service.publish(&context, &input, UnixMillis(i64::MAX)),
        Err(PricingError::Denied)
    );
    let retained: u32 = rusqlite::Connection::open(store.path())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM pricing_intents", [], |r| r.get(0))
        .unwrap();
    assert_eq!(retained, 1);
    server
        .deny_status
        .store(false, std::sync::atomic::Ordering::SeqCst);
    service
        .refresh(&mutation(manager.clone(), 31), UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(
        service
            .publish(&context, &input, UnixMillis(i64::MAX))
            .unwrap(),
        confirmed
    );
    assert_eq!(
        service
            .publish(&context, &input, UnixMillis(i64::MAX))
            .unwrap(),
        confirmed
    );
    let db = rusqlite::Connection::open(store.path()).unwrap();
    let count: u32 = db
        .query_row("SELECT COUNT(*) FROM pricing_revisions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    let audits: u32 = db.query_row("SELECT COUNT(*) FROM mutation_audit WHERE operation='eitmad.pricing.publish.v1' AND resulting_revision=1", [], |r| r.get(0)).unwrap();
    assert_eq!(audits, 1);
}

#[test]
fn variant_pages_fill_across_definitions_and_filtered_batches_without_gaps() {
    let dir = TempDir::new().unwrap();
    let (store, products, manager, _) = setup(&dir);
    let mut input = fixture(&products, &manager);
    let mut expected = Vec::new();
    for index in 0..12 {
        input.name = if index < 9 {
            "مصباح"
        } else {
            "مرتبة"
        }
        .into();
        for (variant_index, variant) in input.variants.iter_mut().enumerate() {
            variant.id = ProductVariantId::new(Uuid::from_u128(
                2000 + index * 2 + u128::try_from(variant_index).unwrap(),
            ));
        }
        let saved = products
            .save(&mutation(manager.clone(), 20 + index), &input)
            .unwrap();
        if index >= 9 {
            for variant in &saved.variants {
                expected.push(PriceTarget::Product(ProductReference {
                    variant_id: variant.id,
                    ..match target(&saved) {
                        PriceTarget::Product(reference) => reference,
                        PriceTarget::Furniture(_) => unreachable!(),
                    }
                }));
            }
        }
    }
    expected.sort_by_key(PriceTarget::identity);
    let service = PricingService::new(store.clone(), AuthorizationService::new(store));
    let first = service.list(&manager, &prices()).unwrap();
    assert_eq!(first.items.len(), 24);
    assert!(first.next.is_none());

    let query = ListPrices {
        term: "مرتبة".into(),
        limit: 4,
        ..prices()
    };
    let first = service.list(&manager, &query).unwrap();
    assert_eq!(first.items.len(), 4);
    assert!(first.next.is_some());
    let second = service
        .list(
            &manager,
            &ListPrices {
                after: first.next,
                ..query
            },
        )
        .unwrap();
    assert_eq!(second.items.len(), 2);
    assert!(second.next.is_none());
    let actual: Vec<_> = first
        .items
        .into_iter()
        .chain(second.items)
        .map(|p| p.target)
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn original_confirmed_intent_survives_newer_price_refresh_before_retry() {
    let dir = TempDir::new().unwrap();
    let (store, products, manager, _) = setup(&dir);
    let saved = products
        .save(
            &mutation(manager.clone(), 20),
            &fixture(&products, &manager),
        )
        .unwrap();
    let server = Arc::new(Confirmed::default());
    let service = PricingService::new(store.clone(), AuthorizationService::new(store))
        .with_confirmation(server.clone());
    let input = publish_input(target(&saved));
    let context = mutation(manager.clone(), 30);
    let hash: [u8; 32] =
        Sha256::digest(serde_json::to_vec(&(manager.identity.principal_id, &input)).unwrap())
            .into();
    let prepared = service.prepare(&context, &input, &hash).unwrap();
    server
        .synchronize_catalog(
            &manager,
            &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions {
                scope: manager.scope.clone(),
                records: service.catalog_revisions(&input.target).unwrap(),
            },
            UnixMillis(i64::MAX),
        )
        .unwrap();
    let original = server
        .confirm(&manager, &prepared, UnixMillis(i64::MAX))
        .unwrap();
    let mut newer = prepared.clone();
    newer.idempotency_key = IdempotencyKey::new(Uuid::from_u128(31));
    newer.command.expected_revision = Some(1);
    newer.command.selling_price_yer = 75000;
    server
        .confirm(&manager, &newer, UnixMillis(i64::MAX))
        .unwrap();
    service
        .refresh(&mutation(manager.clone(), 32), UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(
        service
            .publish(&context, &input, UnixMillis(i64::MAX))
            .unwrap(),
        original
    );
    assert_eq!(
        service.list(&manager, &prices()).unwrap().items[0]
            .published
            .as_ref()
            .unwrap()
            .revision,
        2
    );
}

#[test]
fn receipt_audit_failure_retains_intent_and_withholds_price_until_atomic_retry() {
    let dir = TempDir::new().unwrap();
    let (store, products, manager, receptionist) = setup(&dir);
    let saved = products
        .save(
            &mutation(manager.clone(), 20),
            &fixture(&products, &manager),
        )
        .unwrap();
    let server = Arc::new(Confirmed::default());
    let service = PricingService::new(store.clone(), AuthorizationService::new(store.clone()))
        .with_confirmation(server.clone());
    let db = rusqlite::Connection::open(store.path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_price_audit BEFORE INSERT ON mutation_audit WHEN NEW.operation='eitmad.pricing.publish.v1' AND NEW.resulting_revision IS NOT NULL BEGIN SELECT RAISE(ABORT, 'synthetic audit failure'); END;").unwrap();
    let input = publish_input(target(&saved));
    let context = mutation(manager.clone(), 30);
    assert_eq!(
        service.publish(&context, &input, UnixMillis(i64::MAX)),
        Err(PricingError::Unconfirmed)
    );
    assert!(
        service
            .list(&receptionist, &prices())
            .unwrap()
            .items
            .is_empty()
    );
    let intents: u32 = db
        .query_row("SELECT COUNT(*) FROM pricing_intents", [], |r| r.get(0))
        .unwrap();
    assert_eq!(intents, 1);
    db.execute_batch("DROP TRIGGER fail_price_audit").unwrap();
    assert_eq!(
        service
            .publish(&context, &input, UnixMillis(i64::MAX))
            .unwrap()
            .revision,
        1
    );
    assert_eq!(server.records.lock().unwrap().len(), 1);
    assert_eq!(
        service.list(&receptionist, &prices()).unwrap().items.len(),
        1
    );
    let intents: u32 = db
        .query_row("SELECT COUNT(*) FROM pricing_intents", [], |r| r.get(0))
        .unwrap();
    assert_eq!(intents, 0);
}
