//! Public catalog queries never fall back to private manager definitions.
use crate::{PricingError, PricingService, adjustment};
use eitmad_authorization::CATALOG_READ_PERMISSION;
use eitmad_contracts::{
    identity::AuthorizationContext,
    pricing::{PriceTarget, SellingPrice},
    sales_catalog::{
        CheckSalesConfiguration, GetSalesCatalogItem, ListSalesCatalog, SalesCatalogDetails,
        SalesCatalogPage, SalesConfiguration,
    },
};

impl PricingService {
    /// Searches confirmed sales entries in bounded storage batches with Arabic normalization.
    /// # Errors
    /// Rejects denied, oversized, or unavailable reads.
    pub fn sales_catalog(
        &self,
        actor: &AuthorizationContext,
        input: &ListSalesCatalog,
    ) -> Result<SalesCatalogPage, PricingError> {
        self.require(actor, CATALOG_READ_PERMISSION)?;
        if !(1..=100).contains(&input.limit)
            || input.term.len() > 256
            || input.category.as_ref().is_some_and(|c| c.len() > 256)
        {
            return Err(PricingError::Invalid);
        }
        let term = eitmad_material::normalize_search(&input.term);
        let mut cursor = input.after;
        let mut items = Vec::new();
        let mut ids = Vec::new();
        let mut scan_exhausted = true;
        for _ in 0..20 {
            let batch = self.store.catalog_sales_batch(&actor.scope, cursor)?;
            let ended = batch.len() < 100;
            for (id, entry) in batch {
                cursor = Some(id);
                if input
                    .category
                    .as_ref()
                    .is_none_or(|c| c == &entry.category_name)
                    && eitmad_material::normalize_search(&format!(
                        "{} {} {}",
                        entry.name, entry.variant_name, entry.category_name
                    ))
                    .contains(&term)
                {
                    items.push(entry);
                    ids.push(id);
                    if items.len() > input.limit as usize {
                        break;
                    }
                }
            }
            if ended || items.len() > input.limit as usize {
                scan_exhausted = false;
                break;
            }
        }
        let next = if items.len() > input.limit as usize {
            items.truncate(input.limit as usize);
            ids.get(input.limit as usize - 1).copied()
        } else if scan_exhausted {
            cursor
        } else {
            None
        };
        self.require(actor, CATALOG_READ_PERMISSION)?;
        Ok(SalesCatalogPage {
            items,
            next,
            categories: self.store.catalog_sales_categories(&actor.scope)?,
            server_available: false,
        })
    }

    /// Reads all current public variants for explicit selection or refresh, at most 100.
    /// # Errors
    /// Rejects foreign scopes, archives, missing publication, and denied reads.
    pub fn sales_catalog_item(
        &self,
        actor: &AuthorizationContext,
        input: &GetSalesCatalogItem,
    ) -> Result<SalesCatalogDetails, PricingError> {
        self.require(actor, CATALOG_READ_PERMISSION)?;
        if input.target.scope() != &actor.scope || input.target.schema_version() != 1 {
            return Err(PricingError::Reference);
        }
        let variants = self.store.catalog_sales_item(&actor.scope, &input.target)?;
        if variants.is_empty() {
            return Err(PricingError::Reference);
        }
        self.require(actor, CATALOG_READ_PERMISSION)?;
        Ok(SalesCatalogDetails {
            variants,
            server_available: false,
        })
    }

    /// Validates the exact price, definition, options, dimensions, and quantity in Rust.
    /// # Errors
    /// Rejects stale prices, changed definitions, archives, incompatible choices, and overflow.
    pub fn sales_configuration(
        &self,
        actor: &AuthorizationContext,
        input: &CheckSalesConfiguration,
    ) -> Result<SalesConfiguration, PricingError> {
        let selection = &input.selection;
        let details = self.sales_catalog_item(
            actor,
            &GetSalesCatalogItem {
                target: selection.target.clone(),
            },
        )?;
        let entry = details
            .variants
            .into_iter()
            .find(|e| e.price.target == selection.target)
            .ok_or(PricingError::Reference)?;
        if entry.price.revision != selection.price_revision {
            return Err(PricingError::Conflict {
                expected: Some(selection.price_revision),
                actual: Some(entry.price.revision),
            });
        }
        if selection.quantity == 0 || selection.quantity > 1_000_000 {
            return Err(PricingError::Invalid);
        }
        match &selection.target {
            PriceTarget::Product(_)
                if input.dimensions.is_some()
                    || selection.color_id.is_some()
                    || selection.handle_id.is_some() =>
            {
                return Err(PricingError::Invalid);
            }
            PriceTarget::Product(_) => {}
            PriceTarget::Furniture(_) => {
                eitmad_furniture::validate_selection_dimensions(
                    input.dimensions.as_ref().ok_or(PricingError::Invalid)?,
                    entry.dimensions.as_ref().ok_or(PricingError::Reference)?,
                    entry.customization.as_ref(),
                )
                .map_err(|_| PricingError::Invalid)?;
                for (selected, options) in [
                    (selection.color_id, &entry.colors),
                    (selection.handle_id, &entry.handles),
                ] {
                    if selected.map_or(!options.is_empty(), |id| {
                        !options.iter().any(|o| o.id == id && !o.archived)
                    }) {
                        return Err(PricingError::Reference);
                    }
                }
            }
        }
        let color = adjustment(&entry.price.colors, selection.color_id)?;
        let handle = adjustment(&entry.price.handles, selection.handle_id)?;
        let unit = entry
            .price
            .selling_price_yer
            .checked_add(color)
            .and_then(|v| v.checked_add(handle))
            .ok_or(PricingError::Invalid)?;
        if unit <= 0 {
            return Err(PricingError::Invalid);
        }
        let total = unit
            .checked_mul(i64::from(selection.quantity))
            .ok_or(PricingError::Invalid)?;
        let price = SellingPrice {
            snapshot: entry.price.clone(),
            unit_price_yer: unit,
            total_yer: total,
        };
        self.require(actor, CATALOG_READ_PERMISSION)?;
        Ok(SalesConfiguration {
            entry,
            dimensions: input.dimensions.clone(),
            price,
            additions_yer: color.checked_add(handle).ok_or(PricingError::Invalid)?,
            server_available: false,
        })
    }
}
