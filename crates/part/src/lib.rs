use eitmad_authorization::{
    AuthorizationService, MutationContext, PART_READ_PERMISSION, PART_WRITE_PERMISSION,
};
use eitmad_contracts::{
    events::Event,
    identity::{AuthorizationContext, ScopeRef},
    material::MaterialQuantity,
    part::{
        CalculatePartCost, CompositionReference, CostedUsage, GetPartComposition,
        ListPartCategories, ListParts, Part, PartCategories, PartCategory, PartCategoryId,
        PartChangeNotice, PartCost, PartId, PartPage, PartProjection, PartUsage, SavePart,
        SavePartCategory,
    },
};
use eitmad_material::normalize_search;
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use eitmad_storage::{
    AuthorityStore, DurableIdempotency, DurablePublication, PartRecord, PartTransaction,
    StorageError,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{ToPrimitive as _, Zero as _};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartError {
    Denied,
    Invalid,
    InvalidReference,
    NotFound,
    RevisionConflict {
        expected: Option<u64>,
        actual: Option<u64>,
    },
    Unavailable,
}
impl From<StorageError> for PartError {
    fn from(_: StorageError) -> Self {
        Self::Unavailable
    }
}

#[derive(Clone, Debug)]
pub struct PartService {
    store: AuthorityStore,
    authorization: AuthorizationService,
}
impl PartService {
    #[must_use]
    pub const fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self {
            store,
            authorization,
        }
    }
    fn read(&self, context: &AuthorizationContext) -> Result<(), PartError> {
        if context.scope.kind.as_str() != "organization" {
            return Err(PartError::Denied);
        }
        self.authorization
            .authorize(context, PART_READ_PERMISSION)
            .map_err(map_authorization)
    }
    fn write(&self, context: &MutationContext, operation: &str) -> Result<(), PartError> {
        if context.authorization.scope.kind.as_str() != "organization" {
            return Err(PartError::Denied);
        }
        match self
            .authorization
            .authorize(&context.authorization, PART_WRITE_PERMISSION)
        {
            Ok(()) => Ok(()),
            Err(e) => {
                let error = map_authorization(e);
                if error == PartError::Denied {
                    self.store
                        .append_audit(&audit(context, operation, None).with_outcome(
                            AuditOutcome::Denied,
                            Some("eitmad.error.authorization-denied.v1".into()),
                        ))?;
                }
                Err(error)
            }
        }
    }
    /// Saves a separate part category using an audited revision check.
    /// # Errors
    /// Rejects denied, invalid, duplicate, stale, or unavailable operations.
    pub fn save_category(
        &self,
        context: &MutationContext,
        input: &SavePartCategory,
    ) -> Result<PartCategory, PartError> {
        const OP: &str = "eitmad.part-category.save.v1";
        self.write(context, OP)?;
        validate_text(&input.name, 120, false)?;
        validate_identity(input.id.is_some(), input.expected_revision, input.archived)?;
        let scope = &context.authorization.scope;
        let retry = retry(context, OP, input)?;
        self.store.transact_parts(|tx| -> Result<_, PartError> {
            if let Some(result) = replay(tx, scope, &retry)? {
                return Ok(result);
            }
            let id = input
                .id
                .unwrap_or_else(|| PartCategoryId::new(Uuid::new_v4()));
            let actual = tx.category(scope, id.value())?.map(|c| c.revision);
            if actual != input.expected_revision {
                return reject(
                    tx,
                    context,
                    OP,
                    id.value(),
                    conflict(input.expected_revision, actual),
                );
            }
            let normalized = normalize_search(&input.name);
            if tx.category_name_exists(scope, id, &normalized)? {
                return reject(tx, context, OP, id.value(), PartError::Invalid);
            }
            let record = PartCategory {
                id,
                scope: scope.clone(),
                name: input.name.clone(),
                archived: input.archived,
                revision: next_revision(actual)?,
                updated_at: context.occurred_at,
            };
            let publication = publication(context, id.value(), record.revision, true);
            let mut evidence = audit(context, OP, Some(id.value()));
            evidence.previous_revision = actual;
            evidence.resulting_revision = Some(record.revision);
            tx.persist(
                PartRecord::Category(&record),
                &normalized,
                OP,
                &evidence,
                &retry,
                &publication,
            )?;
            Ok(Ok(record))
        })?
    }
    /// Saves a part and its immutable material/unit/cost snapshot atomically.
    /// # Errors
    /// Rejects invalid composition, stale part or material revisions, denied access, or mandatory write failure.
    pub fn save(&self, context: &MutationContext, input: &SavePart) -> Result<Part, PartError> {
        const OP: &str = "eitmad.part.save.v1";
        self.write(context, OP)?;
        validate_text(&input.name, 160, false)?;
        validate_description(&input.description)?;
        validate_identity(input.id.is_some(), input.expected_revision, input.archived)?;
        let scope = &context.authorization.scope;
        let retry = retry(context, OP, input)?;
        self.store.transact_parts(|tx| -> Result<_, PartError> {
            if let Some(result) = replay(tx, scope, &retry)? {
                return Ok(result);
            }
            let id = input.id.unwrap_or_else(|| PartId::new(Uuid::new_v4()));
            let previous = tx.part(scope, id.value())?;
            let actual = previous.as_ref().map(|p| p.revision);
            if actual != input.expected_revision {
                return reject(
                    tx,
                    context,
                    OP,
                    id.value(),
                    conflict(input.expected_revision, actual),
                );
            }
            let category = tx.category(scope, input.category_id.value())?;
            if category.is_none_or(|c| {
                c.archived && previous.as_ref().is_none_or(|p| p.category_id != c.id)
            }) {
                return reject(tx, context, OP, id.value(), PartError::InvalidReference);
            }
            let cost = match calculate(tx, scope, &input.usages, true, previous.as_ref()) {
                Ok(c) => c,
                Err(e) => return reject(tx, context, OP, id.value(), e),
            };
            let revision = next_revision(actual)?;
            let record = Part {
                id,
                scope: scope.clone(),
                name: input.name.clone(),
                category_id: input.category_id,
                description: input.description.clone(),
                archived: input.archived,
                revision,
                updated_at: context.occurred_at,
                composition: CompositionReference {
                    scope: scope.clone(),
                    part_id: id,
                    revision,
                    schema_version: 1,
                },
                cost,
            };
            let mut evidence = audit(context, OP, Some(id.value()));
            evidence.previous_revision = actual;
            evidence.resulting_revision = Some(revision);
            tx.persist(
                PartRecord::Part(&record),
                &normalize_search(&record.name),
                OP,
                &evidence,
                &retry,
                &publication(context, id.value(), revision, false),
            )?;
            Ok(Ok(record))
        })?
    }
    /// Calculates a reviewed composition using exact conversion and current revision checks.
    /// # Errors
    /// Rejects invalid, inactive, cross-scope, cross-dimension, or stale references.
    pub fn cost(
        &self,
        context: &AuthorizationContext,
        query: &CalculatePartCost,
    ) -> Result<PartCost, PartError> {
        self.read(context)?;
        self.store.transact_parts(|tx| {
            let previous = query
                .part_id
                .map(|id| tx.part(&context.scope, id.value()))
                .transpose()?
                .flatten();
            if query.part_id.is_some() && previous.is_none() {
                return Err(PartError::NotFound);
            }
            calculate(tx, &context.scope, &query.usages, true, previous.as_ref())
        })
    }
    /// Returns current cost projections without altering saved composition snapshots.
    /// # Errors
    /// Rejects unauthorized or unbounded queries and invalid current cost references.
    pub fn list(
        &self,
        context: &AuthorizationContext,
        query: &ListParts,
    ) -> Result<PartPage, PartError> {
        self.read(context)?;
        validate_query(query)?;
        self.store.transact_parts(|tx| -> Result<_, PartError> {
            let mut items = tx.list(
                &context.scope,
                &normalize_search(&query.term),
                query.after,
                query.limit,
            )?;
            let next = if items.len() > query.limit as usize {
                items.truncate(query.limit as usize);
                items.last().map(|p| p.id)
            } else {
                None
            };
            let items = items
                .into_iter()
                .map(|part| {
                    let usages = part
                        .cost
                        .rows
                        .iter()
                        .map(|r| r.usage.clone())
                        .collect::<Vec<_>>();
                    let current_cost = calculate(tx, &context.scope, &usages, false, Some(&part))?;
                    Ok(PartProjection { part, current_cost })
                })
                .collect::<Result<Vec<_>, PartError>>()?;
            Ok(PartPage { items, next })
        })
    }
    /// Loads the immutable composition used by a historical commercial reference.
    /// # Errors
    /// Rejects denied, foreign-scope, unsupported, or missing references.
    pub fn composition(
        &self,
        context: &AuthorizationContext,
        query: &GetPartComposition,
    ) -> Result<Part, PartError> {
        self.read(context)?;
        if query.reference.scope != context.scope
            || query.reference.schema_version != 1
            || query.reference.revision == 0
        {
            return Err(PartError::InvalidReference);
        }
        self.store
            .transact_parts(|tx| tx.composition(&query.reference)?.ok_or(PartError::NotFound))
    }
    /// Reads scoped part categories, distinct from material categories.
    /// # Errors
    /// Rejects denied reads or unavailable data.
    pub fn categories(
        &self,
        context: &AuthorizationContext,
        query: &ListPartCategories,
    ) -> Result<PartCategories, PartError> {
        self.read(context)?;
        if !(1..=100).contains(&query.limit) {
            return Err(PartError::Invalid);
        }
        self.store.transact_parts(|tx| {
            let mut items = tx.categories(&context.scope, query.after, query.limit)?;
            let next = if items.len() > query.limit as usize {
                items.truncate(query.limit as usize);
                items.last().map(|c| c.id)
            } else {
                None
            };
            Ok(PartCategories { items, next })
        })
    }
}

