//! Scoped Furniture production definitions and immutable revisions.
use eitmad_authorization::{
    AuthorizationService, FURNITURE_READ_PERMISSION, FURNITURE_WRITE_PERMISSION, MutationContext,
};
use eitmad_contracts::{
    events::Event,
    furniture::{
        CheckFurnitureSelection, Furniture, FurnitureCategories, FurnitureCategory,
        FurnitureCategoryId, FurnitureChangeNotice, FurnitureId, FurnitureOption, FurniturePage,
        FurnitureReview, FurnitureSelection, FurnitureState, FurnitureVariant,
        GetFurnitureRevision, ListFurnitureCategories, ListFurnitures, SaveFurniture,
        SaveFurnitureCategory,
    },
    identity::{AuthorizationContext, ScopeRef},
};
use eitmad_material::normalize_search;
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use eitmad_storage::{
    AuthorityStore, DurableIdempotency, DurablePublication, FurnitureRecord, FurnitureTransaction,
    StorageError,
};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FurnitureError {
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
impl From<StorageError> for FurnitureError {
    /// Converts storage failures to a redacted unavailable result at the domain boundary.
    fn from(_: StorageError) -> Self {
        Self::Unavailable
    }
}

#[derive(Clone, Debug)]
pub struct FurnitureService {
    store: AuthorityStore,
    authorization: AuthorizationService,
}
impl FurnitureService {
    /// Composes scoped storage and authorization without granting access until each operation is checked.
    #[must_use]
    pub const fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self {
            store,
            authorization,
        }
    }
    /// Requires organization scope and an authorized furniture-read relationship.
    fn read(&self, context: &AuthorizationContext) -> Result<(), FurnitureError> {
        if context.scope.kind.as_str() != "organization" {
            return Err(FurnitureError::Denied);
        }
        self.authorization
            .authorize(context, FURNITURE_READ_PERMISSION)
            .map_err(map_authorization)
    }
    /// Requires Manager write permission and records denied attempts without furniture payloads.
    fn write(&self, context: &MutationContext, operation: &str) -> Result<(), FurnitureError> {
        match self
            .authorization
            .authorize(&context.authorization, FURNITURE_WRITE_PERMISSION)
        {
            Ok(()) => Ok(()),
            Err(e) => {
                let error = map_authorization(e);
                if error == FurnitureError::Denied {
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
    /// Saves a separate furniture category using an audited revision check.
    /// # Errors
    /// Rejects denied, invalid, duplicate, stale, or unavailable operations.
    pub fn save_category(
        &self,
        context: &MutationContext,
        input: &SaveFurnitureCategory,
    ) -> Result<FurnitureCategory, FurnitureError> {
        const OP: &str = "eitmad.furniture-category.save.v1";
        self.write(context, OP)?;
        validate_text(&input.name, 120, false)?;
        validate_identity(input.id.is_some(), input.expected_revision, input.archived)?;
        if input.id.is_some_and(|id| id.value().is_nil()) {
            return Err(FurnitureError::Invalid);
        }
        let scope = &context.authorization.scope;
        let retry = retry(context, OP, input)?;
        self.store
            .transact_furnitures(|tx| -> Result<_, FurnitureError> {
                if let Some(result) = replay(tx, scope, &retry)? {
                    return Ok(result);
                }
                let id = input
                    .id
                    .unwrap_or_else(|| FurnitureCategoryId::new(Uuid::new_v4()));
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
                    return reject(tx, context, OP, id.value(), FurnitureError::Invalid);
                }
                let record = FurnitureCategory {
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
                    FurnitureRecord::Category(&record),
                    &normalized,
                    OP,
                    &evidence,
                    &retry,
                    &publication,
                )?;
                Ok(Ok(record))
            })?
    }

    /// Checks the exact scoped Furniture asset before retaining a new definition revision.
    fn validate_image(
        &self,
        context: &MutationContext,
        input: &SaveFurniture,
    ) -> Result<(), FurnitureError> {
        eitmad_catalog_image::CatalogImageService::new(
            self.store.clone(),
            self.authorization.clone(),
        )
        .attach(
            &context.authorization,
            eitmad_contracts::catalog_image::CatalogImageKind::Furniture,
            input.image.as_deref(),
        )
        .map_err(|e| match e {
            eitmad_catalog_image::ImageError::Denied => FurnitureError::Denied,
            eitmad_catalog_image::ImageError::Unavailable => FurnitureError::Unavailable,
            _ => FurnitureError::InvalidReference,
        })
    }

    /// Saves one audited definition revision. Removed options become archived, never deleted.
    /// # Errors
    /// Rejects denied, invalid, stale, cross-scope, or unavailable saves.
    pub fn save(
        &self,
        context: &MutationContext,
        input: &SaveFurniture,
    ) -> Result<Furniture, FurnitureError> {
        const OP: &str = "eitmad.furniture.save.v1";
        self.write(context, OP)?;
        self.validate_image(context, input)?;
        validate_text(&input.name, 160, false)?;
        validate_description(&input.description)?;
        validate_description(&input.notes)?;
        validate_identity(
            input.id.is_some(),
            input.expected_revision,
            input.state == FurnitureState::Archived,
        )?;
        if input.id.is_some_and(|id| id.value().is_nil()) {
            return Err(FurnitureError::Invalid);
        }
        if input.variants.is_empty() || input.variants.len() > 100 {
            return Err(FurnitureError::Invalid);
        }
        let scope = &context.authorization.scope;
        let retry = retry(context, OP, input)?;
        self.store
            .transact_furnitures(|tx| -> Result<_, FurnitureError> {
                if let Some(result) = replay(tx, scope, &retry)? {
                    return Ok(result);
                }
                let id = input.id.unwrap_or_else(|| FurnitureId::new(Uuid::new_v4()));
                let previous = tx.furniture(scope, id.value())?;
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
                let Some(category) = tx.category(scope, input.category_id.value())? else {
                    return reject(
                        tx,
                        context,
                        OP,
                        id.value(),
                        FurnitureError::InvalidReference,
                    );
                };
                if category.archived
                    && previous
                        .as_ref()
                        .is_none_or(|p| p.category_id != category.id)
                {
                    return reject(
                        tx,
                        context,
                        OP,
                        id.value(),
                        FurnitureError::InvalidReference,
                    );
                }
                let variants = match validate_variants(tx, scope, id, input, previous.as_ref()) {
                    Ok(variants) => variants,
                    Err(error) => return reject(tx, context, OP, id.value(), error),
                };
                let reviewed = match validate_composition(tx, scope, input, previous.as_ref()) {
                    Ok(value) => value,
                    Err(error) => return reject(tx, context, OP, id.value(), error),
                };
                let revision = next_revision(actual)?;
                let record = definition(
                    context,
                    input,
                    (id, revision),
                    category,
                    variants,
                    previous.as_ref(),
                    reviewed.parts_cost_yer,
                );
                let search = search_name(&record);
                let mut evidence = audit(context, OP, Some(id.value()));
                evidence.previous_revision = actual;
                evidence.resulting_revision = Some(revision);
                if input.confirm_below_cost {
                    evidence
                        .changed_identifiers
                        .push("below-cost-confirmed".into());
                }
                tx.persist(
                    FurnitureRecord::Furniture(&record),
                    &search,
                    OP,
                    &evidence,
                    &retry,
                    &publication(context, id.value(), revision, false),
                )?;
                Ok(Ok(record))
            })?
    }
    /// Lists bounded current definitions. Selection projections exclude archived options.
    /// # Errors
    /// Rejects unauthorized, unbounded, or unavailable reads.
    pub fn list(
        &self,
        context: &AuthorizationContext,
        query: &ListFurnitures,
    ) -> Result<FurniturePage, FurnitureError> {
        self.read(context)?;
        if !(1..=100).contains(&query.limit) {
            return Err(FurnitureError::Invalid);
        }
        validate_text(&query.term, 256, true)?;
        let costs = true;
        let can_manage = match self
            .authorization
            .authorize(context, FURNITURE_WRITE_PERMISSION)
        {
            Ok(()) => true,
            Err(eitmad_authorization::AuthorizationError::Denied) => false,
            Err(e) => return Err(map_authorization(e)),
        };
        self.store
            .read_furnitures(|tx| -> Result<_, FurnitureError> {
                let mut items = tx.list(
                    &context.scope,
                    &normalize_search(&query.term),
                    query.selectable_only,
                    query.after,
                    query.limit,
                )?;
                let next = if items.len() > query.limit as usize {
                    items.truncate(query.limit as usize);
                    items.last().map(|p| p.id)
                } else {
                    None
                };
                for item in &mut items {
                    if query.selectable_only {
                        item.variants.retain(|v| !v.archived);
                    }
                }
                Ok(FurniturePage {
                    items,
                    next,
                    can_manage,
                    can_read_costs: costs,
                })
            })
    }
    /// Resolves immutable history; new selections require current active references.
    /// # Errors
    /// Rejects invalid scope/schema, missing references, and archived or stale new selections.
    pub fn revision(
        &self,
        context: &AuthorizationContext,
        query: &GetFurnitureRevision,
    ) -> Result<Furniture, FurnitureError> {
        self.read(context)?;
        let r = &query.reference;
        if r.scope != context.scope
            || r.schema_version != 1
            || r.revision == 0
            || i64::try_from(r.revision).is_err()
        {
            return Err(FurnitureError::InvalidReference);
        }
        self.store
            .read_furnitures(|tx| -> Result<_, FurnitureError> {
                let record = tx.revision(r)?.ok_or(FurnitureError::NotFound)?;
                let variant = record
                    .variants
                    .iter()
                    .find(|v| v.id == r.variant_id)
                    .ok_or(FurnitureError::InvalidReference)?;
                if query.for_new_work {
                    let current = tx
                        .furniture(&context.scope, r.furniture_id.value())?
                        .ok_or(FurnitureError::NotFound)?;
                    let category = tx
                        .category(&context.scope, current.category_id.value())?
                        .ok_or(FurnitureError::InvalidReference)?;
                    if current.revision != r.revision
                        || current.state != FurnitureState::Active
                        || category.archived
                        || variant.archived
                    {
                        return Err(FurnitureError::InvalidReference);
                    }
                }

                Ok(record)
            })
    }
    /// Lists separate organization-scoped categories.
    /// # Errors
    /// Rejects denied, unbounded, or unavailable reads.
    pub fn categories(
        &self,
        context: &AuthorizationContext,
        query: &ListFurnitureCategories,
    ) -> Result<FurnitureCategories, FurnitureError> {
        self.read(context)?;
        if !(1..=100).contains(&query.limit) {
            return Err(FurnitureError::Invalid);
        }
        self.store
            .read_furnitures(|tx| -> Result<_, FurnitureError> {
                let mut items = tx.categories(&context.scope, query.after, query.limit)?;
                let next = if items.len() > query.limit as usize {
                    items.truncate(query.limit as usize);
                    items.last().map(|c| c.id)
                } else {
                    None
                };
                Ok(FurnitureCategories { items, next })
            })
    }
}
/// Checks byte bounds, required content, whitespace, and unsafe direction controls without rewriting text.
fn validate_text(value: &str, max: usize, empty: bool) -> Result<(), FurnitureError> {
    if (!empty && value.is_empty())
        || value.len() > max
        || value.trim() != value
        || value.chars().any(|c| {
            c.is_control()
                || matches!(c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'|'\u{200e}'|'\u{200f}')
        })
    {
        Err(FurnitureError::Invalid)
    } else {
        Ok(())
    }
}
/// Requires ID and expected revision together for updates and rejects archived creates.
fn validate_identity(
    id: bool,
    expected: Option<u64>,
    archived: bool,
) -> Result<(), FurnitureError> {
    if id != expected.is_some() || expected == Some(0) || !id && archived {
        Err(FurnitureError::Invalid)
    } else {
        Ok(())
    }
}
/// Advances a revision within the positive signed `SQLite` integer range.
fn next_revision(actual: Option<u64>) -> Result<u64, FurnitureError> {
    actual
        .unwrap_or(0)
        .checked_add(1)
        .filter(|v| i64::try_from(*v).is_ok())
        .ok_or(FurnitureError::Invalid)
}
/// Preserves expected and actual revisions for typed conflict reporting.
fn conflict(expected: Option<u64>, actual: Option<u64>) -> FurnitureError {
    FurnitureError::RevisionConflict { expected, actual }
}
/// Preserves explicit denials while redacting authorization-store failures as unavailable.
fn map_authorization(e: eitmad_authorization::AuthorizationError) -> FurnitureError {
    match e {
        eitmad_authorization::AuthorizationError::Denied
        | eitmad_authorization::AuthorizationError::UnsupportedScope => FurnitureError::Denied,
        _ => FurnitureError::Unavailable,
    }
}
/// Binds the durable retry hash to the actor, operation, and exact serialized request.
fn retry(
    context: &MutationContext,
    operation: &str,
    input: &impl serde::Serialize,
) -> Result<DurableIdempotency, FurnitureError> {
    let request_hash = Sha256::digest(
        serde_json::to_vec(&(
            context.authorization.identity.principal_id,
            operation,
            input,
        ))
        .map_err(|_| FurnitureError::Unavailable)?,
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
    tx: &FurnitureTransaction<'_>,
    scope: &ScopeRef,
    retry: &DurableIdempotency,
) -> Result<Option<Result<T, FurnitureError>>, FurnitureError> {
    tx.replay(scope, retry)?
        .map(|stored| {
            if stored.request_hash == retry.request_hash {
                serde_json::from_slice(&stored.response_json)
                    .map(Ok)
                    .map_err(|_| FurnitureError::Unavailable)
            } else {
                Ok(Err(FurnitureError::Invalid))
            }
        })
        .transpose()
}
/// Records rejection or conflict evidence within the transaction before returning the domain error.
fn reject<T>(
    tx: &FurnitureTransaction<'_>,
    context: &MutationContext,
    operation: &str,
    id: Uuid,
    error: FurnitureError,
) -> Result<Result<T, FurnitureError>, FurnitureError> {
    let (outcome, code) = match error {
        FurnitureError::RevisionConflict { .. } => (
            AuditOutcome::Conflict,
            "eitmad.error.furniture-revision-conflict.v1",
        ),
        FurnitureError::InvalidReference => (
            AuditOutcome::Invalid,
            "eitmad.error.furniture-reference-invalid.v1",
        ),
        _ => (AuditOutcome::Invalid, "eitmad.error.furniture-invalid.v1"),
    };
    tx.audit(&audit(context, operation, Some(id)).with_outcome(outcome, Some(code.into())))?;
    Ok(Err(error))
}
/// Builds redacted mutation evidence with scope, actor, causation, and retry correlation.
fn audit(context: &MutationContext, operation: &str, id: Option<Uuid>) -> MutationAuditRecord {
    let mut record = MutationAuditRecord::from_authorization(
        &context.authorization,
        context.occurred_at,
        context.correlation_id,
        operation,
        AuditTarget {
            kind: "furniture-definition".into(),
            identifiers: id.map(|v| vec![v.to_string()]).unwrap_or_default(),
        },
    );
    record.causation_id = context.causation_id;
    record.idempotency_key = Some(context.idempotency_key);
    record.changed_identifiers = vec!["definition".into()];
    record
}
/// Builds a compact scoped notice without furniture names, notes, or costs.
fn publication(
    context: &MutationContext,
    id: Uuid,
    revision: u64,
    category: bool,
) -> DurablePublication {
    DurablePublication {
        event: Event::FurnitureChanged(FurnitureChangeNotice {
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

/// Allows multiline descriptions while rejecting excessive size and unsafe direction controls.
fn validate_description(value: &str) -> Result<(), FurnitureError> {
    if value.len() > 4096
        || value.chars().any(|c| {
            c.is_control() && !matches!(c, '\n' | '\r' | '\t')
                || matches!(c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'|'\u{200e}'|'\u{200f}')
        })
    {
        Err(FurnitureError::Invalid)
    } else {
        Ok(())
    }
}

/// Missing options stay archived so their identities cannot be recycled.
fn retain_options(input: &[FurnitureOption], old: &[FurnitureOption]) -> Vec<FurnitureOption> {
    let mut result = input.to_vec();
    for option in old {
        if !input.iter().any(|v| v.id == option.id) {
            let mut v = option.clone();
            v.archived = true;
            result.push(v);
        }
    }
    result
}
/// Validates fixed sizes, explicit customization bounds, stable identities, and compatible option references.
fn validate_variants(
    tx: &FurnitureTransaction<'_>,
    scope: &ScopeRef,
    id: FurnitureId,
    input: &SaveFurniture,
    previous: Option<&Furniture>,
) -> Result<Vec<FurnitureVariant>, FurnitureError> {
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    let mut result = Vec::new();
    for v in &input.variants {
        validate_text(&v.name, 120, false)?;
        if v.id.value().is_nil()
            || !ids.insert(v.id)
            || !v.archived && !names.insert(normalize_search(&v.name))
            || v.selling_price_yer < 0
        {
            return Err(FurnitureError::Invalid);
        }
        validate_dimensions(&v.dimensions)?;
        if let Some(c) = &v.customization {
            validate_dimensions(&c.minimum)?;
            validate_dimensions(&c.maximum)?;
            if !within(&v.dimensions, &c.minimum, &c.maximum) {
                return Err(FurnitureError::Invalid);
            }
        }
        if tx
            .option_owner(scope, v.id.value())?
            .is_some_and(|owner| owner != id.value())
        {
            return Err(FurnitureError::InvalidReference);
        }
        let old = previous.and_then(|p| p.variants.iter().find(|old| old.id == v.id));
        if tx.option_owner(scope, v.id.value())?.is_some() && old.is_none() {
            return Err(FurnitureError::InvalidReference);
        }
        if old.is_some_and(|old| old.archived && !v.archived) || old.is_none() && v.archived {
            return Err(FurnitureError::InvalidReference);
        }
        for (allowed, options) in [
            (&v.color_ids, &input.colors),
            (&v.handle_ids, &input.handles),
        ] {
            if allowed.len() > 100
                || allowed
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != allowed.len()
                || allowed
                    .iter()
                    .any(|id| !options.iter().any(|o| o.id == *id))
            {
                return Err(FurnitureError::InvalidReference);
            }
            if !v.archived
                && !options.is_empty()
                && !options
                    .iter()
                    .any(|o| !o.archived && (allowed.is_empty() || allowed.contains(&o.id)))
            {
                return Err(FurnitureError::InvalidReference);
            }
        }
        result.push(v.clone());
    }
    if let Some(old) = previous {
        for v in &old.variants {
            if !ids.contains(&v.id) {
                let mut v = v.clone();
                v.archived = true;
                result.push(v);
            }
        }
    }
    if result.len() > 100
        || input.state != FurnitureState::Archived && result.iter().all(|v| v.archived)
    {
        return Err(FurnitureError::Invalid);
    }
    Ok(result)
}
/// Physical sizes are positive integer millimetres with a bounded storage and UI range.
fn validate_dimensions(
    d: &eitmad_contracts::furniture::FurnitureDimensions,
) -> Result<(), FurnitureError> {
    if [d.width_mm, d.height_mm, d.depth_mm]
        .iter()
        .all(|v| (1..=100_000).contains(v))
    {
        Ok(())
    } else {
        Err(FurnitureError::Invalid)
    }
}
/// Checks each dimension against its inclusive permitted bounds.
fn within(
    d: &eitmad_contracts::furniture::FurnitureDimensions,
    min: &eitmad_contracts::furniture::FurnitureDimensions,
    max: &eitmad_contracts::furniture::FurnitureDimensions,
) -> bool {
    [
        (d.width_mm, min.width_mm, max.width_mm),
        (d.height_mm, min.height_mm, max.height_mm),
        (d.depth_mm, min.depth_mm, max.depth_mm),
    ]
    .iter()
    .all(|(v, min, max)| min <= v && v <= max)
}

/// Checks dimensions against the same fixed-size or inclusive customization rule as manager selections.
/// # Errors
/// Rejects nonpositive, oversized, or unpermitted dimensions.
pub fn validate_selection_dimensions(
    dimensions: &eitmad_contracts::furniture::FurnitureDimensions,
    fixed: &eitmad_contracts::furniture::FurnitureDimensions,
    customization: Option<&eitmad_contracts::furniture::FurnitureCustomization>,
) -> Result<(), FurnitureError> {
    validate_dimensions(dimensions)?;
    if customization.map_or(dimensions != fixed, |c| {
        !within(dimensions, &c.minimum, &c.maximum)
    }) {
        return Err(FurnitureError::Invalid);
    }
    Ok(())
}
/// Calculates exact count-based Part costs using immutable composition references on the save transaction.
fn validate_composition(
    tx: &FurnitureTransaction<'_>,
    scope: &ScopeRef,
    input: &SaveFurniture,
    previous: Option<&Furniture>,
) -> Result<FurnitureReview, FurnitureError> {
    if input.parts.is_empty()
        || input.parts.len() > 100
        || input.colors.len() > 100
        || input.handles.len() > 100
    {
        return Err(FurnitureError::Invalid);
    }
    let mut ids = std::collections::HashSet::new();
    let mut rows = Vec::new();
    let mut total = 0i64;
    for usage in &input.parts {
        let r = &usage.reference;
        if usage.quantity == 0
            || usage.quantity > 1_000_000
            || r.scope != *scope
            || r.schema_version != 1
            || r.revision == 0
            || !ids.insert(r.part_id)
        {
            return Err(FurnitureError::InvalidReference);
        }
        let part = tx
            .part(scope, r.part_id.value())?
            .ok_or(FurnitureError::InvalidReference)?;
        let unchanged = previous.is_some_and(|p| p.parts.iter().any(|old| old.reference == *r));
        if !unchanged && (part.archived || part.revision != r.revision) {
            return Err(FurnitureError::InvalidReference);
        }
        let snapshot = tx.composition(r)?.ok_or(FurnitureError::InvalidReference)?;
        let cost = snapshot
            .cost
            .total_cost_yer
            .checked_mul(i64::from(usage.quantity))
            .ok_or(FurnitureError::Invalid)?;
        total = total.checked_add(cost).ok_or(FurnitureError::Invalid)?;
        rows.push(cost);
    }
    validate_options(tx, scope, input, previous)?;
    for v in input.variants.iter().filter(|v| !v.archived) {
        let color = input
            .colors
            .iter()
            .filter(|o| !o.archived && (v.color_ids.is_empty() || v.color_ids.contains(&o.id)))
            .map(|o| o.price_adjustment_yer)
            .max()
            .unwrap_or(0);
        let handle = input
            .handles
            .iter()
            .filter(|o| !o.archived && (v.handle_ids.is_empty() || v.handle_ids.contains(&o.id)))
            .map(|o| o.price_adjustment_yer)
            .max()
            .unwrap_or(0);
        v.selling_price_yer
            .checked_add(color)
            .and_then(|n| n.checked_add(handle))
            .ok_or(FurnitureError::Invalid)?;
        if input.state == FurnitureState::Active
            && (v.selling_price_yer <= 0
                || v.selling_price_yer < total && !input.confirm_below_cost)
        {
            return Err(FurnitureError::Invalid);
        }
    }
    Ok(FurnitureReview {
        parts_cost_yer: total,
        row_costs_yer: rows,
        margins_yer: input
            .variants
            .iter()
            .map(|v| {
                v.selling_price_yer
                    .checked_sub(total)
                    .ok_or(FurnitureError::Invalid)
            })
            .collect::<Result<Vec<_>, _>>()?,
    })
}

/// Checks stable option identities, visuals, archived history, and money inputs.
fn validate_options(
    tx: &FurnitureTransaction<'_>,
    scope: &ScopeRef,
    input: &SaveFurniture,
    previous: Option<&Furniture>,
) -> Result<(), FurnitureError> {
    let mut all_ids: std::collections::HashSet<_> =
        input.variants.iter().map(|v| v.id.value()).collect();
    for (options, old) in [
        (
            &input.colors,
            previous.map(|p| p.colors.as_slice()).unwrap_or_default(),
        ),
        (
            &input.handles,
            previous.map(|p| p.handles.as_slice()).unwrap_or_default(),
        ),
    ] {
        let mut names = std::collections::HashSet::new();
        for o in options {
            let owner = tx.option_owner(scope, o.id)?;
            if owner.is_some() && !old.iter().any(|v| v.id == o.id) {
                return Err(FurnitureError::InvalidReference);
            }
            if owner.is_some_and(|id| Some(FurnitureId::new(id)) != input.id) {
                return Err(FurnitureError::InvalidReference);
            }
            if previous.is_some_and(|p| p.variants.iter().any(|v| v.id.value() == o.id)) {
                return Err(FurnitureError::InvalidReference);
            }
            validate_text(&o.name, 120, false)?;
            validate_text(&o.visual, 32, false)?;
            if o.id.is_nil()
                || !all_ids.insert(o.id)
                || !o.archived && !names.insert(normalize_search(&o.name))
                || o.price_adjustment_yer < 0
            {
                return Err(FurnitureError::Invalid);
            }
            if old.iter().any(|v| v.id == o.id && v.archived) && !o.archived {
                return Err(FurnitureError::InvalidReference);
            }
        }
        if retain_options(options, old).len() > 100 {
            return Err(FurnitureError::Invalid);
        }
    }
    for color in &input.colors {
        if color.visual.len() != 7
            || !color.visual.starts_with('#')
            || !color.visual[1..].bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err(FurnitureError::Invalid);
        }
    }
    for handle in &input.handles {
        if !["Standard", "BlackMetal", "Brass"].contains(&handle.visual.as_str()) {
            return Err(FurnitureError::Invalid);
        }
    }
    Ok(())
}

impl FurnitureService {
    /// Reviews unsaved data in Rust without changing durable state.
    /// # Errors
    /// Rejects unauthorized, invalid, stale, or unavailable definitions.
    pub fn review(
        &self,
        context: &AuthorizationContext,
        input: &SaveFurniture,
    ) -> Result<FurnitureReview, FurnitureError> {
        self.read(context)?;
        validate_text(&input.name, 160, false)?;
        validate_description(&input.description)?;
        validate_description(&input.notes)?;
        validate_identity(
            input.id.is_some(),
            input.expected_revision,
            input.state == FurnitureState::Archived,
        )?;
        if input.variants.len() > 100 {
            return Err(FurnitureError::Invalid);
        }
        self.store.read_furnitures(|tx| {
            let old = match input.id {
                Some(id) => tx.furniture(&context.scope, id.value())?,
                None => None,
            };
            if old.as_ref().map(|p| p.revision) != input.expected_revision {
                return Err(conflict(
                    input.expected_revision,
                    old.as_ref().map(|p| p.revision),
                ));
            }
            let category = tx
                .category(&context.scope, input.category_id.value())?
                .ok_or(FurnitureError::InvalidReference)?;
            if category.archived && old.as_ref().is_none_or(|p| p.category_id != category.id) {
                return Err(FurnitureError::InvalidReference);
            }
            if !input.variants.is_empty() {
                validate_variants(
                    tx,
                    &context.scope,
                    input.id.unwrap_or_else(|| FurnitureId::new(Uuid::nil())),
                    input,
                    old.as_ref(),
                )?;
            }
            validate_composition(tx, &context.scope, input, old.as_ref())
        })
    }
    /// Validates a prospective selection against the current active manager definition and its permitted bounds.
    /// # Errors
    /// Rejects stale, archived, incompatible, or overflowing selections.
    pub fn selection(
        &self,
        context: &AuthorizationContext,
        input: &CheckFurnitureSelection,
    ) -> Result<FurnitureSelection, FurnitureError> {
        let definition = self.revision(
            context,
            &GetFurnitureRevision {
                reference: input.reference.clone(),
                for_new_work: true,
            },
        )?;
        let v = definition
            .variants
            .iter()
            .find(|v| v.id == input.reference.variant_id)
            .ok_or(FurnitureError::InvalidReference)?;
        if input.quantity == 0 || input.quantity > 1_000_000 {
            return Err(FurnitureError::Invalid);
        }
        validate_selection_dimensions(&input.dimensions, &v.dimensions, v.customization.as_ref())?;
        let mut price = v.selling_price_yer;
        for (selected, options, allowed) in [
            (input.color_id, &definition.colors, &v.color_ids),
            (input.handle_id, &definition.handles, &v.handle_ids),
        ] {
            let compatible: Vec<_> = options
                .iter()
                .filter(|o| !o.archived && (allowed.is_empty() || allowed.contains(&o.id)))
                .collect();
            match selected {
                Some(id) => {
                    let option = compatible
                        .iter()
                        .find(|o| o.id == id)
                        .ok_or(FurnitureError::InvalidReference)?;
                    price = price
                        .checked_add(option.price_adjustment_yer)
                        .ok_or(FurnitureError::Invalid)?;
                }
                None if !compatible.is_empty() => return Err(FurnitureError::InvalidReference),
                None => {}
            }
        }
        let total = price
            .checked_mul(i64::from(input.quantity))
            .ok_or(FurnitureError::Invalid)?;
        Ok(FurnitureSelection {
            definition,
            unit_price_yer: price,
            total_yer: total,
        })
    }
}

/// Builds one immutable definition from the validated request and retained archived options.
fn definition(
    context: &MutationContext,
    input: &SaveFurniture,
    identity: (FurnitureId, u64),
    category: FurnitureCategory,
    variants: Vec<FurnitureVariant>,
    previous: Option<&Furniture>,
    parts_cost_yer: i64,
) -> Furniture {
    let (id, revision) = identity;
    Furniture {
        image: input.image.clone(),
        id,
        scope: context.authorization.scope.clone(),
        name: input.name.clone(),
        category_id: category.id,
        category_name: category.name,
        description: input.description.clone(),
        notes: input.notes.clone(),
        variants,
        parts: input.parts.clone(),
        colors: retain_options(
            &input.colors,
            previous.map(|p| p.colors.as_slice()).unwrap_or_default(),
        ),
        handles: retain_options(
            &input.handles,
            previous.map(|p| p.handles.as_slice()).unwrap_or_default(),
        ),
        state: input.state,
        parts_cost_yer,
        revision,
        updated_at: context.occurred_at,
    }
}

/// Indexes the current definition name and its fixed variant names for Arabic search.
fn search_name(record: &Furniture) -> String {
    normalize_search(&format!(
        "{} {}",
        record.name,
        record
            .variants
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    ))
}
