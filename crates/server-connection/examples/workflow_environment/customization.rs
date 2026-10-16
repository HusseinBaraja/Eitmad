//! Add a separate, published customization sample without changing accepted family work.
use super::{Client, Result, checked, correlation, deadline, mutation};
use eitmad_authorization::AuthorizationService;
use eitmad_contracts::{
    furniture::{
        Furniture, FurnitureCustomization, FurnitureDimensions, FurnitureOption,
        FurnitureReference, FurnitureState, FurnitureVariant, FurnitureVariantId, ListFurnitures,
        SaveFurniture,
    },
    pricing::{PriceSelection, PriceTarget, PublishPrice},
    quotation::{EvaluateQuotation, QuotationField, QuotationIssue, QuotationLineIntent},
    quotation_approval::{ConfirmDiscountApproval, DiscountApprovalAction},
    quotation_draft::CreateQuotationDraft,
    quotation_lifecycle::{
        ConfirmQuotation, IssueQuotation, ListQuotations, QuotationAction, QuotationRecord,
        QuotationState,
    },
    sales_catalog::{CheckSalesConfiguration, GetSalesCatalogItem},
    transport::IdempotencyKey,
};
use eitmad_furniture::FurnitureService;
use eitmad_pricing::{
    CatalogReplication, DiscountApprovalServer, PricingService, QuotationDraftService,
    QuotationServer,
};
use std::path::Path;
use uuid::Uuid;

fn dimensions(width_mm: u32, height_mm: u32, depth_mm: u32) -> FurnitureDimensions {
    FurnitureDimensions {
        width_mm,
        height_mm,
        depth_mm,
    }
}

fn option(id: u128, name: &str, visual: &str, price_adjustment_yer: i64) -> FurnitureOption {
    FurnitureOption {
        id: Uuid::from_u128(id),
        name: name.into(),
        visual: visual.into(),
        price_adjustment_yer,
        archived: false,
    }
}

fn definition(source: Furniture) -> SaveFurniture {
    SaveFurniture {
        id: None,
        expected_revision: None,
        name: "خزانة السكينة حسب المقاس".into(),
        category_id: source.category_id,
        image: source.image,
        description: "خزانة ببابين؛ اختر المقاس واللون والمقبض. المقبض النحاسي يضيف ١٥٬٠٠٠ ريال."
            .into(),
        notes: "عينة تخصيص اصطناعية مستقلة عن طلب الأسرة؛ الصورة توضيحية.".into(),
        parts: source.parts,
        variants: [
            (3001, 1200, 185_000, "١٢٠ × ٢٠٠ × ٦٠ سم"),
            (3002, 1600, 225_000, "١٦٠ × ٢٠٠ × ٦٠ سم"),
        ]
        .into_iter()
        .map(|(id, width, price, name)| FurnitureVariant {
            id: FurnitureVariantId::new(Uuid::from_u128(id)),
            name: name.into(),
            dimensions: dimensions(width, 2000, 600),
            customization: Some(FurnitureCustomization {
                minimum: dimensions(1000, 1800, 500),
                maximum: dimensions(1800, 2400, 700),
            }),
            selling_price_yer: price,
            archived: false,
            color_ids: vec![Uuid::from_u128(3101), Uuid::from_u128(3102)],
            handle_ids: vec![Uuid::from_u128(3201), Uuid::from_u128(3202)],
        })
        .collect(),
        colors: vec![
            option(3101, "جوزي", "#6F4E37", 0),
            option(3102, "أبيض", "#FFFFFF", 0),
        ],
        handles: vec![
            option(3201, "مقبض أسود", "BlackMetal", 0),
            option(3202, "مقبض نحاسي (+١٥٬٠٠٠ ريال)", "Brass", 15_000),
        ],
        state: FurnitureState::Active,
        confirm_below_cost: false,
    }
}

fn publish(manager: &Client) -> Result<Furniture> {
    let furniture = FurnitureService::new(
        manager.store.clone(),
        AuthorizationService::new(manager.store.clone()),
    );
    let source = checked(
        furniture.list(
            &manager.actor,
            &ListFurnitures {
                term: "خزانة السكينة ببابين".into(),
                after: None,
                limit: 100,
                selectable_only: true,
            },
        ),
        "source wardrobe",
    )?
    .items
    .into_iter()
    .find(|item| {
        item.variants
            .iter()
            .any(|v| v.id.value() == Uuid::from_u128(1001))
    })
    .ok_or("source wardrobe missing")?;
    // Stable keys resume a partial preparation. They never update an existing definition.
    let sample = checked(
        furniture.save(&mutation(&manager.actor, 300), &definition(source)),
        "save customization sample",
    )?;
    checked(
        manager.catalog.synchronize(&manager.actor, deadline()),
        "publish customization definition through TLS",
    )?;
    let pricing = PricingService::new(
        manager.store.clone(),
        AuthorizationService::new(manager.store.clone()),
    )
    .with_confirmation(manager.prices.clone());
    for (step, variant) in (310..).zip(&sample.variants) {
        checked(
            pricing.publish(
                &mutation(&manager.actor, step),
                &PublishPrice {
                    target: PriceTarget::Furniture(FurnitureReference {
                        scope: sample.scope.clone(),
                        furniture_id: sample.id,
                        variant_id: variant.id,
                        revision: sample.revision,
                        schema_version: 1,
                    }),
                    expected_revision: None,
                    selling_price_yer: variant.selling_price_yer,
                    confirm_below_cost: false,
                },
                deadline(),
            ),
            "server-confirmed customization price",
        )?;
    }
    Ok(sample)
}