/// Validates scoped references and reviewed revisions, retains existing archived references, and sums exact rational costs.
fn calculate(
    tx: &PartTransaction<'_>,
    scope: &ScopeRef,
    usages: &[PartUsage],
    strict: bool,
    previous: Option<&Part>,
) -> Result<PartCost, PartError> {
    if usages.is_empty() || usages.len() > 100 {
        return Err(PartError::Invalid);
    }
    let mut seen = std::collections::HashSet::new();
    let mut rows = Vec::new();
    let mut total = BigRational::zero();
    for usage in usages {
        if !seen.insert(usage.material_id) {
            return Err(PartError::Invalid);
        }
        let material = tx
            .material(scope, usage.material_id.value())?
            .ok_or(PartError::InvalidReference)?;
        let unit = tx
            .unit(scope, usage.unit_id.value())?
            .ok_or(PartError::InvalidReference)?;
        let cost_unit = tx
            .unit(scope, material.unit_id.value())?
            .ok_or(PartError::InvalidReference)?;
        let retained = previous.is_some_and(|p| {
            p.cost.rows.iter().any(|r| {
                r.usage.material_id == usage.material_id && r.usage.unit_id == usage.unit_id
            })
        });
        if strict && !retained && (material.archived || unit.archived || cost_unit.archived)
            || unit.dimension != cost_unit.dimension
        {
            return Err(PartError::InvalidReference);
        }
        if strict && material.revision != usage.material_revision {
            return Err(conflict(
                Some(usage.material_revision),
                Some(material.revision),
            ));
        }
        if strict && unit.revision != usage.unit_revision {
            return Err(conflict(Some(usage.unit_revision), Some(unit.revision)));
        }
        if material.current_cost_yer < 0
            || unit.numerator == 0
            || unit.denominator == 0
            || cost_unit.numerator == 0
            || cost_unit.denominator == 0
        {
            return Err(PartError::InvalidReference);
        }
        let exact = usage_cost(usage, &material, &unit, &cost_unit)?;
        let cost_yer = round(&exact)?;
        total += exact;
        let mut usage = usage.clone();
        usage.material_revision = material.revision;
        usage.unit_revision = unit.revision;
        rows.push(CostedUsage {
            usage,
            material,
            unit,
            cost_unit,
            cost_yer,
        });
    }
    Ok(PartCost {
        rows,
        total_cost_yer: round(&total)?,
    })
}
/// Checks immutable Part costs using the same exact arithmetic as local composition.
/// # Errors
/// Rejects inconsistent references, incompatible units, altered row costs, and overflow.
pub fn verify_snapshot_cost(cost: &PartCost) -> Result<(), PartError> {
    if cost.rows.is_empty() || cost.rows.len() > 100 {
        return Err(PartError::Invalid);
    }
    let mut total = BigRational::zero();
    let mut seen = std::collections::HashSet::new();
    for row in &cost.rows {
        if !seen.insert(row.usage.material_id)
            || row.material.id != row.usage.material_id
            || row.material.revision != row.usage.material_revision
            || row.unit.id != row.usage.unit_id
            || row.unit.revision != row.usage.unit_revision
            || row.cost_unit.id != row.material.unit_id
        {
            return Err(PartError::InvalidReference);
        }
        let exact = usage_cost(&row.usage, &row.material, &row.unit, &row.cost_unit)?;
        if row.cost_yer != round(&exact)? {
            return Err(PartError::Invalid);
        }
        total += exact;
    }
    if cost.total_cost_yer != round(&total)? {
        return Err(PartError::Invalid);
    }
    Ok(())
}

