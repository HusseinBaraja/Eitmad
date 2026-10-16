use super::{Client, Result, checked, correlation, deadline, mutation};
use eitmad_authorization::AuthorizationService;
use eitmad_catalog_image::{CatalogImageService, CatalogImageTransfer};
use eitmad_contracts::{
    catalog_image::{CatalogImageKind, CatalogImageRef, ImportCatalogImage},
    commands::CreateCustomer,
    customer::{CustomerAddress, CustomerName, CustomerNotes, CustomerPhone},
    furniture::{
        FurnitureDimensions, FurniturePart, FurnitureState, FurnitureVariant, FurnitureVariantId,
        SaveFurniture, SaveFurnitureCategory,
    },
    material::{
        MaterialQuantity, SaveMaterial, SaveMaterialCategory, SaveMaterialUnit, UnitDimension,
    },
    order::{
        AcceptQuotation, AcceptanceMethod, ConfirmOrder, ConvertQuotation, OrderAction,
        OrderRecord, TransitionOrderWork,
    },
    part::{PartUsage, SavePart, SavePartCategory},
    pricing::{ListPrices, PriceSelection, PriceTarget, PublishPrice},
    product::{ProductVariantId, SaveProduct, SaveProductCategory, SaveProductVariant},
    quotation::{EvaluateQuotation, QuotationCustomerIntent, QuotationLineIntent},
    quotation_approval::{ConfirmDiscountApproval, DiscountApprovalAction},
    quotation_draft::CreateQuotationDraft,
    quotation_lifecycle::{ConfirmQuotation, IssueQuotation, ListQuotations, QuotationAction},
    sales_catalog::CheckSalesConfiguration,
    transport::IdempotencyKey,
};
use eitmad_orders::OrderServer;
use eitmad_pricing::{
    CatalogReplication, DiscountApprovalServer, PricingService, QuotationDraftService,
    QuotationServer,
};
use uuid::Uuid;

pub(super) fn run(manager: &Client, reception: &Client) -> Result<()> {
    let part = seed_part(manager)?;
    seed_furniture(manager, part)?;
    seed_product(manager)?;
    publish_catalog(manager, reception)?;
    seed_workflow(manager, reception)
}

fn seed_image(
    manager: &Client,
    kind: CatalogImageKind,
    file: &str,
    step: u128,
) -> Result<Box<CatalogImageRef>> {
    let images = CatalogImageService::new(
        manager.store.clone(),
        AuthorizationService::new(manager.store.clone()),
    )
    .with_transfer(manager.media.clone());
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/workflow_environment/photos")
        .join(file);
    let image = checked(
        images.import(
            &mutation(&manager.actor, step),
            &ImportCatalogImage {
                kind,
                source_path: path.to_string_lossy().into_owned(),
            },
            deadline(),
        ),
        "import showroom photo",
    )?;
    checked(images.retry_uploads(), "upload showroom photo")?;
    checked(
        manager.media.download(&manager.actor, &image, deadline()),
        "confirm showroom photo upload",
    )?;
    Ok(Box::new(image))
}

