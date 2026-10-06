//! Quotation evaluation reads one public catalog/customer snapshot and never commits a quotation.
use crate::{PricingError, PricingService, discount, sales_catalog::validate_configuration};
use eitmad_authorization::{CATALOG_READ_PERMISSION, CUSTOMER_READ_PERMISSION};
use eitmad_contracts::{
    customer::CustomerStatus,
    identity::{AuthorizationContext, ScopeId, ScopeKind, ScopeRef},
    pricing::CalculateDiscount,
    quotation::{
        EvaluateQuotation, EvaluatedQuotationLine, QuotationCustomerSnapshot, QuotationEvaluation,
        QuotationField as Field, QuotationFieldError, QuotationIssue as Issue,
    },
};
use std::collections::HashSet;

impl PricingService {
    /// Authorizes the exact branch customer and tenant catalog before evaluating intent.
    /// # Errors
    /// Denied or unavailable reads fail the entire query. Validation errors withhold totals.
    pub fn evaluate_quotation(
        &self,
        actor: &AuthorizationContext,
        input: &EvaluateQuotation,
    ) -> Result<QuotationEvaluation, PricingError> {
        let catalog_actor = self.authorize_quotation(actor)?;
        if input
            .lines
            .iter()
            .any(|line| line.configuration.selection.target.scope() != &catalog_actor.scope)
        {
            return Err(PricingError::Denied);
        }
        let result = self.store.transact_pricing(false, |tx| {
            let mut result = QuotationEvaluation {
                scope: actor.scope.clone(),
                customer: None,
                lines: vec![],
                currency: "YER".into(),
                discount_basis_points: input.discount_basis_points,
                totals: None,
                errors: vec![],
                server_available: false,
            };
            evaluate_customer(tx, &actor.scope, input.customer.as_ref(), &mut result)?;
            if input.discount_basis_points > 10_000 {
                error(
                    &mut result,
                    None,
                    Field::DiscountBasisPoints,
                    Issue::Invalid,
                );
            }
            if input.lines.is_empty() || input.lines.len() > 1000 {
                error(&mut result, None, Field::Lines, Issue::Invalid);
                return Ok::<_, PricingError>(result);
            }
            let mut ids = HashSet::new();
            for line in &input.lines {
                if line.id.is_nil() || !ids.insert(line.id) {
                    error(
                        &mut result,
                        Some(line.id),
                        Field::LineId,
                        if line.id.is_nil() {
                            Issue::Invalid
                        } else {
                            Issue::Duplicate
                        },
                    );
                    continue;
                }
                let target = &line.configuration.selection.target;
                let (_, item_id, variant_id) = target.identity();
                if target.schema_version() != 1
                    || target.revision() == 0
                    || item_id.is_nil()
                    || variant_id.is_nil()
                {
                    error(&mut result, Some(line.id), Field::Target, Issue::Invalid);
                    continue;
                }
                let entries = tx.sales_item(&catalog_actor.scope, target)?;
                let Some(entry) = entries.into_iter().find(|e| e.price.target == *target) else {
                    error(&mut result, Some(line.id), Field::Target, Issue::Stale);
                    continue;
                };
                match validate_configuration(entry, &line.configuration) {
                    Ok(checked) => result.lines.push(project_line(line, checked)),
                    Err((field, cause)) => error(
                        &mut result,
                        Some(line.id),
                        field,
                        match cause {
                            PricingError::Conflict { .. } => Issue::Stale,
                            PricingError::Reference => Issue::Unavailable,
                            _ if field == Field::Total => Issue::Overflow,
                            _ => Issue::Invalid,
                        },
                    ),
                }
            }
            if result.errors.is_empty() {
                match discount(&CalculateDiscount {
                    line_totals_yer: result.lines.iter().map(|l| l.price.total_yer).collect(),
                    discount_basis_points: input.discount_basis_points,
                }) {
                    Ok(totals) => result.totals = Some(totals),
                    Err(_) => error(&mut result, None, Field::Total, Issue::Overflow),
                }
            }
            Ok(result)
        })?;
        self.authorize_quotation(actor)?;
        Ok(result)
    }

    /// Both scopes are derived from authenticated identity, never from client totals or role flags.
    /// # Errors
    /// Rejects actors without access to both the exact branch and its tenant catalog.
    pub fn authorize_quotation(
        &self,
        actor: &AuthorizationContext,
    ) -> Result<AuthorizationContext, PricingError> {
        if actor.scope.kind.as_str() != "branch" {
            return Err(PricingError::Denied);
        }
        self.authorization
            .authorize(actor, CUSTOMER_READ_PERMISSION)?;
        let catalog_actor = AuthorizationContext {
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").map_err(|_| PricingError::Unconfirmed)?,
                id: ScopeId::new(actor.tenant_id.value()),
            },
            ..actor.clone()
        };
        self.require(&catalog_actor, CATALOG_READ_PERMISSION)?;
        Ok(catalog_actor)
    }
}

fn error(
    result: &mut QuotationEvaluation,
    line_id: Option<uuid::Uuid>,
    field: Field,
    issue: Issue,
) {
    result.errors.push(QuotationFieldError {
        line_id,
        field,
        issue,
    });
}

fn customer_snapshot(customer: eitmad_contracts::customer::Customer) -> QuotationCustomerSnapshot {
    QuotationCustomerSnapshot {
        id: customer.id,
        revision: customer.revision,
        name: customer.name.as_str().into(),
        phone: customer.phone.as_str().into(),
        address: customer.address.map(|a| a.as_str().into()),
    }
}

fn project_line(
    line: &eitmad_contracts::quotation::QuotationLineIntent,
    checked: eitmad_contracts::sales_catalog::SalesConfiguration,
) -> EvaluatedQuotationLine {
    let selection = &line.configuration.selection;
    let option_name = |options: &[eitmad_contracts::furniture::FurnitureOption], id| {
        options
            .iter()
            .find(|o| Some(o.id) == id)
            .map(|o| o.name.clone())
    };
    EvaluatedQuotationLine {
        id: line.id,
        color_name: option_name(&checked.entry.colors, selection.color_id),
        handle_name: option_name(&checked.entry.handles, selection.handle_id),
        name: checked.entry.name,
        description: checked.entry.description,
        variant_name: checked.entry.variant_name,
        color_id: selection.color_id,
        handle_id: selection.handle_id,
        dimensions: checked.dimensions,
        quantity: selection.quantity,
        price: checked.price,
    }
}

fn evaluate_customer(
    tx: &eitmad_storage::PricingTransaction<'_>,
    scope: &ScopeRef,
    intent: Option<&eitmad_contracts::quotation::QuotationCustomerIntent>,
    result: &mut QuotationEvaluation,
) -> Result<(), PricingError> {
    match intent {
        None => error(result, None, Field::Customer, Issue::Required),
        Some(intent) => match tx.customer(scope, intent.id)? {
            Some(customer) if customer.status == CustomerStatus::Active => {
                if intent.revision == 0 || intent.revision != customer.revision {
                    error(result, None, Field::CustomerRevision, Issue::Stale);
                } else {
                    result.customer = Some(customer_snapshot(customer));
                }
            }
            _ => error(result, None, Field::Customer, Issue::Unavailable),
        },
    }
    Ok(())
}
