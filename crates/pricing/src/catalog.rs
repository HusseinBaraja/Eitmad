use crate::{PricingError, PricingService, Source};
use eitmad_contracts::{
    catalog_revision::CatalogRevision,
    furniture::FurnitureState,
    pricing::{PriceAdjustment, PriceTarget},
};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

type RevisionKey = (&'static str, Uuid, u64);

#[must_use]
pub fn revision_schema(record: &CatalogRevision) -> &'static str {
    match record {
        CatalogRevision::Unit(_)
        | CatalogRevision::Material(_)
        | CatalogRevision::MaterialCategory(_) => "eitmad.schema.material.v1",
        CatalogRevision::Part(_) | CatalogRevision::PartCategory(_) => "eitmad.schema.part.v1",
        CatalogRevision::Product(_) | CatalogRevision::ProductCategory(_) => {
            "eitmad.schema.product.v1"
        }
        CatalogRevision::Furniture(_) | CatalogRevision::FurnitureCategory(_) => {
            "eitmad.schema.furniture.v1"
        }
    }
}

/// A stable envelope identity for an immutable domain revision, independent of transfer attempts.
#[must_use]
pub fn revision_record_id(record: &CatalogRevision) -> Uuid {
    let (kind, id, revision, _) = record.identity();
    catalog_record_id(&format!("{kind}:{id}:{revision}"))
}