fn seed_part(manager: &Client) -> Result<eitmad_contracts::part::Part> {
    let auth = AuthorizationService::new(manager.store.clone());
    let materials = eitmad_material::MaterialService::new(manager.store.clone(), auth.clone());
    let category = checked(
        materials.save_category(
            &mutation(&manager.actor, 1),
            &SaveMaterialCategory {
                id: None,
                expected_revision: None,
                name: "ألواح خشبية".into(),
                archived: false,
            },
        ),
        "material category",
    )?;
    let unit = checked(
        materials.save_unit(
            &mutation(&manager.actor, 2),
            &SaveMaterialUnit {
                id: None,
                expected_revision: None,
                name: "قطعة".into(),
                symbol: "قطعة".into(),
                dimension: UnitDimension::Count,
                numerator: 1,
                denominator: 1,
                archived: false,
            },
        ),
        "material unit",
    )?;
    let material = checked(
        materials.save_material(
            &mutation(&manager.actor, 3),
            &SaveMaterial {
                id: None,
                expected_revision: None,
                name: "لوح خشب بقشرة الجوز".into(),
                category_id: category.id,
                unit_id: unit.id,
                current_cost_yer: 9_000,
                archived: false,
            },
        ),
        "material",
    )?;
    let parts = eitmad_part::PartService::new(manager.store.clone(), auth.clone());
    let category = checked(
        parts.save_category(
            &mutation(&manager.actor, 4),
            &SavePartCategory {
                id: None,
                expected_revision: None,
                name: "ألواح الخزائن".into(),
                archived: false,
            },
        ),
        "part category",
    )?;
    let part = checked(
        parts.save(
            &mutation(&manager.actor, 5),
            &SavePart {
                id: None,
                expected_revision: None,
                name: "لوح خزانة جوزي".into(),
                category_id: category.id,
                description: "مجموعة ألواح بقشرة الجوز لخزانة السكينة".into(),
                usages: vec![PartUsage {
                    material_id: material.id,
                    material_revision: material.revision,
                    unit_id: unit.id,
                    unit_revision: unit.revision,
                    quantity: checked(MaterialQuantity::parse("2".into()), "quantity")?,
                }],
                archived: false,
            },
        ),
        "part",
    )?;
    Ok(part)
}

fn seed_furniture(manager: &Client, part: eitmad_contracts::part::Part) -> Result<()> {
    let auth = AuthorizationService::new(manager.store.clone());
    let furniture = eitmad_furniture::FurnitureService::new(manager.store.clone(), auth.clone());
    let category = checked(
        furniture.save_category(
            &mutation(&manager.actor, 6),
            &SaveFurnitureCategory {
                id: None,
                expected_revision: None,
                name: "خزائن غرف النوم".into(),
                archived: false,
            },
        ),
        "furniture category",
    )?;
    checked(
        furniture.save(
            &mutation(&manager.actor, 7),
            &SaveFurniture {
                id: None,
                expected_revision: None,
                name: "خزانة السكينة ببابين".into(),
                category_id: category.id,
                image: Some(seed_image(manager, CatalogImageKind::Furniture, "walnut-wardrobe.png", 10)?),
                description: "خزانة بقشرة الجوز، بابان ومقبضان بلون نحاسي. العرض ١٢٠ سم، الارتفاع ٢٠٠ سم، العمق ٦٠ سم.".into(),
                notes: "عينة عرض اصطناعية لغرفة نوم أسرة السعدي؛ الصورة مولدة.".into(),
                parts: vec![FurniturePart {
                    reference: part.composition,
                    quantity: 3,
                }],
                variants: vec![FurnitureVariant {
                    id: FurnitureVariantId::new(Uuid::from_u128(1001)),
                    name: "جوزي — ١٢٠ × ٢٠٠ × ٦٠ سم".into(),
                    dimensions: FurnitureDimensions {
                        width_mm: 1200,
                        height_mm: 2000,
                        depth_mm: 600,
                    },
                    customization: None,
                    selling_price_yer: 185_000,
                    archived: false,
                    color_ids: vec![],
                    handle_ids: vec![],
                }],
                colors: vec![],
                handles: vec![],
                state: FurnitureState::Active,
                confirm_below_cost: false,
            },
        ),
        "furniture",
    )?;
    Ok(())
}