pub(super) fn run(root: &Path, manager: &Client, reception: &Client) -> Result<()> {
    let sample = publish(manager)?;
    checked(
        reception.catalog.synchronize(&reception.actor, deadline()),
        "Receptionist downloads customization",
    )?;
    super::customers::synchronize(reception)?;
    let pricing = PricingService::new(
        reception.store.clone(),
        AuthorizationService::new(reception.store.clone()),
    );
    let target = PriceTarget::Furniture(FurnitureReference {
        scope: sample.scope.clone(),
        furniture_id: sample.id,
        variant_id: sample.variants[0].id,
        revision: sample.revision,
        schema_version: 1,
    });
    let details = checked(
        pricing.sales_catalog_item(
            &reception.actor,
            &GetSalesCatalogItem {
                target: target.clone(),
            },
        ),
        "Receptionist customization catalog",
    )?;
    if details.variants.len() != 2 {
        return Err("two published sizes required");
    }
    let mut configurations = Vec::new();
    for entry in &details.variants {
        let configuration = CheckSalesConfiguration {
            selection: PriceSelection {
                target: entry.price.target.clone(),
                price_revision: entry.price.revision,
                quantity: 1,
                color_id: Some(Uuid::from_u128(3101)),
                handle_id: Some(Uuid::from_u128(3202)),
            },
            dimensions: Some(dimensions(1400, 2100, 600)),
        };
        let selected = checked(
            pricing.sales_configuration(&reception.actor, &configuration),
            "Receptionist selects custom size and paid handle",
        )?;
        let expected = if configuration.selection.target == target {
            200_000
        } else {
            240_000
        };
        if selected.price.total_yer != expected || selected.additions_yer != 15_000 {
            return Err("customization total mismatch");
        }
        configurations.push(configuration);
    }
    let configuration = configurations
        .into_iter()
        .find(|c| c.selection.target == target)
        .ok_or("small customization variant missing")?;
    let issued = quote(reception, &pricing, configuration.clone())?;
    let mut invalid = issued.quotation.intent.clone();
    invalid.lines[0].configuration.dimensions = Some(dimensions(1801, 2100, 600));
    let rejected = checked(
        pricing.evaluate_quotation(&reception.branch, &invalid),
        "evaluate out-of-range size",
    )?;
    if rejected.totals.is_some()
        || !rejected
            .errors
            .iter()
            .any(|e| e.field == QuotationField::Dimensions && e.issue == QuotationIssue::Invalid)
    {
        return Err("out-of-range size was not rejected");
    }
    let proof = serde_json::json!({
        "verified_utc": eitmad_authorization::now(),
        "verification": "Rust CLI; authenticated Receptionist; real TLS and PostgreSQL; no desktop automation",
        "published_variants": details.variants,
        "selected": configuration,
        "quotation_number": issued.number, "quotation_state": issued.state,
        "quotation_lines": issued.quotation.evaluation.lines,
        "totals": issued.quotation.evaluation.totals,
        "rejected_dimensions_mm": [1801, 2100, 600],
        "rejection_errors": rejected.errors, "rejected_totals": rejected.totals,
    });
    checked(
        std::fs::write(
            root.join("customization-proof.json"),
            checked(
                serde_json::to_vec_pretty(&proof),
                "encode customization proof",
            )?,
        ),
        "save customization proof",
    )?;
    println!(
        "Customization verified: two published sizes; custom 1400 x 2100 x 600 mm; 185000 + 15000 = 200000 YER; width 1801 mm rejected."
    );
    Ok(())
}

fn quote(
    reception: &Client,
    pricing: &PricingService,
    configuration: CheckSalesConfiguration,
) -> Result<QuotationRecord> {
    let family = checked(
        reception.workflow.quotations(
            &reception.branch,
            &ListQuotations {
                after: None,
                limit: 100,
            },
            deadline(),
        ),
        "read family customer",
    )?
    .items
    .into_iter()
    .find(|q| q.state == QuotationState::Converted)
    .ok_or("family quotation missing")?;
    let intent = EvaluateQuotation {
        customer: family.quotation.intent.customer,
        lines: vec![QuotationLineIntent {
            id: Uuid::from_u128(330),
            configuration,
        }],
        discount_basis_points: 0,
    };
    let evaluated = checked(
        pricing.evaluate_quotation(&reception.branch, &intent),
        "evaluate customization quotation",
    )?;
    if !evaluated.errors.is_empty()
        || evaluated.totals.as_ref().map(|t| t.total_yer) != Some(200_000)
    {
        return Err("customization quotation total mismatch");
    }
    let drafts = QuotationDraftService::new(
        reception.store.clone(),
        AuthorizationService::new(reception.store.clone()),
    );
    let draft = checked(
        drafts.create(
            &mutation(&reception.branch, 330),
            &CreateQuotationDraft { intent },
        ),
        "save customization quotation",
    )?;
    checked(
        DiscountApprovalServer::transition(
            &reception.workflow,
            &reception.branch,
            &ConfirmDiscountApproval {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(331)),
                action: DiscountApprovalAction::Refresh(draft.snapshot.clone()),
            },
            deadline(),
        ),
        "server validates customization quotation",
    )?;
    let issued = checked(
        reception.workflow.quotation_transition(
            &reception.branch,
            &ConfirmQuotation {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(332)),
                action: QuotationAction::Issue(IssueQuotation {
                    draft_id: draft.snapshot.id,
                    expected_revision: 1,
                    expected_draft_revision: draft.snapshot.revision,
                }),
            },
            deadline(),
        ),
        "issue customization quotation through TLS",
    )?;
    if issued.quotation.evaluation.lines != evaluated.lines
        || issued.quotation.evaluation.totals != evaluated.totals
    {
        return Err("server quotation specification or total mismatch");
    }
    checked(
        drafts.cache_quotation(&reception.branch, &issued, correlation()),
        "cache issued customization quotation",
    )?;
    Ok(issued)
}
