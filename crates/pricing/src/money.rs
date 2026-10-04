//! Checked integer arithmetic. Money is whole YER; discount precision is 1 basis point.
use crate::PricingError;
use eitmad_contracts::pricing::{CalculateDiscount, ConfirmPrice, DiscountTotal};

/// Calculates the quotation discount once on the subtotal, half away from zero.
/// # Errors
/// Rejects fractional representations at deserialization, negative values, invalid rates, and overflow.
pub fn discount(input: &CalculateDiscount) -> Result<DiscountTotal, PricingError> {
    if input.line_totals_yer.is_empty()
        || input.line_totals_yer.len() > 1000
        || input.discount_basis_points > 10_000
    {
        return Err(PricingError::Invalid);
    }
    let subtotal = input
        .line_totals_yer
        .iter()
        .try_fold(0_i64, |total, value| {
            if *value < 0 {
                return Err(PricingError::Invalid);
            }
            total.checked_add(*value).ok_or(PricingError::Invalid)
        })?;
    let discount = i64::try_from(
        (i128::from(subtotal) * i128::from(input.discount_basis_points) + 5_000) / 10_000,
    )
    .map_err(|_| PricingError::Invalid)?;
    Ok(DiscountTotal {
        subtotal_yer: subtotal,
        discount_yer: discount,
        total_yer: subtotal - discount,
        approval_required: input.discount_basis_points > 500,
    })
}

/// Validates the public price proposal at both local and server boundaries.
/// # Errors
/// Rejects invalid identities, revisions, currency amounts, duplicate options, or overflow.
pub fn validate_publication(input: &ConfirmPrice) -> Result<(), PricingError> {
    let target = &input.command.target;
    if input.cost_yer < 0 {
        return Err(PricingError::Invalid);
    }
    let (_, entry, variant) = target.identity();
    if target.scope().kind.as_str() != "organization"
        || entry.is_nil()
        || variant.is_nil()
        || target.schema_version() != 1
        || target.revision() == 0
        || i64::try_from(target.revision()).is_err()
        || input
            .command
            .expected_revision
            .is_some_and(|r| r == 0 || r >= i64::MAX as u64)
        || input.command.selling_price_yer <= 0
        || input.colors.len() > 100
        || input.handles.len() > 100
        || matches!(target, eitmad_contracts::pricing::PriceTarget::Product(_))
            && (!input.colors.is_empty() || !input.handles.is_empty())
    {
        return Err(PricingError::Invalid);
    }
    if input.command.selling_price_yer < input.cost_yer && !input.command.confirm_below_cost {
        return Err(PricingError::BelowCost);
    }
    let mut maximum = [0, 0];
    for (index, options) in [&input.colors, &input.handles].iter().enumerate() {
        let mut ids = std::collections::HashSet::new();
        for option in *options {
            if option.id.is_nil() || option.price_adjustment_yer < 0 || !ids.insert(option.id) {
                return Err(PricingError::Invalid);
            }
            maximum[index] = maximum[index].max(option.price_adjustment_yer);
        }
    }
    input
        .command
        .selling_price_yer
        .checked_add(maximum[0])
        .and_then(|v| v.checked_add(maximum[1]))
        .ok_or(PricingError::Invalid)?;
    Ok(())
}