fn seed_product(manager: &Client) -> Result<eitmad_contracts::product::Product> {
    let auth = AuthorizationService::new(manager.store.clone());
    let products = eitmad_product::ProductService::new(manager.store.clone(), auth);
    let category = checked(
        products.save_category(
            &mutation(&manager.actor, 8),
            &SaveProductCategory {
                id: None,
                expected_revision: None,
                name: "مراتب غرف النوم".into(),
                archived: false,
            },
        ),
        "product category",
    )?;
    let product = checked(
        products.save(
            &mutation(&manager.actor, 9),
            &SaveProduct {
                id: None,
                expected_revision: None,
                name: "مرتبة الراحة المزدوجة".into(),
                category_id: category.id,
                image: Some(seed_image(manager, CatalogImageKind::Product, "double-mattress.png", 11)?),
                description: "مرتبة مزدوجة بسطح أبيض مبطن وجوانب بيج. العرض ١٦٠ سم، الطول ٢٠٠ سم، السماكة ٢٥ سم.".into(),
                notes: "عينة عرض اصطناعية لغرفة نوم أسرة السعدي؛ الصورة مولدة.".into(),
                variants: vec![SaveProductVariant {
                    id: ProductVariantId::new(Uuid::from_u128(1002)),
                    name: "١٦٠ × ٢٠٠ سم — سماكة ٢٥ سم".into(),
                    purchase_cost_yer: 65_000,
                    archived: false,
                }],
                archived: false,
            },
        ),
        "product",
    )?;
    Ok(product)
}

fn publish_catalog(manager: &Client, reception: &Client) -> Result<()> {
    checked(
        manager.catalog.synchronize(&manager.actor, deadline()),
        "upload catalog through TLS",
    )?;
    let pricing = PricingService::new(
        manager.store.clone(),
        AuthorizationService::new(manager.store.clone()),
    )
    .with_confirmation(manager.prices.clone());
    let page = checked(
        pricing.list(
            &manager.actor,
            &ListPrices {
                term: String::new(),
                after: None,
                limit: 100,
            },
        ),
        "manager prices",
    )?;
    for (index, item) in page.items.into_iter().enumerate() {
        let amount = if matches!(item.target, PriceTarget::Product(_)) {
            95_000
        } else {
            185_000
        };
        checked(
            pricing.publish(
                &mutation(&manager.actor, 20 + index as u128),
                &PublishPrice {
                    target: item.target,
                    expected_revision: None,
                    selling_price_yer: amount,
                    confirm_below_cost: false,
                },
                deadline(),
            ),
            "server-confirmed price",
        )?;
    }
    checked(
        reception.catalog.synchronize(&reception.actor, deadline()),
        "download public catalog through TLS",
    )?;
    Ok(())
}

fn seed_customer(reception: &Client) -> Result<eitmad_contracts::customer::Customer> {
    let auth = AuthorizationService::new(reception.store.clone());
    let customers = eitmad_customer::CustomerService::new(reception.store.clone(), auth.clone());
    let customer = checked(customers.create(&mutation(&reception.branch, 40), &CreateCustomer {
        name: checked(CustomerName::parse("أسرة سامر السعدي"), "customer name")?,
        phone: checked(CustomerPhone::parse("000000000"), "customer phone")?,
        address: Some(checked(CustomerAddress::parse("حي السكينة الافتراضي، منزل ١٢ — عنوان عرض غير حقيقي"), "customer address")?),
        notes: Some(checked(CustomerNotes::parse("أسرة خيالية؛ رقم غير صالح للاتصال. تجهيز غرفة النوم بخزانة ومرتبة. لا توجد بيانات أشخاص حقيقيين."), "customer notes")?),
    }), "synthetic family")?.customer;
    super::customers::synchronize(reception)?;
    Ok(customer)
}

