use super::{Client, Result, checked, correlation, deadline, mutation};
use eitmad_authorization::AuthorizationService;
use eitmad_contracts::{
    commands::CreateCustomer,
    customer::{CustomerAddress, CustomerName, CustomerNotes, CustomerPhone},
    furniture::{
        FurnitureDimensions, FurniturePart, FurnitureState, FurnitureVariant, FurnitureVariantId,
        SaveFurniture, SaveFurnitureCategory,
    },
    material::{
        MaterialQuantity, SaveMaterial, SaveMaterialCategory, SaveMaterialUnit, UnitDimension,
    },
    order::{AcceptQuotation, AcceptanceMethod, ConfirmOrder, ConvertQuotation, OrderAction},
    part::{PartUsage, SavePart, SavePartCategory},
    pricing::{ListPrices, PriceSelection, PriceTarget, PublishPrice},
    product::{
        ProductReference, ProductVariantId, SaveProduct, SaveProductCategory, SaveProductVariant,
    },
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
    let product = seed_product(manager)?;
    publish_catalog(manager, reception)?;
    seed_workflow(
        reception,
        &PriceTarget::Product(ProductReference {
            scope: reception.actor.scope.clone(),
            product_id: product.id,
            variant_id: product.variants[0].id,
            revision: product.revision,
            schema_version: 1,
        }),
    )
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
                name: "أخشاب تجريبية".into(),
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
                name: "لوح خشب تجريبي".into(),
                category_id: category.id,
                unit_id: unit.id,
                current_cost_yer: 500,
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
                name: "ألواح تجريبية".into(),
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
                name: "جانب خزانة تجريبي".into(),
                category_id: category.id,
                description: "بيانات اختبار اصطناعية".into(),
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
                name: "خزائن تجريبية".into(),
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
                name: "خزانة اختبار".into(),
                category_id: category.id,
                image: None,
                description: "خزانة ببيانات اصطناعية".into(),
                notes: "للاختبار فقط".into(),
                parts: vec![FurniturePart {
                    reference: part.composition,
                    quantity: 3,
                }],
                variants: vec![FurnitureVariant {
                    id: FurnitureVariantId::new(Uuid::from_u128(1001)),
                    name: "قياسي".into(),
                    dimensions: FurnitureDimensions {
                        width_mm: 1000,
                        height_mm: 1800,
                        depth_mm: 500,
                    },
                    customization: None,
                    selling_price_yer: 5000,
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
                name: "مراتب تجريبية".into(),
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
                name: "مرتبة اختبار".into(),
                category_id: category.id,
                image: None,
                description: "مرتبة ببيانات اصطناعية".into(),
                notes: "للاختبار فقط".into(),
                variants: vec![SaveProductVariant {
                    id: ProductVariantId::new(Uuid::from_u128(1002)),
                    name: "مفرد".into(),
                    purchase_cost_yer: 90_000,
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
            100_000
        } else {
            5000
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

fn seed_workflow(reception: &Client, target: &PriceTarget) -> Result<()> {
    let auth = AuthorizationService::new(reception.store.clone());
    let customers = eitmad_customer::CustomerService::new(reception.store.clone(), auth.clone());
    let customer = checked(
        customers.create(
            &mutation(&reception.branch, 40),
            &CreateCustomer {
                name: checked(
                    CustomerName::parse("عميل الاختبار الاصطناعي"),
                    "customer name",
                )?,
                phone: checked(CustomerPhone::parse("777123456"), "customer phone")?,
                address: Some(checked(
                    CustomerAddress::parse("صنعاء - عنوان تجريبي"),
                    "customer address",
                )?),
                notes: Some(checked(
                    CustomerNotes::parse("بيانات اصطناعية للاختبار فقط"),
                    "customer notes",
                )?),
            },
        ),
        "synthetic customer",
    )?
    .customer;
    super::customers::synchronize(reception)?;
    for (index, discount) in [(0_u128, 0), (1, 600), (2, 500), (3, 0), (4, 0)] {
        seed_quotation(reception, target, &customer, index, discount)?;
    }
    Ok(())
}

fn seed_quotation(
    reception: &Client,
    target: &PriceTarget,
    customer: &eitmad_contracts::customer::Customer,
    index: u128,
    discount: u32,
) -> Result<()> {
    let auth = AuthorizationService::new(reception.store.clone());
    let drafts = QuotationDraftService::new(reception.store.clone(), auth);
    // One draft, one pending discount approval, one issued quotation, one accepted quotation,
    // and one converted ready-made order exercise distinct durable workflow states.
    let step = 100 + index * 10;
    let draft = checked(
        drafts.create(
            &mutation(&reception.branch, step),
            &CreateQuotationDraft {
                intent: EvaluateQuotation {
                    customer: Some(QuotationCustomerIntent {
                        id: customer.id,
                        revision: customer.revision,
                    }),
                    lines: vec![QuotationLineIntent {
                        id: Uuid::from_u128(step),
                        configuration: CheckSalesConfiguration {
                            selection: PriceSelection {
                                target: target.clone(),
                                price_revision: 1,
                                quantity: 1,
                                color_id: None,
                                handle_id: None,
                            },
                            dimensions: None,
                        },
                    }],
                    discount_basis_points: discount,
                },
            },
        ),
        "quotation draft",
    )?;
    checked(
        DiscountApprovalServer::transition(
            &reception.workflow,
            &reception.branch,
            &ConfirmDiscountApproval {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(step + 1)),
                action: if index == 1 {
                    DiscountApprovalAction::Request(draft.snapshot.clone())
                } else {
                    DiscountApprovalAction::Refresh(draft.snapshot.clone())
                },
            },
            deadline(),
        ),
        "confirm draft or request discount approval",
    )?;
    if index < 2 {
        return Ok(());
    }
    finish_quotation(reception, &drafts, &draft.snapshot, index, step)
}

fn finish_quotation(
    reception: &Client,
    drafts: &QuotationDraftService,
    draft: &eitmad_contracts::quotation_draft::QuotationDraftSnapshot,
    index: u128,
    step: u128,
) -> Result<()> {
    let record = checked(
        reception.workflow.quotations(
            &reception.branch,
            &ListQuotations {
                after: None,
                limit: 100,
            },
            deadline(),
        ),
        "read confirmed quotation",
    )?
    .items
    .into_iter()
    .find(|r| r.quotation.id == draft.id)
    .ok_or("confirmed draft missing")?;
    let issued = checked(
        reception.workflow.quotation_transition(
            &reception.branch,
            &ConfirmQuotation {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(step + 2)),
                action: QuotationAction::Issue(IssueQuotation {
                    draft_id: record.quotation.id,
                    expected_revision: 1,
                    expected_draft_revision: draft.revision,
                }),
            },
            deadline(),
        ),
        "issue quotation",
    )?;
    checked(
        drafts.cache_quotation(&reception.branch, &issued, correlation()),
        "cache issued quotation",
    )?;
    if index == 2 {
        return Ok(());
    }
    let accepted = checked(
        reception.workflow.quotation_transition(
            &reception.branch,
            &ConfirmQuotation {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(step + 3)),
                action: QuotationAction::Accept(AcceptQuotation {
                    draft_id: issued.quotation.id,
                    expected_revision: issued.revision,
                    method: AcceptanceMethod::Phone,
                    note: Some("موافقة اختبار اصطناعية".into()),
                }),
            },
            deadline(),
        ),
        "accept quotation",
    )?;
    checked(
        drafts.cache_quotation(&reception.branch, &accepted, correlation()),
        "cache accepted quotation",
    )?;
    if index == 4 {
        checked(
            OrderServer::transition(
                &reception.workflow,
                &reception.branch,
                &ConfirmOrder {
                    scope: reception.branch.scope.clone(),
                    idempotency_key: IdempotencyKey::new(Uuid::from_u128(step + 4)),
                    action: OrderAction::Convert(ConvertQuotation {
                        draft_id: accepted.quotation.id,
                        expected_revision: accepted.revision,
                    }),
                },
                deadline(),
            ),
            "convert accepted quotation to order",
        )?;
    }
    Ok(())
}

pub(super) fn verify(manager: &Client, reception: &Client) -> Result<()> {
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
        if records.items.len() < 5 {
            return Err("seeded workflow state missing");
        }
    }
    Ok(())
}