#[must_use]
pub fn catalog_record_id(identity: &str) -> Uuid {
    use sha2::{Digest as _, Sha256};
    let digest = Sha256::digest(identity.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}

/// Builds an explicit public allowlist from one validated definition and its exact price receipt.
/// # Errors
/// Rejects missing, archived, or mismatched definition/variant references.
pub fn public_entry(
    record: &CatalogRevision,
    price: &eitmad_contracts::pricing::PublishedPrice,
) -> Result<eitmad_contracts::catalog_revision::CatalogEntry, PricingError> {
    publication_basis(record, &price.target)?;
    let mut entry = eitmad_contracts::catalog_revision::CatalogEntry {
        price: price.clone(),
        name: String::new(),
        category_name: String::new(),
        description: String::new(),
        variant_name: String::new(),
        image: None,
        dimensions: None,
        customization: None,
        colors: vec![],
        handles: vec![],
    };
    let variant_id = price.target.identity().2;
    match record {
        CatalogRevision::Product(p) => {
            entry.name.clone_from(&p.name);
            entry.category_name.clone_from(&p.category_name);
            entry.description.clone_from(&p.description);
            entry.image.clone_from(&p.image);
            entry.variant_name.clone_from(
                &p.variants
                    .iter()
                    .find(|v| v.id.value() == variant_id)
                    .ok_or(PricingError::Reference)?
                    .name,
            );
        }
        CatalogRevision::Furniture(f) => {
            let variant = f
                .variants
                .iter()
                .find(|v| v.id.value() == variant_id)
                .ok_or(PricingError::Reference)?;
            entry.name.clone_from(&f.name);
            entry.category_name.clone_from(&f.category_name);
            entry.description.clone_from(&f.description);
            entry.image.clone_from(&f.image);
            entry.variant_name.clone_from(&variant.name);
            entry.dimensions = Some(variant.dimensions.clone());
            entry.customization.clone_from(&variant.customization);
            entry.colors = f
                .colors
                .iter()
                .filter(|o| price.colors.iter().any(|p| p.id == o.id))
                .cloned()
                .collect();
            entry.handles = f
                .handles
                .iter()
                .filter(|o| price.handles.iter().any(|p| p.id == o.id))
                .cloned()
                .collect();
        }
        _ => return Err(PricingError::Reference),
    }
    Ok(entry)
}

/// Lists stored dependencies; revision zero denotes the current category.
/// # Errors
/// Rejects unbounded collections and zero Part revisions before database work.
pub fn catalog_dependencies(record: &CatalogRevision) -> Result<Vec<RevisionKey>, PricingError> {
    if matches!(record, CatalogRevision::Part(p) if p.cost.rows.is_empty() || p.cost.rows.len() > 100)
        || matches!(record, CatalogRevision::Furniture(f) if f.parts.len() > 100)
    {
        return Err(PricingError::Invalid);
    }
    if matches!(record, CatalogRevision::Furniture(f) if f.parts.iter().any(|p| p.reference.revision == 0))
    {
        return Err(PricingError::Reference);
    }
    Ok(match record {
        CatalogRevision::Part(p) => std::iter::once(("part-category", p.category_id.value(), 0))
            .chain(p.cost.rows.iter().flat_map(|r| {
                [
                    ("material", r.material.id.value(), r.material.revision),
                    ("unit", r.unit.id.value(), r.unit.revision),
                    ("unit", r.cost_unit.id.value(), r.cost_unit.revision),
                ]
            }))
            .collect(),
        CatalogRevision::Material(m) => vec![
            ("material-category", m.category_id.value(), 0),
            ("unit", m.unit_id.value(), 0),
        ],
        CatalogRevision::Product(p) => vec![("product-category", p.category_id.value(), 0)],
        CatalogRevision::Furniture(f) => {
            std::iter::once(("furniture-category", f.category_id.value(), 0))
                .chain(
                    f.parts
                        .iter()
                        .map(|p| ("part", p.reference.part_id.value(), p.reference.revision)),
                )
                .collect()
        }
        _ => vec![],
    })
}

/// Validates a revision against authority-owned dependencies without accepting calculated totals.
/// # Errors
/// Rejects foreign scopes, invalid definitions, missing references, or altered cost snapshots.
pub fn validate_catalog_revision(
    record: &CatalogRevision,
    known: &BTreeMap<RevisionKey, CatalogRevision>,
) -> Result<(), PricingError> {
    let (_, id, revision, scope) = record.identity();
    if id.is_nil()
        || revision == 0
        || i64::try_from(revision).is_err()
        || scope.kind.as_str() != "organization"
        || scope.id.value().is_nil()
    {
        return Err(PricingError::Invalid);
    }
    for key in catalog_dependencies(record)? {
        let dependency = known.get(&key).ok_or(PricingError::Reference)?;
        if dependency.identity().3 != scope {
            return Err(PricingError::Reference);
        }
    }
    match record {
        CatalogRevision::Unit(v) => {
            name(&v.name)?;
            name(&v.symbol)?;
            if v.numerator == 0 || v.denominator == 0 {
                return Err(PricingError::Invalid);
            }
        }
        CatalogRevision::MaterialCategory(v) => name(&v.name)?,
        CatalogRevision::PartCategory(v) => name(&v.name)?,
        CatalogRevision::Material(v) => {
            name(&v.name)?;
            if v.current_cost_yer < 0
                || v.unit_id.value().is_nil()
                || v.category_id.value().is_nil()
            {
                return Err(PricingError::Invalid);
            }
        }
        CatalogRevision::Part(v) => validate_part(v, scope, known)?,
        CatalogRevision::ProductCategory(v) => name(&v.name)?,
        CatalogRevision::FurnitureCategory(v) => name(&v.name)?,
        CatalogRevision::Product(v) => validate_product(v)?,
        CatalogRevision::Furniture(v) => validate_furniture(v, scope, known)?,
    }
    Ok(())
}

/// Resolves cost and permitted option adjustments from the exact active definition.
/// # Errors
/// Rejects stale or foreign references, archived definitions, and missing variants.
pub fn publication_basis(
    record: &CatalogRevision,
    target: &PriceTarget,
) -> Result<(i64, Vec<PriceAdjustment>, Vec<PriceAdjustment>), PricingError> {
    let (kind, id, revision, scope) = record.identity();
    let (target_kind, target_id, variant_id) = target.identity();
    if kind != target_kind
        || id != target_id
        || revision != target.revision()
        || scope != target.scope()
        || target.schema_version() != 1
    {
        return Err(PricingError::Reference);
    }
    let source = match record {
        CatalogRevision::Product(p) if !p.archived => {
            let index = p
                .variants
                .iter()
                .position(|v| v.id.value() == variant_id && !v.archived)
                .ok_or(PricingError::Reference)?;
            if p.variants[index].purchase_cost_yer.is_none() {
                return Err(PricingError::Reference);
            }
            Source::Product((**p).clone(), index)
        }
        CatalogRevision::Furniture(f) if f.state == FurnitureState::Active => {
            let index = f
                .variants
                .iter()
                .position(|v| v.id.value() == variant_id && !v.archived)
                .ok_or(PricingError::Reference)?;
            Source::Furniture((**f).clone(), index)
        }
        _ => return Err(PricingError::Reference),
    };
    let (colors, handles) = source.adjustments();
    Ok((source.cost(), colors, handles))
}

/// Enforces below-cost policy against the stored definition and rejects substituted costs or options.
/// # Errors
/// Rejects stale references, unconfirmed below-cost prices, and altered proposal fields.
pub fn validate_server_proposal(
    input: &eitmad_contracts::pricing::ConfirmPrice,
    record: &CatalogRevision,
) -> Result<(), PricingError> {
    let (cost, colors, handles) = publication_basis(record, &input.command.target)?;
    let mut authoritative = input.clone();
    authoritative.cost_yer = cost;
    authoritative.colors = colors;
    authoritative.handles = handles;
    crate::validate_publication(&authoritative)?;
    if authoritative != *input {
        return Err(PricingError::Reference);
    }
    Ok(())
}

fn name(value: &str) -> Result<(), PricingError> {
    if value.is_empty() || value.len() > 200 || value.trim() != value
        || value.chars().any(|c| c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}'))
    { return Err(PricingError::Invalid); }
    Ok(())
}

impl PricingService {
    /// Builds the immutable dependency graph from Rust storage, never from the shell proposal.
    pub(super) fn catalog_revisions(
        &self,
        target: &PriceTarget,
    ) -> Result<Vec<CatalogRevision>, PricingError> {
        self.store.transact_pricing(false, |tx| {
            let mut records = BTreeMap::new();
            let mut insert = |record: CatalogRevision| -> Result<(), PricingError> {
                let (kind, id, revision, _) = record.identity();
                let order = match kind {
                    "unit" | "material-category" | "part-category" => 0,
                    "material" => 1,
                    "part" => 2,
                    "product-category" | "furniture-category" => 3,
                    _ => 4,
                };
                let key = (order, kind, id, revision);
                if records
                    .get(&key)
                    .is_some_and(|existing| existing != &record)
                {
                    return Err(PricingError::Reference);
                }
                records.insert(key, record);
                Ok(())
            };
            match target {
                PriceTarget::Product(r) => {
                    let p = tx.products().revision(r)?.ok_or(PricingError::Reference)?;
                    let c = tx
                        .products()
                        .category(&r.scope, p.category_id.value())?
                        .ok_or(PricingError::Reference)?;
                    insert(CatalogRevision::ProductCategory(Box::new(c)))?;
                    insert(CatalogRevision::Product(Box::new(p)))?;
                }
                PriceTarget::Furniture(r) => {
                    let f = tx
                        .furnitures()
                        .revision(r)?
                        .ok_or(PricingError::Reference)?;
                    let c = tx
                        .furnitures()
                        .category(&r.scope, f.category_id.value())?
                        .ok_or(PricingError::Reference)?;
                    insert(CatalogRevision::FurnitureCategory(Box::new(c)))?;
                    for usage in &f.parts {
                        let part = tx
                            .furnitures()
                            .composition(&usage.reference)?
                            .ok_or(PricingError::Reference)?;
                        let category = tx
                            .furnitures()
                            .part_category(&r.scope, part.category_id.value())?
                            .ok_or(PricingError::Reference)?;
                        insert(CatalogRevision::PartCategory(Box::new(category)))?;
                        for row in &part.cost.rows {
                            let category = tx
                                .furnitures()
                                .material_category(&r.scope, row.material.category_id.value())?
                                .ok_or(PricingError::Reference)?;
                            insert(CatalogRevision::MaterialCategory(Box::new(category)))?;
                            insert(CatalogRevision::Unit(Box::new(row.unit.clone())))?;
                            insert(CatalogRevision::Unit(Box::new(row.cost_unit.clone())))?;
                            insert(CatalogRevision::Material(Box::new(row.material.clone())))?;
                        }
                        insert(CatalogRevision::Part(Box::new(part)))?;
                    }
                    insert(CatalogRevision::Furniture(Box::new(f)))?;
                }
            }
            Ok(records.into_values().collect())
        })
    }
}

fn validate_part(
    v: &eitmad_contracts::part::Part,
    scope: &eitmad_contracts::identity::ScopeRef,
    known: &BTreeMap<RevisionKey, CatalogRevision>,
) -> Result<(), PricingError> {
    name(&v.name)?;
    if v.composition.scope != *scope
        || v.composition.part_id != v.id
        || v.composition.revision != v.revision
        || v.composition.schema_version != 1
    {
        return Err(PricingError::Reference);
    }
    for row in &v.cost.rows {
        for dependency in [
            CatalogRevision::Material(Box::new(row.material.clone())),
            CatalogRevision::Unit(Box::new(row.unit.clone())),
            CatalogRevision::Unit(Box::new(row.cost_unit.clone())),
        ] {
            let (kind, id, rev, _) = dependency.identity();
            if known.get(&(kind, id, rev)) != Some(&dependency) {
                return Err(PricingError::Reference);
            }
        }
    }
    eitmad_part::verify_snapshot_cost(&v.cost).map_err(|_| PricingError::Invalid)?;

    Ok(())
}

fn validate_product(v: &eitmad_contracts::product::Product) -> Result<(), PricingError> {
    name(&v.name)?;
    if v.variants.is_empty() || v.variants.len() > 100 {
        return Err(PricingError::Invalid);
    }
    let mut ids = HashSet::new();
    for variant in &v.variants {
        name(&variant.name)?;
        if variant.id.value().is_nil()
            || !ids.insert(variant.id)
            || variant.purchase_cost_yer.is_none_or(|c| c < 0)
        {
            return Err(PricingError::Invalid);
        }
    }

    Ok(())
}

fn validate_furniture(
    v: &eitmad_contracts::furniture::Furniture,
    scope: &eitmad_contracts::identity::ScopeRef,
    known: &BTreeMap<RevisionKey, CatalogRevision>,
) -> Result<(), PricingError> {
    name(&v.name)?;
    if v.parts.len() > 100
        || v.variants.is_empty()
        || v.variants.len() > 100
        || v.colors.len() > 100
        || v.handles.len() > 100
        || v.state == FurnitureState::Active && v.parts.is_empty()
    {
        return Err(PricingError::Invalid);
    }
    let mut total = 0_i64;
    let mut part_ids = HashSet::new();
    for usage in &v.parts {
        let r = &usage.reference;
        if r.scope != *scope
            || r.schema_version != 1
            || usage.quantity == 0
            || !part_ids.insert(r.part_id)
        {
            return Err(PricingError::Reference);
        }
        let Some(CatalogRevision::Part(part)) = known.get(&("part", r.part_id.value(), r.revision))
        else {
            return Err(PricingError::Reference);
        };
        if part.id != r.part_id
            || part.revision != r.revision
            || part.scope != r.scope
            || part.composition != *r
        {
            return Err(PricingError::Reference);
        }
        total = total
            .checked_add(
                part.cost
                    .total_cost_yer
                    .checked_mul(i64::from(usage.quantity))
                    .ok_or(PricingError::Invalid)?,
            )
            .ok_or(PricingError::Invalid)?;
    }
    if total != v.parts_cost_yer {
        return Err(PricingError::Invalid);
    }
    let mut ids = HashSet::new();
    for variant in &v.variants {
        name(&variant.name)?;
        if variant.id.value().is_nil()
            || !ids.insert(variant.id.value())
            || variant.dimensions.width_mm == 0
            || variant.dimensions.height_mm == 0
            || variant.dimensions.depth_mm == 0
            || variant.selling_price_yer < 0
        {
            return Err(PricingError::Invalid);
        }
    }
    for option in v.colors.iter().chain(&v.handles) {
        name(&option.name)?;
        if option.id.is_nil() || !ids.insert(option.id) || option.price_adjustment_yer < 0 {
            return Err(PricingError::Invalid);
        }
    }
    for variant in &v.variants {
        if variant
            .color_ids
            .iter()
            .any(|id| !v.colors.iter().any(|o| o.id == *id))
            || variant
                .handle_ids
                .iter()
                .any(|id| !v.handles.iter().any(|o| o.id == *id))
        {
            return Err(PricingError::Reference);
        }
    }

    Ok(())
}
