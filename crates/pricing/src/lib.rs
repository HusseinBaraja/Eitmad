//! Rust-authoritative prices with distinct ready-made and manufactured references.
mod catalog;
mod money;
mod quotation;
pub use quotation::validate_draft_snapshot;
mod draft_sync;
mod drafts;
pub use draft_sync::{QuotationDraftSyncCycle, QuotationDraftSyncError};
pub use drafts::{QUOTATION_DRAFT_SCHEMA, QuotationDraftError, QuotationDraftService};
mod sales_catalog;
pub use catalog::{
    catalog_dependencies, catalog_record_id, public_entry, publication_basis, revision_record_id,
    revision_schema, validate_catalog_revision, validate_server_proposal,
};
use eitmad_authorization::{
    AuthorizationError, AuthorizationService, CATALOG_READ_PERMISSION, MutationContext,
    PRICING_COST_READ_PERMISSION, PRICING_WRITE_PERMISSION,
};
use eitmad_contracts::{
    events::Event,
    furniture::{Furniture, FurnitureReference, FurnitureState},
    identity::AuthorizationContext,
    pricing::{
        CalculateDiscount, ConfirmPrice, DiscountTotal, ListPrices, PriceAdjustment,
        PriceChangeNotice, PriceItem, PricePage, PriceReview, PriceSelection, PriceSummary,
        PriceTarget, PublishPrice, PublishedPrice, PublishedPricePage, ReadPublishedPrices,
        ReviewPrice, SellingPrice,
    },
    product::{Product, ProductReference},
    transport::UnixMillis,
};
use eitmad_observability_audit::{AuditOutcome, AuditTarget, MutationAuditRecord};
use eitmad_storage::{
    AuthorityStore, DurableIdempotency, DurablePublication, PricingTransaction, StorageError,
};
pub use money::{discount, validate_publication};
use sha2::{Digest as _, Sha256};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PricingError {
    Denied,
    Invalid,
    Reference,
    Conflict {
        expected: Option<u64>,
        actual: Option<u64>,
    },
    BelowCost,
    Unconfirmed,
}
impl From<StorageError> for PricingError {
    fn from(_: StorageError) -> Self {
        Self::Unconfirmed
    }
}
impl From<AuthorizationError> for PricingError {
    fn from(e: AuthorizationError) -> Self {
        match e {
            AuthorizationError::Denied | AuthorizationError::UnsupportedScope => Self::Denied,
            _ => Self::Unconfirmed,
        }
    }
}