/// Computes one exact rational cost; both live reviews and immutable verification use it.
fn usage_cost(
    usage: &PartUsage,
    material: &eitmad_contracts::material::Material,
    unit: &eitmad_contracts::material::MaterialUnit,
    cost_unit: &eitmad_contracts::material::MaterialUnit,
) -> Result<BigRational, PartError> {
    if material.current_cost_yer < 0
        || unit.dimension != cost_unit.dimension
        || unit.numerator == 0
        || unit.denominator == 0
        || cost_unit.numerator == 0
        || cost_unit.denominator == 0
    {
        return Err(PartError::InvalidReference);
    }
    Ok(quantity(&usage.quantity)?
        * BigRational::new(
            BigInt::from(unit.numerator) * BigInt::from(cost_unit.denominator),
            BigInt::from(unit.denominator) * BigInt::from(cost_unit.numerator),
        )
        * BigInt::from(material.current_cost_yer))
}
/// Converts validated fixed-point quantity text to an exact rational without floating-point loss.
fn quantity(value: &MaterialQuantity) -> Result<BigRational, PartError> {
    if value.as_str().len() > 32 {
        return Err(PartError::Invalid);
    }
    let (whole, fraction) = value
        .as_str()
        .split_once('.')
        .unwrap_or((value.as_str(), ""));
    let digits = format!("{whole}{fraction}");
    let numerator = BigInt::parse_bytes(digits.as_bytes(), 10).ok_or(PartError::Invalid)?;
    Ok(BigRational::new(
        numerator,
        BigInt::from(10u64.pow(u32::try_from(fraction.len()).map_err(|_| PartError::Invalid)?)),
    ))
}
/// Rounds a nonnegative cost half upward once and rejects amounts outside signed whole-YER money.
fn round(value: &BigRational) -> Result<i64, PartError> {
    (value + BigRational::new(BigInt::from(1), BigInt::from(2)))
        .to_integer()
        .to_i64()
        .ok_or(PartError::Invalid)
}
fn validate_text(value: &str, max: usize, empty: bool) -> Result<(), PartError> {
    if (!empty && value.is_empty())
        || value.len() > max
        || value.trim() != value
        || value.chars().any(|c| {
            c.is_control()
                || matches!(c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'|'\u{200e}'|'\u{200f}')
        })
    {
        Err(PartError::Invalid)
    } else {
        Ok(())
    }
}
fn validate_query(query: &ListParts) -> Result<(), PartError> {
    if !(1..=100).contains(&query.limit) {
        return Err(PartError::Invalid);
    }
    validate_text(&query.term, 256, true)
}
/// Requires ID and expected revision together for updates and rejects archived creates.
fn validate_identity(id: bool, expected: Option<u64>, archived: bool) -> Result<(), PartError> {
    if id != expected.is_some() || expected == Some(0) || !id && archived {
        Err(PartError::Invalid)
    } else {
        Ok(())
    }
}
/// Advances a revision within the positive signed `SQLite` integer range.
fn next_revision(actual: Option<u64>) -> Result<u64, PartError> {
    actual
        .unwrap_or(0)
        .checked_add(1)
        .filter(|v| i64::try_from(*v).is_ok())
        .ok_or(PartError::Invalid)
}
fn conflict(expected: Option<u64>, actual: Option<u64>) -> PartError {
    PartError::RevisionConflict { expected, actual }
}
fn map_authorization(e: eitmad_authorization::AuthorizationError) -> PartError {
    match e {
        eitmad_authorization::AuthorizationError::Denied
        | eitmad_authorization::AuthorizationError::UnsupportedScope => PartError::Denied,
        _ => PartError::Unavailable,
    }
}
/// Binds the durable retry hash to the actor, operation, and exact serialized request.
fn retry(
    context: &MutationContext,
    operation: &str,
    input: &impl serde::Serialize,
) -> Result<DurableIdempotency, PartError> {
    let request_hash = Sha256::digest(
        serde_json::to_vec(&(
            context.authorization.identity.principal_id,
            operation,
            input,
        ))
        .map_err(|_| PartError::Unavailable)?,
    )
    .into();
    Ok(DurableIdempotency {
        key: context.idempotency_key,
        request_hash,
        response_json: Vec::new(),
    })
}
/// Returns the original scoped result for an exact retry and rejects reuse with different input or actor.
fn replay<T: serde::de::DeserializeOwned>(
    tx: &PartTransaction<'_>,
    scope: &ScopeRef,
    retry: &DurableIdempotency,
) -> Result<Option<Result<T, PartError>>, PartError> {
    tx.replay(scope, retry)?
        .map(|stored| {
            if stored.request_hash == retry.request_hash {
                serde_json::from_slice(&stored.response_json)
                    .map(Ok)
                    .map_err(|_| PartError::Unavailable)
            } else {
                Ok(Err(PartError::Invalid))
            }
        })
        .transpose()
}
/// Records rejection or conflict evidence within the transaction before returning the domain error.
fn reject<T>(
    tx: &PartTransaction<'_>,
    context: &MutationContext,
    operation: &str,
    id: Uuid,
    error: PartError,
) -> Result<Result<T, PartError>, PartError> {
    let (outcome, code) = match error {
        PartError::RevisionConflict { .. } => (
            AuditOutcome::Conflict,
            "eitmad.error.part-revision-conflict.v1",
        ),
        PartError::InvalidReference => (
            AuditOutcome::Invalid,
            "eitmad.error.part-reference-invalid.v1",
        ),
        _ => (AuditOutcome::Invalid, "eitmad.error.part-invalid.v1"),
    };
    tx.audit(&audit(context, operation, Some(id)).with_outcome(outcome, Some(code.into())))?;
    Ok(Err(error))
}
fn audit(context: &MutationContext, operation: &str, id: Option<Uuid>) -> MutationAuditRecord {
    let mut record = MutationAuditRecord::from_authorization(
        &context.authorization,
        context.occurred_at,
        context.correlation_id,
        operation,
        AuditTarget {
            kind: "part-definition".into(),
            identifiers: id.map(|v| vec![v.to_string()]).unwrap_or_default(),
        },
    );
    record.causation_id = context.causation_id;
    record.idempotency_key = Some(context.idempotency_key);
    record.changed_identifiers = vec!["composition".into()];
    record
}
fn publication(
    context: &MutationContext,
    id: Uuid,
    revision: u64,
    category: bool,
) -> DurablePublication {
    DurablePublication {
        event: Event::PartChanged(PartChangeNotice {
            scope: context.authorization.scope.clone(),
            id,
            revision,
            category,
            changed_at: context.occurred_at,
        }),
        policy_changed: false,
    }
}

#[cfg(test)]
mod tests;

fn validate_description(value: &str) -> Result<(), PartError> {
    if value.len() > 4096
        || value.chars().any(|c| {
            c.is_control() && !matches!(c, '\n' | '\r' | '\t')
                || matches!(c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'|'\u{200e}'|'\u{200f}')
        })
    {
        Err(PartError::Invalid)
    } else {
        Ok(())
    }
}