fn seed_workflow(manager: &Client, reception: &Client) -> Result<()> {
    let customer = seed_customer(reception)?;
    let entries = checked(
        reception.store.catalog_sales(&reception.actor.scope),
        "family catalog",
    )?;
    let lines = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| QuotationLineIntent {
            id: Uuid::from_u128(100 + index as u128),
            configuration: CheckSalesConfiguration {
                selection: PriceSelection {
                    target: entry.price.target,
                    price_revision: entry.price.revision,
                    quantity: 1,
                    color_id: None,
                    handle_id: None,
                },
                dimensions: entry.dimensions,
            },
        })
        .collect();
    let drafts = QuotationDraftService::new(
        reception.store.clone(),
        AuthorizationService::new(reception.store.clone()),
    );
    let draft = checked(
        drafts.create(
            &mutation(&reception.branch, 100),
            &CreateQuotationDraft {
                intent: EvaluateQuotation {
                    customer: Some(QuotationCustomerIntent {
                        id: customer.id,
                        revision: customer.revision,
                    }),
                    lines,
                    discount_basis_points: 0,
                },
            },
        ),
        "family quotation draft",
    )?;
    checked(
        DiscountApprovalServer::transition(
            &reception.workflow,
            &reception.branch,
            &ConfirmDiscountApproval {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(101)),
                action: DiscountApprovalAction::Refresh(draft.snapshot.clone()),
            },
            deadline(),
        ),
        "confirm family draft",
    )?;
    let issued = checked(
        reception.workflow.quotation_transition(
            &reception.branch,
            &ConfirmQuotation {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(102)),
                action: QuotationAction::Issue(IssueQuotation {
                    draft_id: draft.snapshot.id,
                    expected_revision: 1,
                    expected_draft_revision: draft.snapshot.revision,
                }),
            },
            deadline(),
        ),
        "issue family quotation",
    )?;
    let accepted = checked(reception.workflow.quotation_transition(&reception.branch, &ConfirmQuotation {
        scope: reception.branch.scope.clone(), idempotency_key: IdempotencyKey::new(Uuid::from_u128(103)),
        action: QuotationAction::Accept(AcceptQuotation { draft_id: issued.quotation.id,
            expected_revision: issued.revision, method: AcceptanceMethod::InPerson,
            note: Some("وافقت أسرة السعدي على الخزانة والمرتبة بسعر إجمالي ٢٨٠٬٠٠٠ ريال. مثال اصطناعي.".into()) }),
    }, deadline()), "accept family quotation")?;
    let order = checked(
        OrderServer::transition(
            &reception.workflow,
            &reception.branch,
            &ConfirmOrder {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(104)),
                action: OrderAction::Convert(ConvertQuotation {
                    draft_id: accepted.quotation.id,
                    expected_revision: accepted.revision,
                }),
            },
            deadline(),
        ),
        "convert family order",
    )?;
    checked(
        drafts.cache_quotation(&reception.branch, &order.source, correlation()),
        "cache converted family quotation",
    )?;
    complete_work(manager, order)
}

fn complete_work(manager: &Client, mut order: OrderRecord) -> Result<()> {
    let work_id = order.work.first().ok_or("family work order missing")?.id;
    for (step, starting) in [(105, true), (106, false)] {
        let work = TransitionOrderWork {
            order_id: order.id,
            expected_revision: order.revision,
            work_id,
            due_at: Some(eitmad_contracts::transport::UnixMillis(
                order.created_at.0 + 86_400_000,
            )),
            assignment: Some("ورشة السكينة — فريق العرض الافتراضي".into()),
        };
        order = checked(
            OrderServer::transition(
                &manager.workflow,
                &manager.branch,
                &ConfirmOrder {
                    scope: manager.branch.scope.clone(),
                    idempotency_key: IdempotencyKey::new(Uuid::from_u128(step)),
                    action: if starting {
                        OrderAction::StartWork(work)
                    } else {
                        OrderAction::CompleteWork(work)
                    },
                },
                deadline(),
            ),
            "complete family wardrobe production",
        )?;
    }
    if order.state != eitmad_contracts::order::OrderState::Ready {
        return Err("family order is not ready for delivery");
    }
    Ok(())
}

pub(super) fn verify(manager: &Client, reception: &Client) -> Result<()> {
    // Restart verification must permit later rehearsal work and delivery. Never reseed or reset it.
    for client in [manager, reception] {
        super::customers::synchronize(client)?;
        checked(
            client.catalog.synchronize(&client.actor, deadline()),
            "authenticated catalog restart check",
        )?;
        let records = checked(
            client.workflow.quotations(
                &client.branch,
                &ListQuotations {
                    after: None,
                    limit: 100,
                },
                deadline(),
            ),
            "authenticated workflow restart check",
        )?;
        if records.items.is_empty() {
            return Err("seeded family quotation missing");
        }
    }
    Ok(())
}