/// Authenticated server authority. Local IPC never accepts confirmation from the shell.
pub trait PriceConfirmation: Send + Sync {
    /// Synchronizes an audited catalog batch before confirming a new price intent.
    /// # Errors
    /// Rejects unauthorized, conflicting, invalid, or unavailable catalog revisions.
    fn synchronize_catalog(
        &self,
        actor: &AuthorizationContext,
        request: &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions,
        deadline: UnixMillis,
    ) -> Result<(), PricingError>;
    /// Reads a bounded page of current confirmed prices for the authorized organization.
    /// # Errors
    /// Rejects unauthorized or unavailable server reads.
    fn read(
        &self,
        actor: &AuthorizationContext,
        request: &ReadPublishedPrices,
        deadline: UnixMillis,
    ) -> Result<PublishedPricePage, PricingError>;
    /// Looks up the exact intent before a retry without creating a new price.
    /// # Errors
    /// Denies unrelated actors and retains unknown outcomes when status is unavailable.
    fn status(
        &self,
        actor: &AuthorizationContext,
        request: &ConfirmPrice,
        deadline: UnixMillis,
    ) -> Result<Option<PublishedPrice>, PricingError>;
    /// Publishes with server-side authorization, CAS, durable idempotency, and audit.
    /// # Errors
    /// An unknown outcome must remain unconfirmed and be retried with the same intent.
    fn confirm(
        &self,
        actor: &AuthorizationContext,
        request: &ConfirmPrice,
        deadline: UnixMillis,
    ) -> Result<PublishedPrice, PricingError>;
}
/// Runs catalog replication on a Rust worker through the existing authenticated sync transport.
pub trait CatalogReplication: Send + Sync {
    /// Transfers bounded private work and commits complete public read models.
    /// # Errors
    /// Retains work and checkpoints on interruption, denial, or missing dependencies.
    fn synchronize(
        &self,
        actor: &AuthorizationContext,
        deadline: UnixMillis,
    ) -> Result<usize, PricingError>;
}
#[derive(Clone)]
pub struct PricingService {
    store: AuthorityStore,
    authorization: AuthorizationService,
    confirmation: Option<Arc<dyn PriceConfirmation>>,
}
impl PricingService {
    #[must_use]
    pub const fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self {
            store,
            authorization,
            confirmation: None,
        }
    }
    #[must_use]
    pub fn with_confirmation(mut self, confirmation: Arc<dyn PriceConfirmation>) -> Self {
        self.confirmation = Some(confirmation);
        self
    }
    /// Refreshes confirmed public cache through an authenticated server projection.
    /// # Errors
    /// Fails closed for denied access and retains confirmed local data for transport failure.
    pub fn refresh(
        &self,
        context: &MutationContext,
        deadline: UnixMillis,
    ) -> Result<(), PricingError> {
        self.require(&context.authorization, CATALOG_READ_PERMISSION)?;
        let confirmation = self
            .confirmation
            .as_ref()
            .ok_or(PricingError::Unconfirmed)?;
        let mut after = None;
        loop {
            let page = confirmation.read(
                &context.authorization,
                &ReadPublishedPrices {
                    scope: context.authorization.scope.clone(),
                    after: after.clone(),
                    limit: 100,
                },
                deadline,
            )?;
            if page.items.len() > 100 || page.next == after && after.is_some() {
                return Err(PricingError::Unconfirmed);
            }
            self.require(&context.authorization, CATALOG_READ_PERMISSION)?;
            self.store
                .transact_pricing(true, |tx| -> Result<_, PricingError> {
                    for price in &page.items {
                        if price.target.scope() != &context.authorization.scope
                            || price.currency != "YER"
                            || price.revision == 0
                            || i64::try_from(price.revision).is_err()
                        {
                            return Err(PricingError::Unconfirmed);
                        }
                        validate_publication(&ConfirmPrice {
                            cost_yer: 0,
                            command: PublishPrice {
                                target: price.target.clone(),
                                expected_revision: None,
                                selling_price_yer: price.selling_price_yer,
                                confirm_below_cost: false,
                            },
                            colors: price.colors.clone(),
                            handles: price.handles.clone(),
                            idempotency_key: context.idempotency_key,
                        })?;
                        let latest = tx.latest(&price.target)?;
                        if latest
                            .as_ref()
                            .is_some_and(|p| p.revision == price.revision && p != price)
                        {
                            return Err(PricingError::Unconfirmed);
                        }
                        if latest.is_none_or(|p| p.revision < price.revision) {
                            let input = PublishPrice {
                                target: price.target.clone(),
                                expected_revision: None,
                                selling_price_yer: price.selling_price_yer,
                                confirm_below_cost: false,
                            };
                            let mut evidence = audit(context, &input);
                            evidence.operation = "eitmad.pricing.cache.refresh.v1".into();
                            evidence.resulting_revision = Some(price.revision);
                            evidence.changed_identifiers = vec!["confirmed-price-cache".into()];
                            tx.cache(price, &evidence)?;
                        }
                    }
                    Ok(())
                })?;
            after = page.next;
            if after.is_none() {
                break;
            }
        }
        Ok(())
    }
    fn require(&self, actor: &AuthorizationContext, permission: &str) -> Result<(), PricingError> {
        if actor.scope.kind.as_str() != "organization" {
            return Err(PricingError::Denied);
        }
        self.authorization
            .authorize(actor, permission)
            .map_err(Into::into)
    }
    fn allowed(
        &self,
        actor: &AuthorizationContext,
        permission: &str,
    ) -> Result<bool, PricingError> {
        match self.require(actor, permission) {
            Ok(()) => Ok(true),
            Err(PricingError::Denied) => Ok(false),
            Err(e) => Err(e),
        }
    }
    /// Lists only active current catalog references. Unpublished entries are Manager-only.
    /// # Errors
    /// Rejects denied, malformed, and unavailable queries.
    pub fn list(
        &self,
        actor: &AuthorizationContext,
        input: &ListPrices,
    ) -> Result<PricePage, PricingError> {
        self.require(actor, CATALOG_READ_PERMISSION)?;
        if !(1..=100).contains(&input.limit)
            || input.term.len() > 256
            || input.after.as_ref().is_some_and(|v| v.len() > 90)
        {
            return Err(PricingError::Invalid);
        }
        let can_manage = self.allowed(actor, PRICING_WRITE_PERMISSION)?;
        let costs = self.allowed(actor, PRICING_COST_READ_PERMISSION)?;
        let catalog_sync_issues = if can_manage {
            self.store.catalog_sync_issues(&actor.scope)?
        } else {
            vec![]
        };
        if !can_manage
            && self
                .store
                .catalog_checkpoint(
                    actor,
                    &eitmad_contracts::transport::SchemaId::parse(
                        "eitmad.schema.catalog-public.v1",
                    )
                    .map_err(|_| PricingError::Invalid)?,
                )?
                .is_some()
        {
            return self.public_list(actor, input);
        }
        self.store.transact_pricing(false, |tx| {
            // Fill the variant page across bounded definition batches, including filtered gaps.
            let after = input.after.as_deref().unwrap_or("");
            let catalog_after = if after.matches(':').count() == 2 {
                after
                    .rsplit_once(':')
                    .map_or(after.to_owned(), |(entry, _)| entry.to_owned())
            } else if after.is_empty() {
                String::new()
            } else {
                format!("{after}~")
            };
            let batch_limit = input.limit + 1;
            let mut cursor = catalog_after;
            let mut items = Vec::new();
            loop {
                let ids = tx.catalog_ids(&actor.scope, &cursor, batch_limit)?;
                if ids.is_empty() {
                    break;
                }
                for (kind, id) in &ids {
                    let mut variants = list_definition(
                        tx,
                        &actor.scope,
                        kind,
                        *id,
                        (can_manage, costs),
                        &input.term,
                    )?;
                    variants.retain(|item| item_cursor(&item.target).as_str() > after);
                    items.extend(variants);
                    if items.len() > input.limit as usize {
                        break;
                    }
                }
                if items.len() > input.limit as usize || ids.len() < batch_limit as usize {
                    break;
                }
                if let Some((kind, id)) = ids.last() {
                    cursor = format!("{kind}:{id}~");
                }
            }
            items.sort_by_key(|item| item_cursor(&item.target));
            let next = if items.len() > input.limit as usize {
                items.truncate(input.limit as usize);
                items.last().map(|item| item_cursor(&item.target))
            } else {
                None
            };
            Ok(PricePage {
                catalog_sync_issues,
                server_available: false,
                items,
                next,
                can_manage,
                can_read_costs: costs,
            })
        })
    }
    /// Searches and pages confirmed public sales without returning private costs or margins.
    /// The caller must authorize catalog access before reading the scoped cache.
    fn public_list(
        &self,
        actor: &AuthorizationContext,
        input: &ListPrices,
    ) -> Result<PricePage, PricingError> {
        let term = eitmad_material::normalize_search(&input.term);
        let mut entries = self.store.catalog_sales(&actor.scope)?;
        entries.sort_by_key(|entry| item_cursor(&entry.price.target));
        let mut items = entries
            .into_iter()
            .filter(|e| {
                item_cursor(&e.price.target) > input.after.clone().unwrap_or_default()
                    && eitmad_material::normalize_search(&format!(
                        "{} {} {}",
                        e.name, e.variant_name, e.category_name
                    ))
                    .contains(&term)
            })
            .take(input.limit as usize + 1)
            .map(|e| PriceItem {
                publication_required: false,
                target: e.price.target,
                name: e.name,
                variant_name: e.variant_name,
                category_name: e.category_name,
                published: Some(PriceSummary {
                    currency: e.price.currency,
                    selling_price_yer: e.price.selling_price_yer,
                    revision: e.price.revision,
                    confirmed_at: e.price.confirmed_at,
                }),
                cost_yer: None,
                margin_yer: None,
            })
            .collect::<Vec<_>>();
        let next = if items.len() > input.limit as usize {
            items.truncate(input.limit as usize);
            items.last().map(|i| item_cursor(&i.target))
        } else {
            None
        };
        Ok(PricePage {
            catalog_sync_issues: vec![],
            server_available: false,
            items,
            next,
            can_manage: false,
            can_read_costs: false,
        })
    }
    /// Reviews whole-rial draft prices with authoritative cost and margin; never publishes.
    /// # Errors
    /// Denies callers without cost permission and rejects stale or inactive references.
    pub fn review(
        &self,
        actor: &AuthorizationContext,
        input: &ReviewPrice,
    ) -> Result<PriceReview, PricingError> {
        self.require(actor, PRICING_COST_READ_PERMISSION)?;
        if input.selling_price_yer < 0 {
            return Err(PricingError::Invalid);
        }
        self.store.transact_pricing(false, |tx| {
            let source = source(tx, actor, &input.target)?;
            review_cost(input.selling_price_yer, source.cost())
        })
    }
    /// Evaluates the accepted 5.00% discount threshold in Rust.
    /// # Errors
    /// Rejects unauthorized, invalid, or overflowing calculations.
    pub fn calculate_discount(
        &self,
        actor: &AuthorizationContext,
        input: &CalculateDiscount,
    ) -> Result<DiscountTotal, PricingError> {
        self.require(actor, CATALOG_READ_PERMISSION)?;
        discount(input)
    }
    /// Publishes only after an exact authenticated server receipt; intents survive restart.
    /// # Errors
    /// Rejects denied, stale, below-cost-unconfirmed, invalid, and unavailable publications.
    pub fn publish(
        &self,
        context: &MutationContext,
        input: &PublishPrice,
        deadline: UnixMillis,
    ) -> Result<PublishedPrice, PricingError> {
        if let Err(e) = self.require(&context.authorization, PRICING_WRITE_PERMISSION) {
            self.store
                .append_audit(&audit(context, input).with_outcome(
                    AuditOutcome::Denied,
                    Some("eitmad.error.authorization-denied.v1".into()),
                ))?;
            return Err(e);
        }
        self.require(&context.authorization, PRICING_COST_READ_PERMISSION)?;
        let hash: [u8; 32] = Sha256::digest(
            serde_json::to_vec(&(context.authorization.identity.principal_id, input))
                .map_err(|_| PricingError::Invalid)?,
        )
        .into();
        let retry = DurableIdempotency {
            key: context.idempotency_key,
            request_hash: hash,
            response_json: vec![],
        };
        if let Some(record) =
            self.store
                .transact_pricing(false, |tx| -> Result<_, PricingError> {
                    Ok(tx.replay(&context.authorization.scope, &retry)?)
                })?
        {
            return if record.request_hash == hash {
                serde_json::from_slice(&record.response_json).map_err(|_| PricingError::Unconfirmed)
            } else {
                Err(PricingError::Invalid)
            };
        }
        let prepared = self.prepare(context, input, &hash)?;
        let receipt = self
            .confirmation
            .as_ref()
            .ok_or(PricingError::Unconfirmed)?
            .status(&context.authorization, &prepared, deadline)
            .and_then(|status| {
                status.map_or_else(
                    || {
                        let records = self.catalog_revisions(&prepared.command.target)?;
                        for records in records.chunks(8) {
                            self.confirmation.as_ref().ok_or(PricingError::Unconfirmed)?
                                .synchronize_catalog(
                                    &context.authorization,
                                    &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions {
                                        scope: context.authorization.scope.clone(),
                                        records: records.to_vec(),
                                    },
                                    deadline,
                                )?;
                        }
                        self.confirmation
                            .as_ref()
                            .ok_or(PricingError::Unconfirmed)?
                            .confirm(&context.authorization, &prepared, deadline)
                    },
                    Ok,
                )
            });
        let price = match receipt {
            Ok(price) => price,
            Err(e) => {
                if e != PricingError::Unconfirmed {
                    self.store
                        .transact_pricing(true, |tx| -> Result<_, PricingError> {
                            // Denial does not prove that a previous unknown request never committed.
                            if e != PricingError::Denied {
                                tx.resolve(&context.authorization.scope, &hash)?;
                            }
                            tx.audit(&audit(context, input).with_outcome(
                                if e == PricingError::Denied {
                                    AuditOutcome::Denied
                                } else {
                                    AuditOutcome::Conflict
                                },
                                Some(error_code(e).into()),
                            ))?;
                            Ok(())
                        })?;
                }
                return Err(e);
            }
        };
        if price.target != prepared.command.target
            || price.currency != "YER"
            || price.selling_price_yer != prepared.command.selling_price_yer
            || price.colors != prepared.colors
            || price.handles != prepared.handles
            || price.revision
                != prepared
                    .command
                    .expected_revision
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or(PricingError::Invalid)?
        {
            return Err(PricingError::Unconfirmed);
        }
        self.apply_receipt(context, input, price, &retry)
    }
    fn apply_receipt(
        &self,
        context: &MutationContext,
        input: &PublishPrice,
        price: PublishedPrice,
        retry: &DurableIdempotency,
    ) -> Result<PublishedPrice, PricingError> {
        // Reauthorization after network I/O prevents a late receipt from restoring revoked access.
        self.require(&context.authorization, PRICING_WRITE_PERMISSION)?;
        self.store
            .transact_pricing(true, |tx| -> Result<_, PricingError> {
                if let Some(latest) = tx.latest(&price.target)? {
                    if latest.revision == price.revision && latest != price {
                        return Err(PricingError::Conflict {
                            expected: input.expected_revision,
                            actual: Some(latest.revision),
                        });
                    }
                }
                let mut evidence = audit(context, input);
                evidence.previous_revision = input.expected_revision;
                evidence.resulting_revision = Some(price.revision);
                if input.confirm_below_cost {
                    evidence
                        .changed_identifiers
                        .push("below-cost-confirmed".into());
                }
                let publication = DurablePublication {
                    event: Event::PriceChanged(PriceChangeNotice {
                        target: price.target.clone(),
                        revision: price.revision,
                    }),
                    policy_changed: false,
                };
                tx.persist(&price, &evidence, retry, &publication)?;
                tx.resolve(&context.authorization.scope, &retry.request_hash)?;
                Ok(price)
            })
    }
    fn prepare(
        &self,
        context: &MutationContext,
        input: &PublishPrice,
        hash: &[u8; 32],
    ) -> Result<ConfirmPrice, PricingError> {
        self.store
            .transact_pricing(true, |tx| -> Result<_, PricingError> {
                if let Some(intent) = tx.intent(&context.authorization.scope, hash)? {
                    return Ok(Ok(intent));
                }
                let result = (|| {
                    let source = source(tx, &context.authorization, &input.target)?;
                    let actual = tx.latest(&input.target)?.map(|p| p.revision);
                    if actual != input.expected_revision {
                        return Err(PricingError::Conflict {
                            expected: input.expected_revision,
                            actual,
                        });
                    }
                    if input.selling_price_yer <= 0 {
                        return Err(PricingError::Invalid);
                    }
                    let reviewed = review_cost(input.selling_price_yer, source.cost())?;
                    if reviewed.below_cost && !input.confirm_below_cost {
                        return Err(PricingError::BelowCost);
                    }
                    let (colors, handles) = source.adjustments();
                    let request = ConfirmPrice {
                        cost_yer: source.cost(),
                        command: input.clone(),
                        colors,
                        handles,
                        idempotency_key: context.idempotency_key,
                    };
                    validate_publication(&request)?;
                    let mut evidence = audit(context, input);
                    evidence.changed_identifiers = vec!["publication-intent".into()];
                    tx.prepare(&request, hash, &evidence)?;
                    Ok(request)
                })();
                if let Err(e) = result {
                    tx.audit(&audit(context, input).with_outcome(
                        if matches!(e, PricingError::Conflict { .. }) {
                            AuditOutcome::Conflict
                        } else {
                            AuditOutcome::Invalid
                        },
                        Some(error_code(e).into()),
                    ))?;
                }
                Ok(result)
            })?
    }
    /// Returns only public prices with exact current references and compatible options.
    /// # Errors
    /// Rejects stale revisions, archive, incompatible options, fractional/zero quantity, and overflow.
    pub fn selection(
        &self,
        actor: &AuthorizationContext,
        input: &PriceSelection,
    ) -> Result<SellingPrice, PricingError> {
        self.require(actor, CATALOG_READ_PERMISSION)?;
        if self
            .store
            .catalog_checkpoint(
                actor,
                &eitmad_contracts::transport::SchemaId::parse("eitmad.schema.catalog-public.v1")
                    .map_err(|_| PricingError::Invalid)?,
            )?
            .is_some()
            && !self.allowed(actor, PRICING_WRITE_PERMISSION)?
        {
            let entry = self
                .store
                .catalog_sales(&actor.scope)?
                .into_iter()
                .find(|e| e.price.target == input.target)
                .ok_or(PricingError::Reference)?;
            let price = entry.price;
            if price.revision != input.price_revision || input.quantity == 0 {
                return Err(PricingError::Reference);
            }
            let color = adjustment(&price.colors, input.color_id)?;
            let handle = adjustment(&price.handles, input.handle_id)?;
            let unit = price
                .selling_price_yer
                .checked_add(color)
                .and_then(|v| v.checked_add(handle))
                .ok_or(PricingError::Invalid)?;
            let total = unit
                .checked_mul(i64::from(input.quantity))
                .ok_or(PricingError::Invalid)?;
            return Ok(SellingPrice {
                snapshot: price,
                unit_price_yer: unit,
                total_yer: total,
            });
        }
        self.store.transact_pricing(false, |tx| {
            let source = source(tx, actor, &input.target)?;
            let price = tx.latest(&input.target)?.ok_or(PricingError::Reference)?;
            if price.target != input.target
                || price.revision != input.price_revision
                || input.quantity == 0
            {
                return Err(PricingError::Reference);
            }
            source.check_options(input)?;
            let color = adjustment(&price.colors, input.color_id)?;
            let handle = adjustment(&price.handles, input.handle_id)?;
            let unit = price
                .selling_price_yer
                .checked_add(color)
                .and_then(|v| v.checked_add(handle))
                .ok_or(PricingError::Invalid)?;
            let total = unit
                .checked_mul(i64::from(input.quantity))
                .ok_or(PricingError::Invalid)?;
            Ok(SellingPrice {
                snapshot: price,
                unit_price_yer: unit,
                total_yer: total,
            })
        })
    }
}
fn list_definition(
    tx: &PricingTransaction<'_>,
    scope: &eitmad_contracts::identity::ScopeRef,
    kind: &str,
    id: uuid::Uuid,
    access: (bool, bool),
    term: &str,
) -> Result<Vec<PriceItem>, PricingError> {
    let mut items = Vec::new();
    if kind == "product" {
        let Some(p) = tx.products().product(scope, id)? else {
            return Ok(items);
        };
        if p.archived
            || tx
                .products()
                .category(scope, p.category_id.value())?
                .is_none_or(|c| c.archived)
        {
            return Ok(items);
        }
        for variant in &p.variants {
            if variant.archived {
                continue;
            }
            let target = PriceTarget::Product(ProductReference {
                scope: p.scope.clone(),
                product_id: p.id,
                variant_id: variant.id,
                revision: p.revision,
                schema_version: 1,
            });
            push_item(
                tx,
                &mut items,
                ItemDetails {
                    target,
                    name: &p.name,
                    variant: &variant.name,
                    category: &p.category_name,
                    cost: variant.purchase_cost_yer.ok_or(PricingError::Unconfirmed)?,
                },
                access,
                term,
            )?;
        }
    } else {
        let Some(f) = tx.furnitures().furniture(scope, id)? else {
            return Ok(items);
        };
        if f.state != FurnitureState::Active
            || tx
                .furnitures()
                .category(scope, f.category_id.value())?
                .is_none_or(|c| c.archived)
        {
            return Ok(items);
        }
        for variant in &f.variants {
            if variant.archived {
                continue;
            }
            let target = PriceTarget::Furniture(FurnitureReference {
                scope: f.scope.clone(),
                furniture_id: f.id,
                variant_id: variant.id,
                revision: f.revision,
                schema_version: 1,
            });
            push_item(
                tx,
                &mut items,
                ItemDetails {
                    target,
                    name: &f.name,
                    variant: &variant.name,
                    category: &f.category_name,
                    cost: f.parts_cost_yer,
                },
                access,
                term,
            )?;
        }
    }
    Ok(items)
}
enum Source {
    Product(Product, usize),
    Furniture(Furniture, usize),
}
impl Source {
    fn cost(&self) -> i64 {
        match self {
            Self::Product(p, index) => p.variants[*index].purchase_cost_yer.unwrap_or(0),
            Self::Furniture(f, _) => f.parts_cost_yer,
        }
    }
    fn adjustments(&self) -> (Vec<PriceAdjustment>, Vec<PriceAdjustment>) {
        match self {
            Self::Product(..) => (vec![], vec![]),
            Self::Furniture(f, index) => {
                let v = &f.variants[*index];
                let options = |items: &[eitmad_contracts::furniture::FurnitureOption],
                               allowed: &[uuid::Uuid]| {
                    items
                        .iter()
                        .filter(|o| !o.archived && (allowed.is_empty() || allowed.contains(&o.id)))
                        .map(|o| PriceAdjustment {
                            id: o.id,
                            price_adjustment_yer: o.price_adjustment_yer,
                        })
                        .collect()
                };
                (
                    options(&f.colors, &v.color_ids),
                    options(&f.handles, &v.handle_ids),
                )
            }
        }
    }
    fn check_options(&self, input: &PriceSelection) -> Result<(), PricingError> {
        let (colors, handles) = self.adjustments();
        adjustment(&colors, input.color_id)?;
        adjustment(&handles, input.handle_id)?;
        Ok(())
    }
}
fn source(
    tx: &PricingTransaction<'_>,
    actor: &AuthorizationContext,
    target: &PriceTarget,
) -> Result<Source, PricingError> {
    if target.scope() != &actor.scope || target.schema_version() != 1 || target.revision() == 0 {
        return Err(PricingError::Reference);
    }
    match target {
        PriceTarget::Product(r) => {
            let p = tx
                .products()
                .product(&actor.scope, r.product_id.value())?
                .ok_or(PricingError::Reference)?;
            if p.revision != r.revision
                || p.archived
                || tx
                    .products()
                    .category(&actor.scope, p.category_id.value())?
                    .is_none_or(|c| c.archived)
            {
                return Err(PricingError::Reference);
            }
            let index = p
                .variants
                .iter()
                .position(|v| v.id == r.variant_id && !v.archived)
                .ok_or(PricingError::Reference)?;
            if p.variants[index].purchase_cost_yer.is_none() {
                return Err(PricingError::Unconfirmed);
            }
            Ok(Source::Product(p, index))
        }
        PriceTarget::Furniture(r) => {
            let f = tx
                .furnitures()
                .furniture(&actor.scope, r.furniture_id.value())?
                .ok_or(PricingError::Reference)?;
            if f.revision != r.revision
                || f.state != FurnitureState::Active
                || tx
                    .furnitures()
                    .category(&actor.scope, f.category_id.value())?
                    .is_none_or(|c| c.archived)
            {
                return Err(PricingError::Reference);
            }
            let index = f
                .variants
                .iter()
                .position(|v| v.id == r.variant_id && !v.archived)
                .ok_or(PricingError::Reference)?;
            Ok(Source::Furniture(f, index))
        }
    }
}
fn review_cost(price: i64, cost: i64) -> Result<PriceReview, PricingError> {
    if price < 0 || cost < 0 {
        return Err(PricingError::Invalid);
    }
    Ok(PriceReview {
        cost_yer: cost,
        margin_yer: price.checked_sub(cost).ok_or(PricingError::Invalid)?,
        below_cost: price < cost,
    })
}
fn adjustment(options: &[PriceAdjustment], id: Option<uuid::Uuid>) -> Result<i64, PricingError> {
    id.map_or(Ok(0), |id| {
        options
            .iter()
            .find(|o| o.id == id)
            .map(|o| o.price_adjustment_yer)
            .ok_or(PricingError::Reference)
    })
}
struct ItemDetails<'a> {
    target: PriceTarget,
    name: &'a str,
    variant: &'a str,
    category: &'a str,
    cost: i64,
}
fn item_cursor(target: &PriceTarget) -> String {
    let (kind, entry, variant) = target.identity();
    format!("{kind}:{entry}:{variant}")
}
fn push_item(
    tx: &PricingTransaction<'_>,
    items: &mut Vec<PriceItem>,
    details: ItemDetails<'_>,
    access: (bool, bool),
    term: &str,
) -> Result<(), PricingError> {
    let ItemDetails {
        target,
        name,
        variant,
        category,
        cost,
    } = details;
    let (manage, costs) = access;
    if !eitmad_material::normalize_search(&format!("{name} {variant} {category}"))
        .contains(&eitmad_material::normalize_search(term))
    {
        return Ok(());
    }
    let published = tx.latest(&target)?;
    let publication_required = published.as_ref().is_none_or(|p| p.target != target);
    if publication_required && !manage {
        return Ok(());
    }
    let margin = if costs {
        published
            .as_ref()
            .map(|p| review_cost(p.selling_price_yer, cost).map(|r| r.margin_yer))
            .transpose()?
    } else {
        None
    };
    let published = published.map(|p| PriceSummary {
        currency: p.currency,
        selling_price_yer: p.selling_price_yer,
        revision: p.revision,
        confirmed_at: p.confirmed_at,
    });
    items.push(PriceItem {
        publication_required,
        target,
        name: name.into(),
        variant_name: variant.into(),
        category_name: category.into(),
        published,
        cost_yer: costs.then_some(cost),
        margin_yer: margin,
    });
    Ok(())
}
fn audit(context: &MutationContext, input: &PublishPrice) -> MutationAuditRecord {
    let (kind, id, variant) = input.target.identity();
    let mut value = MutationAuditRecord::from_authorization(
        &context.authorization,
        context.occurred_at,
        context.correlation_id,
        "eitmad.pricing.publish.v1",
        AuditTarget {
            kind: format!("{kind}-price"),
            identifiers: vec![id.to_string(), variant.to_string()],
        },
    );
    value.idempotency_key = Some(context.idempotency_key);
    value.causation_id = context.causation_id;
    value.changed_identifiers = vec!["selling-price".into()];
    value
}
#[must_use]
pub const fn error_code(error: PricingError) -> &'static str {
    match error {
        PricingError::Denied => "eitmad.error.authorization-denied.v1",
        PricingError::Invalid => "eitmad.error.pricing-invalid.v1",
        PricingError::Reference => "eitmad.error.pricing-reference-invalid.v1",
        PricingError::Conflict { .. } => "eitmad.error.pricing-revision-conflict.v1",
        PricingError::BelowCost => "eitmad.error.pricing-below-cost.v1",
        PricingError::Unconfirmed => "eitmad.error.pricing-unconfirmed.v1",
    }
}

#[cfg(test)]
mod tests;
