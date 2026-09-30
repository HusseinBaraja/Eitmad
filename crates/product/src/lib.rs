//! Ready-made product definitions and immutable supplier-variant revisions.
use eitmad_authorization::{
    AuthorizationService, MutationContext, PRODUCT_COST_READ_PERMISSION, PRODUCT_READ_PERMISSION,
    PRODUCT_WRITE_PERMISSION,
};
use eitmad_contracts::{
    events::Event,
    identity::{AuthorizationContext, ScopeRef},
    product::{
        GetProductRevision, ListProductCategories, ListProducts, Product, ProductCategories,
        ProductCategory, ProductCategoryId, ProductChangeNotice, ProductId, ProductPage,
        ProductVariant, SaveProduct, SaveProductCategory,
    },
};
use eitmad_material::normalize_search;
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use eitmad_storage::{
    AuthorityStore, DurableIdempotency, DurablePublication, ProductRecord, ProductTransaction,
    StorageError,
};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductError {
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
impl From<StorageError> for ProductError {
    /// Converts storage failures to a redacted unavailable result at the domain boundary.
    fn from(_: StorageError) -> Self {
        Self::Unavailable
    }
}

#[derive(Clone, Debug)]
pub struct ProductService {
    store: AuthorityStore,
    authorization: AuthorizationService,
}
impl ProductService {
    /// Composes scoped storage and authorization without granting access until each operation is checked.
    #[must_use]
    pub const fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self {
            store,
            authorization,
        }
    }
    /// Requires organization scope and an authorized product-read relationship.
    fn read(&self, context: &AuthorizationContext) -> Result<(), ProductError> {
        if context.scope.kind.as_str() != "organization" {
            return Err(ProductError::Denied);
        }
        self.authorization
            .authorize(context, PRODUCT_READ_PERMISSION)
            .map_err(map_authorization)
    }
    /// Requires Manager write permission and records denied attempts without product payloads.
    fn write(&self, context: &MutationContext, operation: &str) -> Result<(), ProductError> {
        match self
            .authorization
            .authorize(&context.authorization, PRODUCT_WRITE_PERMISSION)
        {
            Ok(()) => Ok(()),
            Err(e) => {
                let error = map_authorization(e);
                if error == ProductError::Denied {
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
    /// Saves a separate product category using an audited revision check.
    /// # Errors
    /// Rejects denied, invalid, duplicate, stale, or unavailable operations.
    pub fn save_category(
        &self,
        context: &MutationContext,
        input: &SaveProductCategory,
    ) -> Result<ProductCategory, ProductError> {
        const OP: &str = "eitmad.product-category.save.v1";
        self.write(context, OP)?;
        validate_text(&input.name, 120, false)?;
        validate_identity(input.id.is_some(), input.expected_revision, input.archived)?;
        if input.id.is_some_and(|id| id.value().is_nil()) {
            return Err(ProductError::Invalid);
        }
        let scope = &context.authorization.scope;
        let retry = retry(context, OP, input)?;
        self.store
            .transact_products(|tx| -> Result<_, ProductError> {
                if let Some(result) = replay(tx, scope, &retry)? {
                    return Ok(result);
                }
                let id = input
                    .id
                    .unwrap_or_else(|| ProductCategoryId::new(Uuid::new_v4()));
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
                    return reject(tx, context, OP, id.value(), ProductError::Invalid);
                }
                let record = ProductCategory {
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
                    ProductRecord::Category(&record),
                    &normalized,
                    OP,
                    &evidence,
                    &retry,
                    &publication,
                )?;
                Ok(Ok(record))
            })?
    }

    /// Saves one audited definition revision. Removed options become archived, never deleted.
    /// # Errors
    /// Rejects denied, invalid, stale, cross-scope, or unavailable saves.
    pub fn save(
        &self,
        context: &MutationContext,
        input: &SaveProduct,
    ) -> Result<Product, ProductError> {
        const OP: &str = "eitmad.product.save.v1";
        self.write(context, OP)?;
        // A writer must also be allowed to receive the internal-cost save result.
        self.authorization
            .authorize(&context.authorization, PRODUCT_COST_READ_PERMISSION)
            .map_err(map_authorization)?;
        validate_text(&input.name, 160, false)?;
        validate_description(&input.description)?;
        validate_description(&input.notes)?;
        validate_identity(input.id.is_some(), input.expected_revision, input.archived)?;
        if input.id.is_some_and(|id| id.value().is_nil()) {
            return Err(ProductError::Invalid);
        }
        if input.variants.is_empty() || input.variants.len() > 100 {
            return Err(ProductError::Invalid);
        }
        let scope = &context.authorization.scope;
        let retry = retry(context, OP, input)?;
        self.store
            .transact_products(|tx| -> Result<_, ProductError> {
                if let Some(result) = replay(tx, scope, &retry)? {
                    return Ok(result);
                }
                let id = input.id.unwrap_or_else(|| ProductId::new(Uuid::new_v4()));
                let previous = tx.product(scope, id.value())?;
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
                    return reject(tx, context, OP, id.value(), ProductError::InvalidReference);
                };
                if category.archived
                    && previous
                        .as_ref()
                        .is_none_or(|p| p.category_id != category.id)
                {
                    return reject(tx, context, OP, id.value(), ProductError::InvalidReference);
                }
                let variants = match validate_variants(tx, scope, id, input, previous.as_ref()) {
                    Ok(variants) => variants,
                    Err(error) => return reject(tx, context, OP, id.value(), error),
                };
                let revision = next_revision(actual)?;
                let record = Product {
                    id,
                    scope: scope.clone(),
                    name: input.name.clone(),
                    category_id: category.id,
                    category_name: category.name,
                    description: input.description.clone(),
                    notes: input.notes.clone(),
                    variants,
                    archived: input.archived,
                    revision,
                    updated_at: context.occurred_at,
                };
                let search = normalize_search(&format!(
                    "{} {}",
                    record.name,
                    record
                        .variants
                        .iter()
                        .map(|v| v.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
                let mut evidence = audit(context, OP, Some(id.value()));
                evidence.previous_revision = actual;
                evidence.resulting_revision = Some(revision);
                tx.persist(
                    ProductRecord::Product(&record),
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
        query: &ListProducts,
    ) -> Result<ProductPage, ProductError> {
        self.read(context)?;
        if !(1..=100).contains(&query.limit) {
            return Err(ProductError::Invalid);
        }
        validate_text(&query.term, 256, true)?;
        let costs = self.can_read_costs(context)?;
        let can_manage = match self
            .authorization
            .authorize(context, PRODUCT_WRITE_PERMISSION)
        {
            Ok(()) => true,
            Err(eitmad_authorization::AuthorizationError::Denied) => false,
            Err(e) => return Err(map_authorization(e)),
        };
        self.store
            .transact_products(|tx| -> Result<_, ProductError> {
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
                    redact(item, costs);
                }
                Ok(ProductPage {
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
        query: &GetProductRevision,
    ) -> Result<Product, ProductError> {
        self.read(context)?;
        let r = &query.reference;
        if r.scope != context.scope
            || r.schema_version != 1
            || r.revision == 0
            || i64::try_from(r.revision).is_err()
        {
            return Err(ProductError::InvalidReference);
        }
        let costs = self.can_read_costs(context)?;
        self.store
            .transact_products(|tx| -> Result<_, ProductError> {
                let mut record = tx.revision(r)?.ok_or(ProductError::NotFound)?;
                let variant = record
                    .variants
                    .iter()
                    .find(|v| v.id == r.variant_id)
                    .ok_or(ProductError::InvalidReference)?;
                if query.for_new_work {
                    let current = tx
                        .product(&context.scope, r.product_id.value())?
                        .ok_or(ProductError::NotFound)?;
                    let category = tx
                        .category(&context.scope, current.category_id.value())?
                        .ok_or(ProductError::InvalidReference)?;
                    if current.revision != r.revision
                        || current.archived
                        || category.archived
                        || variant.archived
                    {
                        return Err(ProductError::InvalidReference);
                    }
                }
                redact(&mut record, costs);
                Ok(record)
            })
    }
    /// Lists separate organization-scoped categories.
    /// # Errors
    /// Rejects denied, unbounded, or unavailable reads.
    pub fn categories(
        &self,
        context: &AuthorizationContext,
        query: &ListProductCategories,
    ) -> Result<ProductCategories, ProductError> {
        self.read(context)?;
        if !(1..=100).contains(&query.limit) {
            return Err(ProductError::Invalid);
        }
        self.store
            .transact_products(|tx| -> Result<_, ProductError> {
                let mut items = tx.categories(&context.scope, query.after, query.limit)?;
                let next = if items.len() > query.limit as usize {
                    items.truncate(query.limit as usize);
                    items.last().map(|c| c.id)
                } else {
                    None
                };
                Ok(ProductCategories { items, next })
            })
    }
    fn can_read_costs(&self, context: &AuthorizationContext) -> Result<bool, ProductError> {
        match self
            .authorization
            .authorize(context, PRODUCT_COST_READ_PERMISSION)
        {
            Ok(()) => Ok(true),
            Err(eitmad_authorization::AuthorizationError::Denied) => Ok(false),
            Err(e) => Err(map_authorization(e)),
        }
    }
}
fn validate_text(value: &str, max: usize, empty: bool) -> Result<(), ProductError> {
    if (!empty && value.is_empty())
        || value.len() > max
        || value.trim() != value
        || value.chars().any(|c| {
            c.is_control()
                || matches!(c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'|'\u{200e}'|'\u{200f}')
        })
    {
        Err(ProductError::Invalid)
    } else {
        Ok(())
    }
}
/// Requires ID and expected revision together for updates and rejects archived creates.
fn validate_identity(id: bool, expected: Option<u64>, archived: bool) -> Result<(), ProductError> {
    if id != expected.is_some() || expected == Some(0) || !id && archived {
        Err(ProductError::Invalid)
    } else {
        Ok(())
    }
}
/// Advances a revision within the positive signed `SQLite` integer range.
fn next_revision(actual: Option<u64>) -> Result<u64, ProductError> {
    actual
        .unwrap_or(0)
        .checked_add(1)
        .filter(|v| i64::try_from(*v).is_ok())
        .ok_or(ProductError::Invalid)
}
/// Preserves expected and actual revisions for typed conflict reporting.
fn conflict(expected: Option<u64>, actual: Option<u64>) -> ProductError {
    ProductError::RevisionConflict { expected, actual }
}
/// Preserves explicit denials while redacting authorization-store failures as unavailable.
fn map_authorization(e: eitmad_authorization::AuthorizationError) -> ProductError {
    match e {
        eitmad_authorization::AuthorizationError::Denied
        | eitmad_authorization::AuthorizationError::UnsupportedScope => ProductError::Denied,
        _ => ProductError::Unavailable,
    }
}
/// Binds the durable retry hash to the actor, operation, and exact serialized request.
fn retry(
    context: &MutationContext,
    operation: &str,
    input: &impl serde::Serialize,
) -> Result<DurableIdempotency, ProductError> {
    let request_hash = Sha256::digest(
        serde_json::to_vec(&(
            context.authorization.identity.principal_id,
            operation,
            input,
        ))
        .map_err(|_| ProductError::Unavailable)?,
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
    tx: &ProductTransaction<'_>,
    scope: &ScopeRef,
    retry: &DurableIdempotency,
) -> Result<Option<Result<T, ProductError>>, ProductError> {
    tx.replay(scope, retry)?
        .map(|stored| {
            if stored.request_hash == retry.request_hash {
                serde_json::from_slice(&stored.response_json)
                    .map(Ok)
                    .map_err(|_| ProductError::Unavailable)
            } else {
                Ok(Err(ProductError::Invalid))
            }
        })
        .transpose()
}
/// Records rejection or conflict evidence within the transaction before returning the domain error.
fn reject<T>(
    tx: &ProductTransaction<'_>,
    context: &MutationContext,
    operation: &str,
    id: Uuid,
    error: ProductError,
) -> Result<Result<T, ProductError>, ProductError> {
    let (outcome, code) = match error {
        ProductError::RevisionConflict { .. } => (
            AuditOutcome::Conflict,
            "eitmad.error.product-revision-conflict.v1",
        ),
        ProductError::InvalidReference => (
            AuditOutcome::Invalid,
            "eitmad.error.product-reference-invalid.v1",
        ),
        _ => (AuditOutcome::Invalid, "eitmad.error.product-invalid.v1"),
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
            kind: "product-definition".into(),
            identifiers: id.map(|v| vec![v.to_string()]).unwrap_or_default(),
        },
    );
    record.causation_id = context.causation_id;
    record.idempotency_key = Some(context.idempotency_key);
    record.changed_identifiers = vec!["definition".into()];
    record
}
/// Builds a compact scoped notice without product names, notes, or costs.
fn publication(
    context: &MutationContext,
    id: Uuid,
    revision: u64,
    category: bool,
) -> DurablePublication {
    DurablePublication {
        event: Event::ProductChanged(ProductChangeNotice {
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
fn validate_description(value: &str) -> Result<(), ProductError> {
    if value.len() > 4096
        || value.chars().any(|c| {
            c.is_control() && !matches!(c, '\n' | '\r' | '\t')
                || matches!(c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'|'\u{200e}'|'\u{200f}')
        })
    {
        Err(ProductError::Invalid)
    } else {
        Ok(())
    }
}

/// Withholds internal fields from every read surface, including immutable historical revisions.
fn redact(record: &mut Product, costs: bool) {
    if !costs {
        record.notes.clear();
        for variant in &mut record.variants {
            variant.purchase_cost_yer = None;
        }
    }
}

/// Retains option identities and refuses reactivation or ownership transfer.
fn validate_variants(
    tx: &ProductTransaction<'_>,
    scope: &ScopeRef,
    id: ProductId,
    input: &SaveProduct,
    previous: Option<&Product>,
) -> Result<Vec<ProductVariant>, ProductError> {
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    let mut variants = Vec::new();
    for v in &input.variants {
        if validate_text(&v.name, 120, false).is_err()
            || v.id.value().is_nil()
            || v.purchase_cost_yer < 0
            || !ids.insert(v.id)
            || !v.archived && !names.insert(normalize_search(&v.name))
        {
            return Err(ProductError::Invalid);
        }
        if tx
            .variant_owner(scope, v.id.value())?
            .is_some_and(|owner| owner != id.value())
        {
            return Err(ProductError::InvalidReference);
        }
        let old = previous
            .as_ref()
            .and_then(|p| p.variants.iter().find(|old| old.id == v.id));
        // Archived options cannot silently be made selectable again.
        if old.is_some_and(|old| old.archived && !v.archived) || old.is_none() && v.archived {
            return Err(ProductError::InvalidReference);
        }
        variants.push(ProductVariant {
            id: v.id,
            name: v.name.clone(),
            purchase_cost_yer: Some(v.purchase_cost_yer),
            archived: v.archived,
        });
    }
    if let Some(old) = &previous {
        for variant in &old.variants {
            if !ids.contains(&variant.id) {
                let mut retained = variant.clone();
                retained.archived = true;
                variants.push(retained);
            }
        }
    }
    if variants.len() > 100 || !input.archived && variants.iter().all(|v| v.archived) {
        return Err(ProductError::Invalid);
    }
    Ok(variants)
}
