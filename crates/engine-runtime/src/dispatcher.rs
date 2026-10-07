//! Engine composition dispatcher for Rust-owned product verticals.

use std::sync::Arc;
#[path = "discount_approval.rs"]
mod discount_approval;

use crate::accounts::{DesktopAccountError, DesktopAccountService};
use async_trait::async_trait;
use eitmad_authorization::FURNITURE_READ_PERMISSION;
use eitmad_authorization::MATERIAL_READ_PERMISSION;
use eitmad_authorization::{
    AUTHORIZATION_MANAGE_PERMISSION, AccessAuditContext, AuthorizationError, AuthorizationService,
    CONFIG_READ_PERMISSION, MutationContext, PERMISSIONS_READ_PERMISSION, now,
};
use eitmad_authorization::{PART_READ_PERMISSION, PRODUCT_READ_PERMISSION};
use eitmad_configuration::{ConfigurationError, ConfigurationService};
use eitmad_contracts::{
    accounts::CreateDesktopAccount,
    commands::{Command, CommandResult, CreateCustomer, UpdateCustomer},
    errors::{ContractError, ErrorCode, ErrorDetail, MessageId, RetryDisposition},
    events::{Event, Subscription},
    material::{SaveMaterial, SaveMaterialCategory, SaveMaterialUnit},
    queries::{Query, QueryResult},
};
use eitmad_customer::{CUSTOMER_READ_PERMISSION, CustomerError, CustomerService};
use eitmad_furniture::{FurnitureError, FurnitureService};
use eitmad_material::{MaterialError, MaterialService};
use eitmad_observability_audit::AuditOutcome;
use eitmad_part::{PartError, PartService};
use eitmad_product::{ProductError, ProductService};
use eitmad_storage::AuthorityStore;
use eitmad_storage::MAX_PUBLICATION_RECOVERY_PAGE;

use crate::local_ipc::{
    CommandDispatcher, DispatchContext, EventBroker, QueryDispatcher, SubscriptionContext,
};

#[derive(Clone)]
pub struct ProductDispatcher {
    store: AuthorityStore,
    authorization: AuthorizationService,
    configuration: ConfigurationService,
    customers: CustomerService,
    drafts: eitmad_pricing::QuotationDraftService,
    approval_server: Option<Arc<dyn eitmad_pricing::DiscountApprovalServer>>,
    approval_watch: Arc<std::sync::Mutex<Option<discount_approval::ApprovalWatch>>>,
    materials: MaterialService,
    parts: PartService,
    images: eitmad_catalog_image::CatalogImageService,
    image_workers: Arc<tokio::sync::Semaphore>,
    products: ProductService,
    pricing: eitmad_pricing::PricingService,
    catalog_replication: Option<Arc<dyn eitmad_pricing::CatalogReplication>>,
    furnitures: FurnitureService,
    accounts: DesktopAccountService,
    events: Arc<dyn ProductEventPublisher>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicationRecoveryError;

pub const MAX_STARTUP_PUBLICATION_RECOVERY: usize = 1_024;

impl std::fmt::Display for PublicationRecoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("durable event publication recovery failed")
    }
}

impl std::error::Error for PublicationRecoveryError {}

trait ProductEventPublisher: Send + Sync {
    fn publish(&self, scope: eitmad_contracts::identity::ScopeRef, event: Event) -> Result<(), ()>;

    fn policy_changed(&self, scope: eitmad_contracts::identity::ScopeRef);
}

impl ProductEventPublisher for EventBroker {
    fn publish(&self, scope: eitmad_contracts::identity::ScopeRef, event: Event) -> Result<(), ()> {
        self.publish(scope, event).map(|_| ()).map_err(|_| ())
    }

    fn policy_changed(&self, scope: eitmad_contracts::identity::ScopeRef) {
        self.policy_changed(scope);
    }
}

impl ProductDispatcher {
    #[must_use]
    pub fn new(store: AuthorityStore, events: EventBroker) -> Self {
        Self::with_event_publisher(store, Arc::new(events))
    }

    /// Attaches the committed-event publisher used by mutation dispatch.
    fn with_event_publisher(store: AuthorityStore, events: Arc<dyn ProductEventPublisher>) -> Self {
        let authorization = AuthorizationService::new(store.clone());
        let configuration = ConfigurationService::new(store.clone(), authorization.clone());
        let customers = CustomerService::new(store.clone(), authorization.clone());
        let drafts =
            eitmad_pricing::QuotationDraftService::new(store.clone(), authorization.clone());
        let materials = MaterialService::new(store.clone(), authorization.clone());
        let furnitures = FurnitureService::new(store.clone(), authorization.clone());
        let products = ProductService::new(store.clone(), authorization.clone());
        let pricing = eitmad_pricing::PricingService::new(store.clone(), authorization.clone());
        let parts = PartService::new(store.clone(), authorization.clone());
        let accounts = DesktopAccountService::new(store.clone(), authorization.clone());
        let images =
            eitmad_catalog_image::CatalogImageService::new(store.clone(), authorization.clone());
        Self {
            store,
            authorization,
            configuration,
            customers,
            drafts,
            approval_server: None,
            approval_watch: discount_approval::empty_watch(),
            materials,
            parts,
            images,
            image_workers: Arc::new(tokio::sync::Semaphore::new(2)),
            products,
            pricing,
            catalog_replication: None,
            furnitures,
            accounts,
            events,
        }
    }

    #[must_use]
    pub const fn authorization(&self) -> &AuthorizationService {
        &self.authorization
    }
    /// Attaches the Rust-owned catalog worker used by price queries and background retries.
    #[must_use]
    pub fn with_catalog_replication(
        mut self,
        replication: Arc<dyn eitmad_pricing::CatalogReplication>,
    ) -> Self {
        self.catalog_replication = Some(replication);
        self
    }
    /// Retries durable Manager work under current local and remote authority.
    /// # Errors
    /// Retries every actor and drains committed notifications before returning the first failure.
    pub fn retry_catalog_replication(&self) -> Result<usize, eitmad_pricing::PricingError> {
        let Some(replication) = &self.catalog_replication else {
            return Ok(0);
        };
        let mut count = 0;
        let mut failure = None;
        for actor in self.store.pending_catalog_actors()? {
            match replication.synchronize(
                &actor,
                eitmad_contracts::transport::UnixMillis(now().0.saturating_add(10_000)),
            ) {
                Ok(transferred) => count += transferred,
                Err(
                    eitmad_pricing::PricingError::Denied | eitmad_pricing::PricingError::Reference,
                ) => (),
                Err(error) => {
                    failure.get_or_insert(error);
                }
            }
        }
        if self.drain_pending_publications().is_err() {
            failure.get_or_insert(eitmad_pricing::PricingError::Unconfirmed);
        }
        failure.map_or(Ok(count), Err)
    }

    #[must_use]
    pub fn with_price_confirmation(
        mut self,
        confirmation: Arc<dyn eitmad_pricing::PriceConfirmation>,
    ) -> Self {
        self.pricing = self.pricing.with_confirmation(confirmation);
        self
    }

    #[must_use]
    /// Configures one authenticated transfer path shared by image queries and the upload worker.
    pub fn with_catalog_image_transfer(
        mut self,
        transfer: Arc<dyn eitmad_catalog_image::CatalogImageTransfer>,
    ) -> Self {
        self.images = self.images.with_transfer(transfer);
        self
    }

    /// Runs one bounded upload batch on the engine background worker.
    /// # Errors
    /// Retains pending work on failure.
    pub fn retry_catalog_images(&self) -> Result<usize, eitmad_catalog_image::ImageError> {
        self.images.retry_uploads()
    }

    /// Authorizes public reads, attempts refresh within the deadline, and rechecks access before returning confirmed cache data.
    async fn sales_catalog_query(
        &self,
        context: &DispatchContext,
        query: Query,
    ) -> Result<QueryResult, ContractError> {
        let pricing = self.pricing.clone();
        let actor = context.authorization.clone();
        let replication = self.catalog_replication.clone();
        let deadline = eitmad_contracts::transport::UnixMillis(
            context
                .deadline
                .0
                .saturating_sub(1_000)
                .min(now().0.saturating_add(10_000)),
        );
        tokio::task::spawn_blocking(move || {
            // Validate and authorize before network work. Recheck after replication.
            match &query {
                Query::SalesCatalog(q) => {
                    pricing.sales_catalog(&actor, q)?;
                }
                Query::SalesCatalogItem(q) => {
                    if q.target.scope() != &actor.scope {
                        return Err(eitmad_pricing::PricingError::Denied);
                    }
                }
                Query::SalesConfiguration(q) => {
                    if q.selection.target.scope() != &actor.scope {
                        return Err(eitmad_pricing::PricingError::Denied);
                    }
                }
                _ => unreachable!(),
            }
            let refresh = !matches!(&query, Query::SalesCatalog(q) if q.after.is_some());
            let available = refresh
                && replication
                    .as_ref()
                    .is_some_and(|r| r.synchronize(&actor, deadline).is_ok());
            match query {
                Query::SalesCatalog(q) => {
                    let mut p = pricing.sales_catalog(&actor, &q)?;
                    p.server_available = available;
                    Ok(QueryResult::SalesCatalog(p))
                }
                Query::SalesCatalogItem(q) => {
                    let mut p = pricing.sales_catalog_item(&actor, &q)?;
                    p.server_available = available;
                    Ok(QueryResult::SalesCatalogItem(p))
                }
                Query::SalesConfiguration(q) => {
                    let mut p = pricing.sales_configuration(&actor, &q)?;
                    p.server_available = available;
                    Ok(QueryResult::SalesConfiguration(Box::new(p)))
                }
                _ => unreachable!(),
            }
        })
        .await
        .map_err(|_| pricing_error(eitmad_pricing::PricingError::Unconfirmed, context))?
        .map_err(|e| pricing_error(e, context))
    }

    /// Routes public catalog queries to their bounded refresh path and retains existing authorized pricing routes.
    async fn pricing_query(
        &self,
        context: &DispatchContext,
        query: Query,
    ) -> Result<QueryResult, ContractError> {
        match query {
            Query::QuotationEvaluation(input) => {
                let pricing = self.pricing.clone();
                let actor = context.authorization.clone();
                let replication = self.catalog_replication.clone();
                let deadline = eitmad_contracts::transport::UnixMillis(
                    context
                        .deadline
                        .0
                        .saturating_sub(1000)
                        .min(now().0.saturating_add(10_000)),
                );
                tokio::task::spawn_blocking(move || {
                    let catalog_actor = pricing.authorize_quotation(&actor)?;
                    let available = replication
                        .as_ref()
                        .is_some_and(|r| r.synchronize(&catalog_actor, deadline).is_ok());
                    let mut result = pricing.evaluate_quotation(&actor, &input)?;
                    result.server_available = available;
                    Ok::<_, eitmad_pricing::PricingError>(result)
                })
                .await
                .map_err(|_| pricing_error(eitmad_pricing::PricingError::Unconfirmed, context))?
                .map(QueryResult::QuotationEvaluation)
                .map_err(|e| pricing_error(e, context))
            }
            query @ (Query::SalesCatalog(_)
            | Query::SalesCatalogItem(_)
            | Query::SalesConfiguration(_)) => self.sales_catalog_query(context, query).await,
            Query::Prices(query) => {
                let pricing = self.pricing.clone();
                let actor = context.authorization.clone();
                let deadline = context.deadline;
                let correlation_id = context.correlation_id;
                let replication = self.catalog_replication.clone();
                tokio::task::spawn_blocking(move || {
                    let mut server_available = false;
                    if query.after.is_none() {
                        if let Some(replication) = &replication {
                            // A remote failure must not prevent an authorized cache read.
                            server_available = replication
                                .synchronize(
                                    &actor,
                                    eitmad_contracts::transport::UnixMillis(
                                        deadline
                                            .0
                                            .saturating_sub(1_000)
                                            .min(now().0.saturating_add(10_000)),
                                    ),
                                )
                                .is_ok();
                        }
                    }
                    if query.after.is_none() && replication.is_none() {
                        let context = MutationContext {
                            authorization: actor.clone(),
                            correlation_id,
                            causation_id: None,
                            idempotency_key: eitmad_contracts::transport::IdempotencyKey::new(
                                uuid::Uuid::new_v4(),
                            ),
                            occurred_at: now(),
                        };
                        // Bound remote refresh and reserve time for local confirmed-cache reads.
                        let refresh_deadline = eitmad_contracts::transport::UnixMillis(
                            deadline
                                .0
                                .saturating_sub(1_000)
                                .min(now().0.saturating_add(2_000)),
                        );
                        server_available = pricing.refresh(&context, refresh_deadline).is_ok();
                    }
                    let mut page = pricing.list(&actor, &query)?;
                    page.server_available = server_available;
                    Ok(page)
                })
                .await
                .map_err(|_| pricing_error(eitmad_pricing::PricingError::Unconfirmed, context))?
                .map(QueryResult::Prices)
                .map_err(|e| pricing_error(e, context))
            }
            Query::PriceReview(query) => self
                .pricing
                .review(&context.authorization, &query)
                .map(QueryResult::PriceReview)
                .map_err(|e| pricing_error(e, context)),
            Query::SellingPrice(query) => self
                .pricing
                .selection(&context.authorization, &query)
                .map(QueryResult::SellingPrice)
                .map_err(|e| pricing_error(e, context)),
            Query::DiscountTotal(query) => self
                .pricing
                .calculate_discount(&context.authorization, &query)
                .map(QueryResult::DiscountTotal)
                .map_err(|e| pricing_error(e, context)),
            _ => unreachable!("only pricing queries are routed here"),
        }
    }
    async fn publish_price(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: eitmad_contracts::pricing::PublishPrice,
    ) -> Result<CommandResult, ContractError> {
        let pricing = self.pricing.clone();
        let actor = mutation.clone();
        let deadline = context.deadline;
        let result =
            tokio::task::spawn_blocking(move || pricing.publish(&actor, &command, deadline))
                .await
                .map_err(|_| pricing_error(eitmad_pricing::PricingError::Unconfirmed, context))?
                .map_err(|e| pricing_error(e, context))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| pricing_error(eitmad_pricing::PricingError::Unconfirmed, context))?;
        Ok(CommandResult::PricePublished(result))
    }
    fn mutation_context(context: &DispatchContext) -> Result<MutationContext, Box<ContractError>> {
        let idempotency_key = context.idempotency_key.ok_or_else(|| {
            Box::new(error(
                "eitmad.error.contract-invalid.v1",
                "eitmad.message.contract-invalid.v1",
                context,
                RetryDisposition::Never,
                None,
            ))
        })?;
        Ok(MutationContext {
            authorization: context.authorization.clone(),
            correlation_id: context.correlation_id,
            causation_id: context.causation_id,
            idempotency_key,
            occurred_at: now(),
        })
    }

    fn dispatch_account_change(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: Command,
    ) -> Result<CommandResult, Box<ContractError>> {
        let result = match command {
            Command::UpdateDesktopAccount(input) => self
                .accounts
                .update(mutation, &input)
                .map(CommandResult::DesktopAccountUpdated),
            Command::DeactivateDesktopAccount(input) => self
                .accounts
                .deactivate(mutation, &input)
                .map(CommandResult::DesktopAccountDeactivated),
            _ => return Err(Box::new(unsupported(context))),
        }
        .map_err(|e| Box::new(desktop_account_error(e, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| {
                Box::new(desktop_account_error(
                    DesktopAccountError::Unavailable,
                    context,
                ))
            })?;
        Ok(result)
    }
    fn dispatch_draft_query(
        &self,
        context: &DispatchContext,
        query: Query,
    ) -> Result<QueryResult, Box<ContractError>> {
        match query {
            Query::QuotationDraft(query) => self
                .drafts
                .get(&context.authorization, &query)
                .map(|draft| QueryResult::QuotationDraft(Box::new(draft)))
                .map_err(|e| Box::new(draft_error(e, context))),
            Query::QuotationDrafts(query) => self
                .drafts
                .list(&context.authorization, &query)
                .map(QueryResult::QuotationDrafts)
                .map_err(|e| Box::new(draft_error(e, context))),
            _ => Err(Box::new(unsupported(context))),
        }
    }

    fn dispatch_draft_command(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: Command,
    ) -> Result<CommandResult, Box<ContractError>> {
        let result = match command {
            Command::CreateQuotationDraft(input) => self
                .drafts
                .create(mutation, &input)
                .map(|d| CommandResult::QuotationDraftCreated(Box::new(d))),
            Command::UpdateQuotationDraft(input) => self
                .drafts
                .update(mutation, &input)
                .map(|d| CommandResult::QuotationDraftUpdated(Box::new(d))),
            _ => return Err(Box::new(unsupported(context))),
        }
        .map_err(|e| Box::new(draft_error(e, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| {
                Box::new(draft_error(
                    eitmad_pricing::QuotationDraftError::Unavailable,
                    context,
                ))
            })?;
        Ok(result)
    }
    fn create_customer(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: &CreateCustomer,
    ) -> Result<CommandResult, Box<ContractError>> {
        let result = self
            .customers
            .create(mutation, command)
            .map_err(|error| Box::new(customer_error(error, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| Box::new(customer_error(CustomerError::Unavailable, context)))?;
        Ok(CommandResult::CustomerCreated(result))
    }

    fn update_customer(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: &UpdateCustomer,
    ) -> Result<CommandResult, Box<ContractError>> {
        let result = self
            .customers
            .update(mutation, command)
            .map_err(|error| Box::new(customer_error(error, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| Box::new(customer_error(CustomerError::Unavailable, context)))?;
        Ok(CommandResult::CustomerUpdated(result))
    }

    fn save_material_category(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: &SaveMaterialCategory,
    ) -> Result<CommandResult, Box<ContractError>> {
        let saved = self
            .materials
            .save_category(mutation, command)
            .map_err(|error| Box::new(material_error(error, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| Box::new(material_error(MaterialError::Unavailable, context)))?;
        Ok(CommandResult::MaterialCategorySaved(saved))
    }

    fn save_material_unit(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: &SaveMaterialUnit,
    ) -> Result<CommandResult, Box<ContractError>> {
        let saved = self
            .materials
            .save_unit(mutation, command)
            .map_err(|error| Box::new(material_error(error, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| Box::new(material_error(MaterialError::Unavailable, context)))?;
        Ok(CommandResult::MaterialUnitSaved(saved))
    }

    fn save_material(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: &SaveMaterial,
    ) -> Result<CommandResult, Box<ContractError>> {
        let saved = self
            .materials
            .save_material(mutation, command)
            .map_err(|error| Box::new(material_error(error, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| Box::new(material_error(MaterialError::Unavailable, context)))?;
        Ok(CommandResult::MaterialSaved(saved))
    }

    /// Negotiates protocol 1.11, delegates authorization and persistence to Parts, and publishes the committed outbox.
    fn dispatch_part_command(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: Command,
    ) -> Result<CommandResult, Box<ContractError>> {
        let result = match command {
            Command::SavePart(command) => self
                .parts
                .save(mutation, &command)
                .map(CommandResult::PartSaved),
            Command::SavePartCategory(command) => self
                .parts
                .save_category(mutation, &command)
                .map(CommandResult::PartCategorySaved),
            _ => unreachable!("only part commands are routed here"),
        }
        .map_err(|error| Box::new(part_error(error, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| Box::new(part_error(PartError::Unavailable, context)))?;
        Ok(result)
    }

    /// Negotiates protocol 1.11 and delegates scoped part reads and cost review to the Rust authority.
    fn dispatch_part_query(
        &self,
        context: &DispatchContext,
        query: Query,
    ) -> Result<QueryResult, Box<ContractError>> {
        match query {
            Query::Parts(query) => self
                .parts
                .list(&context.authorization, &query)
                .map(QueryResult::Parts),
            Query::PartCategories(query) => self
                .parts
                .categories(&context.authorization, &query)
                .map(QueryResult::PartCategories),
            Query::PartCost(query) => self
                .parts
                .cost(&context.authorization, &query)
                .map(QueryResult::PartCost),
            Query::PartComposition(query) => self
                .parts
                .composition(&context.authorization, &query)
                .map(QueryResult::PartComposition),
            _ => unreachable!("only part queries are routed here"),
        }
        .map_err(|error| Box::new(part_error(error, context)))
    }

    /// Negotiates protocol 1.12, delegates authorization and persistence to Products, and publishes the committed outbox.
    fn dispatch_product_command(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: Command,
    ) -> Result<CommandResult, Box<ContractError>> {
        let result = match command {
            Command::SaveProduct(command) => self
                .products
                .save(mutation, &command)
                .map(CommandResult::ProductSaved),
            Command::SaveProductCategory(command) => self
                .products
                .save_category(mutation, &command)
                .map(CommandResult::ProductCategorySaved),
            _ => unreachable!("only product commands are routed here"),
        }
        .map_err(|error| Box::new(product_error(error, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| Box::new(product_error(ProductError::Unavailable, context)))?;
        Ok(result)
    }

    /// Negotiates protocol 1.12 and delegates scoped product reads and cost review to the Rust authority.
    fn dispatch_product_query(
        &self,
        context: &DispatchContext,
        query: Query,
    ) -> Result<QueryResult, Box<ContractError>> {
        match query {
            Query::Products(query) => self
                .products
                .list(&context.authorization, &query)
                .map(QueryResult::Products),
            Query::ProductCategories(query) => self
                .products
                .categories(&context.authorization, &query)
                .map(QueryResult::ProductCategories),
            Query::ProductRevision(query) => self
                .products
                .revision(&context.authorization, &query)
                .map(QueryResult::ProductRevision),
            _ => unreachable!("only product queries are routed here"),
        }
        .map_err(|error| Box::new(product_error(error, context)))
    }

    /// Records scoped query outcomes without retaining the query payload.
    fn audit_query_result(
        &self,
        context: &DispatchContext,
        operation: &str,
        result: &Result<QueryResult, ContractError>,
    ) -> Result<(), Box<ContractError>> {
        let (outcome, error_code) = match result {
            Ok(_) => (AuditOutcome::Succeeded, None),
            Err(error) if error.code.as_str() == "eitmad.error.authorization-denied.v1" => {
                (AuditOutcome::Denied, Some(error.code.as_str()))
            }
            Err(error) => (AuditOutcome::Failed, Some(error.code.as_str())),
        };
        self.authorization
            .audit_access_result(
                &AccessAuditContext {
                    authorization: context.authorization.clone(),
                    correlation_id: context.correlation_id,
                    causation_id: context.causation_id,
                    occurred_at: now(),
                },
                operation,
                "query-scope",
                outcome,
                error_code,
                Vec::new(),
            )
            .map_err(|error| Box::new(authorization_error(error, context)))?;
        Ok(())
    }

    /// Negotiates protocol 1.13, delegates authorization and persistence to Furnitures, and publishes the committed outbox.
    fn dispatch_furniture_command(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: Command,
    ) -> Result<CommandResult, Box<ContractError>> {
        let result = match command {
            Command::SaveFurniture(command) => self
                .furnitures
                .save(mutation, &command)
                .map(CommandResult::FurnitureSaved),
            Command::SaveFurnitureCategory(command) => self
                .furnitures
                .save_category(mutation, &command)
                .map(CommandResult::FurnitureCategorySaved),
            _ => unreachable!("only furniture commands are routed here"),
        }
        .map_err(|error| Box::new(furniture_error(error, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| Box::new(furniture_error(FurnitureError::Unavailable, context)))?;
        Ok(result)
    }

    /// Negotiates protocol 1.13 and delegates scoped furniture reads and cost review to the Rust authority.
    fn dispatch_furniture_query(
        &self,
        context: &DispatchContext,
        query: Query,
    ) -> Result<QueryResult, Box<ContractError>> {
        match query {
            Query::Furnitures(query) => self
                .furnitures
                .list(&context.authorization, &query)
                .map(QueryResult::Furnitures),
            Query::FurnitureCategories(query) => self
                .furnitures
                .categories(&context.authorization, &query)
                .map(QueryResult::FurnitureCategories),
            Query::FurnitureRevision(query) => self
                .furnitures
                .revision(&context.authorization, &query)
                .map(QueryResult::FurnitureRevision),
            Query::FurnitureReview(query) => self
                .furnitures
                .review(&context.authorization, &query)
                .map(QueryResult::FurnitureReview),
            Query::FurnitureSelection(query) => self
                .furnitures
                .selection(&context.authorization, &query)
                .map(QueryResult::FurnitureSelection),
            _ => unreachable!("only furniture queries are routed here"),
        }
        .map_err(|error| Box::new(furniture_error(error, context)))
    }

    /// Checks protocol support, creates the audited account, and publishes its committed event.
    fn create_desktop_account(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: &CreateDesktopAccount,
    ) -> Result<CommandResult, Box<ContractError>> {
        let account = self
            .accounts
            .create(mutation, command)
            .map_err(|error| Box::new(desktop_account_error(error, context)))?;
        self.publish_pending(context, mutation.idempotency_key)
            .map_err(|()| {
                Box::new(desktop_account_error(
                    DesktopAccountError::Unavailable,
                    context,
                ))
            })?;
        Ok(CommandResult::DesktopAccountCreated(account))
    }

    fn publish_pending(
        &self,
        context: &DispatchContext,
        idempotency_key: eitmad_contracts::transport::IdempotencyKey,
    ) -> Result<(), ()> {
        let Some(publication) = self
            .store
            .pending_publication(&context.authorization.scope, idempotency_key)
            .map_err(|_| ())?
        else {
            return Ok(());
        };
        self.events
            .publish(publication.scope.clone(), publication.event)?;
        if publication.policy_changed {
            self.events.policy_changed(publication.scope.clone());
        }
        self.store
            .complete_publication(&publication.scope, idempotency_key)
            .map_err(|_| ())?;
        Ok(())
    }

    /// Publishes and completes every event left durable by a committed mutation.
    ///
    /// The engine calls this before accepting IPC traffic so a crash between
    /// commit and publication cannot strand configuration or policy state.
    ///
    /// # Errors
    ///
    /// Returns an error while preserving the current and later outbox rows for retry.
    pub fn drain_pending_publications(&self) -> Result<(), PublicationRecoveryError> {
        let mut recovered = 0_usize;
        loop {
            let publications = self
                .store
                .pending_publications(MAX_PUBLICATION_RECOVERY_PAGE)
                .map_err(|_| PublicationRecoveryError)?;
            if publications.is_empty() {
                return Ok(());
            }
            if recovered + publications.len() > MAX_STARTUP_PUBLICATION_RECOVERY {
                return Err(PublicationRecoveryError);
            }
            for publication in &publications {
                self.events
                    .publish(publication.scope.clone(), publication.event.clone())
                    .map_err(|()| PublicationRecoveryError)?;
                if publication.policy_changed {
                    self.events.policy_changed(publication.scope.clone());
                }
            }
            recovered += publications.len();
            self.store
                .complete_publications(&publications)
                .map_err(|_| PublicationRecoveryError)?;
        }
    }
}

#[async_trait]
impl CommandDispatcher for ProductDispatcher {
    /// Checks protocol support and routes typed commands through their Rust authority.
    async fn dispatch_command(
        &self,
        context: DispatchContext,
        command: Command,
    ) -> Result<CommandResult, ContractError> {
        let mutation = Self::mutation_context(&context).map_err(|error| *error)?;
        match command {
            Command::ImportCatalogImage(input) => {
                let permit = Arc::clone(&self.image_workers)
                    .try_acquire_owned()
                    .map_err(|_| {
                        image_error(eitmad_catalog_image::ImageError::Unavailable, &context)
                    })?;
                let images = self.images.clone();
                let mutation = mutation.clone();
                let deadline = context.deadline;
                tokio::task::spawn_blocking(move || {
                    // The permit survives IPC timeout until the actual worker exits.
                    let _permit = permit;
                    images.import(&mutation, &input, deadline)
                })
                .await
                .map_err(|_| image_error(eitmad_catalog_image::ImageError::Unavailable, &context))?
                .map(CommandResult::CatalogImageImported)
                .map_err(|e| image_error(e, &context))
            }
            Command::UpdateConfiguration(command) => {
                let outcome = self
                    .configuration
                    .update(&mutation, &command)
                    .map_err(|error| configuration_error(error, &context))?;
                self.publish_pending(&context, mutation.idempotency_key)
                    .map_err(|()| configuration_error(ConfigurationError::Unavailable, &context))?;
                Ok(CommandResult::ConfigurationUpdated(outcome.snapshot))
            }
            Command::GrantScopeRelationship(command) => {
                let result = self
                    .authorization
                    .grant_relationship(&mutation, &command)
                    .map_err(|error| authorization_error(error, &context))?;
                self.publish_pending(&context, mutation.idempotency_key)
                    .map_err(|()| authorization_error(AuthorizationError::Unavailable, &context))?;
                Ok(CommandResult::RelationshipGranted(result))
            }
            Command::RevokeScopeRelationship(command) => {
                let result = self
                    .authorization
                    .revoke_relationship(&mutation, &command)
                    .map_err(|error| authorization_error(error, &context))?;
                self.publish_pending(&context, mutation.idempotency_key)
                    .map_err(|()| authorization_error(AuthorizationError::Unavailable, &context))?;
                Ok(CommandResult::RelationshipRevoked(result))
            }
            command
            @ (Command::RequestDiscountApproval(_) | Command::DecideDiscountApproval(_)) => {
                self.approval_command(&context, &mutation, command).await
            }
            command @ (Command::CreateQuotationDraft(_) | Command::UpdateQuotationDraft(_)) => {
                self.save_approval_draft(&context, &mutation, command).await
            }
            Command::CreateCustomer(command) => self
                .create_customer(&context, &mutation, &command)
                .map_err(|error| *error),
            Command::PublishPrice(command) => {
                self.publish_price(&context, &mutation, command).await
            }
            Command::UpdateCustomer(command) => self
                .update_customer(&context, &mutation, &command)
                .map_err(|error| *error),
            Command::SaveMaterialCategory(command) => self
                .save_material_category(&context, &mutation, &command)
                .map_err(|error| *error),
            Command::SaveMaterialUnit(command) => self
                .save_material_unit(&context, &mutation, &command)
                .map_err(|error| *error),
            command @ (Command::SaveFurniture(_) | Command::SaveFurnitureCategory(_)) => self
                .dispatch_furniture_command(&context, &mutation, command)
                .map_err(|e| *e),
            command @ (Command::SaveProduct(_) | Command::SaveProductCategory(_)) => self
                .dispatch_product_command(&context, &mutation, command)
                .map_err(|error| *error),
            command @ (Command::SavePart(_) | Command::SavePartCategory(_)) => self
                .dispatch_part_command(&context, &mutation, command)
                .map_err(|error| *error),
            Command::SaveMaterial(command) => self
                .save_material(&context, &mutation, &command)
                .map_err(|error| *error),
            Command::CreateDesktopAccount(command) => self
                .create_desktop_account(&context, &mutation, &command)
                .map_err(|error| *error),
            command @ (Command::UpdateDesktopAccount(_) | Command::DeactivateDesktopAccount(_)) => {
                self.dispatch_account_change(&context, &mutation, command)
                    .map_err(|e| *e)
            }
        }
    }
}

#[async_trait]
impl QueryDispatcher for ProductDispatcher {
    /// Checks protocol support and routes typed queries through their Rust authority.
    async fn dispatch_query(
        &self,
        context: DispatchContext,
        query: Query,
    ) -> Result<QueryResult, ContractError> {
        let operation = query.kind();
        let result = match query {
            Query::CatalogImage(input) => {
                let permit = Arc::clone(&self.image_workers)
                    .try_acquire_owned()
                    .map_err(|_| {
                        image_error(eitmad_catalog_image::ImageError::Unavailable, &context)
                    })?;
                let images = self.images.clone();
                let actor = context.authorization.clone();
                let deadline = context.deadline;
                tokio::task::spawn_blocking(move || {
                    let _permit = permit;
                    images.get(&actor, &input, deadline)
                })
                .await
                .map_err(|_| image_error(eitmad_catalog_image::ImageError::Unavailable, &context))?
                .map(QueryResult::CatalogImage)
                .map_err(|e| image_error(e, &context))
            }
            Query::Configuration(_) => self
                .configuration
                .snapshot(&context.authorization)
                .map(QueryResult::Configuration)
                .map_err(|error| configuration_error(error, &context)),
            Query::EffectivePermissions(_) => self
                .authorization
                .effective_permissions(&context.authorization)
                .map(QueryResult::EffectivePermissions)
                .map_err(|error| authorization_error(error, &context)),
            Query::ScopeRelationships(query) => self
                .authorization
                .list_relationships(&context.authorization, &query)
                .map(QueryResult::ScopeRelationships)
                .map_err(|error| authorization_error(error, &context)),
            Query::DiscountApprovals(query) => self.approval_list(&context, query).await,
            query @ (Query::QuotationDraft(_) | Query::QuotationDrafts(_)) => {
                self.dispatch_draft_query(&context, query).map_err(|e| *e)
            }
            Query::Customer(query) => self
                .customers
                .get(&context.authorization, &query)
                .map(QueryResult::Customer)
                .map_err(|error| customer_error(error, &context)),
            query @ (Query::QuotationEvaluation(_)
            | Query::Prices(_)
            | Query::SalesCatalog(_)
            | Query::SalesCatalogItem(_)
            | Query::SalesConfiguration(_)
            | Query::PriceReview(_)
            | Query::SellingPrice(_)
            | Query::DiscountTotal(_)) => self.pricing_query(&context, query).await,
            Query::Customers(query) => self
                .customers
                .search(&context.authorization, &query)
                .map(QueryResult::Customers)
                .map_err(|error| customer_error(error, &context)),
            query @ (Query::Furnitures(_)
            | Query::FurnitureCategories(_)
            | Query::FurnitureRevision(_)
            | Query::FurnitureReview(_)
            | Query::FurnitureSelection(_)) => self
                .dispatch_furniture_query(&context, query)
                .map_err(|e| *e),
            query @ (Query::Products(_)
            | Query::ProductCategories(_)
            | Query::ProductRevision(_)) => self
                .dispatch_product_query(&context, query)
                .map_err(|error| *error),
            query @ (Query::Parts(_)
            | Query::PartCategories(_)
            | Query::PartCost(_)
            | Query::PartComposition(_)) => self
                .dispatch_part_query(&context, query)
                .map_err(|error| *error),
            Query::Materials(query) => self
                .materials
                .list(&context.authorization, &query)
                .map(QueryResult::Materials)
                .map_err(|error| material_error(error, &context)),
            Query::MaterialReferences(_) => self
                .materials
                .references(&context.authorization)
                .map(QueryResult::MaterialReferences)
                .map_err(|error| material_error(error, &context)),
            Query::DesktopAccounts(query) => self
                .accounts
                .list(&context.authorization, &query)
                .map(QueryResult::DesktopAccounts)
                .map_err(|error| desktop_account_error(error, &context)),
        };
        self.audit_query_result(&context, operation, &result)
            .map_err(|e| *e)?;
        result
    }

    /// Checks the requested stream permission in the authenticated scope before subscription.
    async fn authorize_subscription(
        &self,
        context: SubscriptionContext,
        subscription: &Subscription,
    ) -> Result<(), ContractError> {
        let permission = match subscription {
            Subscription::Configuration(_) => CONFIG_READ_PERMISSION,
            Subscription::Permissions(_) => PERMISSIONS_READ_PERMISSION,
            Subscription::Customers(_) => CUSTOMER_READ_PERMISSION,
            Subscription::DiscountApprovals(_) => eitmad_authorization::DISCOUNT_READ_PERMISSION,
            Subscription::QuotationDrafts(_) => {
                eitmad_authorization::QUOTATION_DRAFT_READ_PERMISSION
            }
            Subscription::Furnitures(_) => FURNITURE_READ_PERMISSION,
            Subscription::Products(_) => PRODUCT_READ_PERMISSION,
            Subscription::Prices(_) => eitmad_authorization::CATALOG_READ_PERMISSION,
            Subscription::Parts(_) => PART_READ_PERMISSION,
            Subscription::Materials(_) => MATERIAL_READ_PERMISSION,
            Subscription::AuthorizationPolicy(_) => AUTHORIZATION_MANAGE_PERMISSION,
        };
        self.authorization
            .authorize(&context.authorization, permission)
            .map_err(|error| authorization_contract_error(error, context.correlation_id, None))?;
        if matches!(subscription, Subscription::DiscountApprovals(_)) {
            self.start_approval_watch(&context.authorization)
                .map_err(|e| {
                    contract_error(
                        eitmad_pricing::approval_error_code(e),
                        "eitmad.message.quotation-approval-unavailable.v1",
                        context.correlation_id,
                        RetryDisposition::SafeAfterDelay(1000),
                        None,
                    )
                })?;
        }
        Ok(())
    }
}

fn pricing_error(value: eitmad_pricing::PricingError, context: &DispatchContext) -> ContractError {
    let code = eitmad_pricing::error_code(value);
    let message = code.replace(".error.", ".message.");
    let detail = if let eitmad_pricing::PricingError::Conflict { expected, actual } = value {
        Some(ErrorDetail::RevisionConflict {
            expected: expected.unwrap_or(0),
            actual: actual.unwrap_or(0),
        })
    } else {
        None
    };
    contract_error(
        code,
        &message,
        context.correlation_id,
        if value == eitmad_pricing::PricingError::Unconfirmed {
            RetryDisposition::SafeAfterDelay(1000)
        } else {
            RetryDisposition::Never
        },
        detail,
    )
}

fn draft_error(
    value: eitmad_pricing::QuotationDraftError,
    context: &DispatchContext,
) -> ContractError {
    use eitmad_pricing::QuotationDraftError as E;
    let (code, message, retry, detail) = match value {
        E::Denied => (
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
            RetryDisposition::Never,
            None,
        ),
        E::NotFound => (
            "eitmad.error.quotation-draft-not-found.v1",
            "eitmad.message.quotation-draft-not-found.v1",
            RetryDisposition::Never,
            None,
        ),
        E::Conflict { expected, actual } => (
            "eitmad.error.quotation-draft-conflict.v1",
            "eitmad.message.quotation-draft-conflict.v1",
            RetryDisposition::Never,
            Some(ErrorDetail::RevisionConflict {
                expected: expected.unwrap_or(0),
                actual: actual.unwrap_or(0),
            }),
        ),
        E::Invalid => (
            "eitmad.error.quotation-draft-invalid.v1",
            "eitmad.message.quotation-draft-invalid.v1",
            RetryDisposition::Never,
            None,
        ),
        E::Validation(errors) => (
            "eitmad.error.quotation-draft-invalid.v1",
            "eitmad.message.quotation-draft-invalid.v1",
            RetryDisposition::Never,
            Some(ErrorDetail::QuotationDraftValidation { errors }),
        ),
        E::UnresolvedConflict => (
            "eitmad.error.quotation-draft-conflict.v1",
            "eitmad.message.quotation-draft-conflict.v1",
            RetryDisposition::Never,
            None,
        ),
        E::IdempotencyMismatch => return unsupported(context),
        E::Unavailable => (
            "eitmad.error.quotation-draft-unavailable.v1",
            "eitmad.message.quotation-draft-unavailable.v1",
            RetryDisposition::SafeAfterDelay(1000),
            None,
        ),
    };
    contract_error(code, message, context.correlation_id, retry, detail)
}

fn customer_error(error_value: CustomerError, context: &DispatchContext) -> ContractError {
    match error_value {
        CustomerError::Denied => contract_error(
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
            context.correlation_id,
            RetryDisposition::Never,
            None,
        ),
        CustomerError::NotFound => contract_error(
            "eitmad.error.customer-not-found.v1",
            "eitmad.message.customer-not-found.v1",
            context.correlation_id,
            RetryDisposition::Never,
            None,
        ),
        CustomerError::RevisionConflict {
            expected_revision,
            actual_revision,
        } => contract_error(
            "eitmad.error.customer-revision-conflict.v1",
            "eitmad.message.customer-revision-conflict.v1",
            context.correlation_id,
            RetryDisposition::SafeImmediately,
            Some(ErrorDetail::RevisionConflict {
                expected: expected_revision.unwrap_or(0),
                actual: actual_revision.unwrap_or(0),
            }),
        ),
        CustomerError::Unavailable => contract_error(
            "eitmad.error.customer-unavailable.v1",
            "eitmad.message.customer-unavailable.v1",
            context.correlation_id,
            RetryDisposition::SafeAfterDelay(1_000),
            None,
        ),
        CustomerError::UnsupportedScope | CustomerError::IdempotencyMismatch => {
            unsupported(context)
        }
    }
}

fn material_error(value: MaterialError, context: &DispatchContext) -> ContractError {
    let (code, message, retry, detail) = match value {
        MaterialError::Denied => (
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
            RetryDisposition::Never,
            None,
        ),
        MaterialError::Invalid => (
            "eitmad.error.material-invalid.v1",
            "eitmad.message.material-invalid.v1",
            RetryDisposition::Never,
            None,
        ),
        MaterialError::InvalidReference => (
            "eitmad.error.material-reference-invalid.v1",
            "eitmad.message.material-reference-invalid.v1",
            RetryDisposition::Never,
            None,
        ),
        MaterialError::NotFound => (
            "eitmad.error.material-not-found.v1",
            "eitmad.message.material-not-found.v1",
            RetryDisposition::Never,
            None,
        ),
        MaterialError::RevisionConflict { expected, actual } => (
            "eitmad.error.material-revision-conflict.v1",
            "eitmad.message.material-revision-conflict.v1",
            RetryDisposition::SafeImmediately,
            Some(ErrorDetail::RevisionConflict {
                expected: expected.unwrap_or(0),
                actual: actual.unwrap_or(0),
            }),
        ),
        MaterialError::Unavailable => (
            "eitmad.error.material-unavailable.v1",
            "eitmad.message.material-unavailable.v1",
            RetryDisposition::SafeAfterDelay(1_000),
            None,
        ),
    };
    contract_error(code, message, context.correlation_id, retry, detail)
}

/// Maps domain failures to redacted versioned errors with revision details and safe retry disposition.
fn part_error(value: PartError, context: &DispatchContext) -> ContractError {
    let (code, message, retry, detail) = match value {
        PartError::Denied => (
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
            RetryDisposition::Never,
            None,
        ),
        PartError::Invalid => (
            "eitmad.error.part-invalid.v1",
            "eitmad.message.part-invalid.v1",
            RetryDisposition::Never,
            None,
        ),
        PartError::InvalidReference => (
            "eitmad.error.part-reference-invalid.v1",
            "eitmad.message.part-reference-invalid.v1",
            RetryDisposition::Never,
            None,
        ),
        PartError::NotFound => (
            "eitmad.error.part-not-found.v1",
            "eitmad.message.part-not-found.v1",
            RetryDisposition::Never,
            None,
        ),
        PartError::RevisionConflict { expected, actual } => (
            "eitmad.error.part-revision-conflict.v1",
            "eitmad.message.part-revision-conflict.v1",
            RetryDisposition::Never,
            Some(ErrorDetail::RevisionConflict {
                expected: expected.unwrap_or(0),
                actual: actual.unwrap_or(0),
            }),
        ),
        PartError::Unavailable => (
            "eitmad.error.part-unavailable.v1",
            "eitmad.message.part-unavailable.v1",
            RetryDisposition::SafeAfterDelay(1_000),
            None,
        ),
    };
    contract_error(code, message, context.correlation_id, retry, detail)
}

/// Maps product failures to stable contract errors without exposing internal storage details.
fn product_error(value: ProductError, context: &DispatchContext) -> ContractError {
    let (code, message, retry, detail) = match value {
        ProductError::Denied => (
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
            RetryDisposition::Never,
            None,
        ),
        ProductError::Invalid => (
            "eitmad.error.product-invalid.v1",
            "eitmad.message.product-invalid.v1",
            RetryDisposition::Never,
            None,
        ),
        ProductError::InvalidReference => (
            "eitmad.error.product-reference-invalid.v1",
            "eitmad.message.product-reference-invalid.v1",
            RetryDisposition::Never,
            None,
        ),
        ProductError::NotFound => (
            "eitmad.error.product-not-found.v1",
            "eitmad.message.product-not-found.v1",
            RetryDisposition::Never,
            None,
        ),
        ProductError::RevisionConflict { expected, actual } => (
            "eitmad.error.product-revision-conflict.v1",
            "eitmad.message.product-revision-conflict.v1",
            RetryDisposition::Never,
            Some(ErrorDetail::RevisionConflict {
                expected: expected.unwrap_or(0),
                actual: actual.unwrap_or(0),
            }),
        ),
        ProductError::Unavailable => (
            "eitmad.error.product-unavailable.v1",
            "eitmad.message.product-unavailable.v1",
            RetryDisposition::SafeAfterDelay(1_000),
            None,
        ),
    };
    contract_error(code, message, context.correlation_id, retry, detail)
}

/// Maps Furniture failures to stable contract errors without exposing stored values.
fn furniture_error(value: FurnitureError, context: &DispatchContext) -> ContractError {
    let (code, message, retry, detail) = match value {
        FurnitureError::Denied => (
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
            RetryDisposition::Never,
            None,
        ),
        FurnitureError::Invalid => (
            "eitmad.error.furniture-invalid.v1",
            "eitmad.message.furniture-invalid.v1",
            RetryDisposition::Never,
            None,
        ),
        FurnitureError::InvalidReference => (
            "eitmad.error.furniture-reference-invalid.v1",
            "eitmad.message.furniture-reference-invalid.v1",
            RetryDisposition::Never,
            None,
        ),
        FurnitureError::NotFound => (
            "eitmad.error.furniture-not-found.v1",
            "eitmad.message.furniture-not-found.v1",
            RetryDisposition::Never,
            None,
        ),
        FurnitureError::RevisionConflict { expected, actual } => (
            "eitmad.error.furniture-revision-conflict.v1",
            "eitmad.message.furniture-revision-conflict.v1",
            RetryDisposition::Never,
            Some(ErrorDetail::RevisionConflict {
                expected: expected.unwrap_or(0),
                actual: actual.unwrap_or(0),
            }),
        ),
        FurnitureError::Unavailable => (
            "eitmad.error.furniture-unavailable.v1",
            "eitmad.message.furniture-unavailable.v1",
            RetryDisposition::SafeAfterDelay(1_000),
            None,
        ),
    };
    contract_error(code, message, context.correlation_id, retry, detail)
}

fn desktop_account_error(
    error_value: DesktopAccountError,
    context: &DispatchContext,
) -> ContractError {
    match error_value {
        DesktopAccountError::Denied => contract_error(
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
            context.correlation_id,
            RetryDisposition::Never,
            None,
        ),
        DesktopAccountError::Invalid | DesktopAccountError::IdempotencyMismatch => contract_error(
            "eitmad.error.desktop-account-invalid.v1",
            "eitmad.message.desktop-account-invalid.v1",
            context.correlation_id,
            RetryDisposition::Never,
            None,
        ),
        DesktopAccountError::RevisionConflict { expected, actual } => contract_error(
            "eitmad.error.desktop-account-revision-conflict.v1",
            "eitmad.message.desktop-account-revision-conflict.v1",
            context.correlation_id,
            RetryDisposition::SafeImmediately,
            Some(ErrorDetail::RevisionConflict { expected, actual }),
        ),
        DesktopAccountError::LastUsableManager => contract_error(
            "eitmad.error.desktop-account-last-manager.v1",
            "eitmad.message.desktop-account-last-manager.v1",
            context.correlation_id,
            RetryDisposition::Never,
            None,
        ),
        DesktopAccountError::Unavailable => contract_error(
            "eitmad.error.desktop-account-unavailable.v1",
            "eitmad.message.desktop-account-unavailable.v1",
            context.correlation_id,
            RetryDisposition::SafeAfterDelay(1_000),
            None,
        ),
    }
}

fn configuration_error(
    error_value: ConfigurationError,
    context: &DispatchContext,
) -> ContractError {
    match error_value {
        ConfigurationError::Denied => contract_error(
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
            context.correlation_id,
            RetryDisposition::Never,
            None,
        ),
        ConfigurationError::RevisionConflict {
            expected_revision,
            actual_revision,
        } => contract_error(
            "eitmad.error.config-revision-conflict.v1",
            "eitmad.message.config-revision-conflict.v1",
            context.correlation_id,
            RetryDisposition::SafeImmediately,
            Some(ErrorDetail::RevisionConflict {
                expected: expected_revision,
                actual: actual_revision,
            }),
        ),
        ConfigurationError::Unavailable
        | ConfigurationError::FutureSchemaVersion
        | ConfigurationError::FutureFormatVersion => contract_error(
            "eitmad.error.config-unavailable.v1",
            "eitmad.message.config-unavailable.v1",
            context.correlation_id,
            RetryDisposition::SafeAfterDelay(1_000),
            None,
        ),
        ConfigurationError::IdempotencyMismatch => unsupported(context),
        ConfigurationError::UnsupportedScope
        | ConfigurationError::EmptyPatch
        | ConfigurationError::TooManyChanges
        | ConfigurationError::DuplicateKey
        | ConfigurationError::UnknownKey
        | ConfigurationError::WrongValueKind
        | ConfigurationError::InvalidValue
        | ConfigurationError::NonCanonicalValue
        | ConfigurationError::ImportTooLarge
        | ConfigurationError::ImportMalformed => contract_error(
            "eitmad.error.config-invalid.v1",
            "eitmad.message.config-invalid.v1",
            context.correlation_id,
            RetryDisposition::Never,
            None,
        ),
    }
}

fn authorization_error(
    error_value: AuthorizationError,
    context: &DispatchContext,
) -> ContractError {
    authorization_contract_error(error_value, context.correlation_id, Some(context))
}

fn authorization_contract_error(
    error_value: AuthorizationError,
    correlation_id: eitmad_contracts::transport::CorrelationId,
    context: Option<&DispatchContext>,
) -> ContractError {
    match error_value {
        AuthorizationError::Denied | AuthorizationError::UnsupportedScope => contract_error(
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
            correlation_id,
            RetryDisposition::Never,
            None,
        ),
        AuthorizationError::PolicyConflict {
            expected_version,
            actual_version,
        } => contract_error(
            "eitmad.error.authorization-policy-conflict.v1",
            "eitmad.message.authorization-policy-conflict.v1",
            correlation_id,
            RetryDisposition::SafeImmediately,
            Some(ErrorDetail::RevisionConflict {
                expected: expected_version,
                actual: actual_version,
            }),
        ),
        AuthorizationError::LastOwner => contract_error(
            "eitmad.error.authorization-last-owner.v1",
            "eitmad.message.authorization-last-owner.v1",
            correlation_id,
            RetryDisposition::Never,
            None,
        ),
        AuthorizationError::InvalidRelation | AuthorizationError::RelationshipNotFound => {
            contract_error(
                "eitmad.error.authorization-relation-invalid.v1",
                "eitmad.message.authorization-relation-invalid.v1",
                correlation_id,
                RetryDisposition::Never,
                None,
            )
        }
        AuthorizationError::IdempotencyMismatch => context.map_or_else(
            || {
                contract_error(
                    "eitmad.error.contract-invalid.v1",
                    "eitmad.message.contract-invalid.v1",
                    correlation_id,
                    RetryDisposition::Never,
                    None,
                )
            },
            unsupported,
        ),
        AuthorizationError::BootstrapUnavailable | AuthorizationError::Unavailable => {
            contract_error(
                "eitmad.error.authorization-unavailable.v1",
                "eitmad.message.authorization-unavailable.v1",
                correlation_id,
                RetryDisposition::SafeAfterDelay(1_000),
                None,
            )
        }
    }
}

fn unsupported(context: &DispatchContext) -> ContractError {
    error(
        "eitmad.error.contract-invalid.v1",
        "eitmad.message.contract-invalid.v1",
        context,
        RetryDisposition::Never,
        None,
    )
}

fn error(
    code: &str,
    message: &str,
    context: &DispatchContext,
    retry: RetryDisposition,
    detail: Option<ErrorDetail>,
) -> ContractError {
    contract_error(code, message, context.correlation_id, retry, detail)
}

fn contract_error(
    code: &str,
    message: &str,
    correlation_id: eitmad_contracts::transport::CorrelationId,
    retry: RetryDisposition,
    detail: Option<ErrorDetail>,
) -> ContractError {
    ContractError {
        code: ErrorCode::parse(code).expect("static error code is valid"),
        message_id: MessageId::parse(message).expect("static message ID is valid"),
        parameters: Vec::new(),
        retry,
        correlation_id,
        detail,
    }
}

/// Preserves permanent failures, retryable unavailability, and the IPC deadline outcome.
fn image_error(
    value: eitmad_catalog_image::ImageError,
    context: &DispatchContext,
) -> ContractError {
    let (code, message) = match value {
        eitmad_catalog_image::ImageError::Denied => (
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
        ),
        eitmad_catalog_image::ImageError::Invalid => (
            "eitmad.error.catalog-image-invalid.v1",
            "eitmad.message.catalog-image-invalid.v1",
        ),
        eitmad_catalog_image::ImageError::NotFound => (
            "eitmad.error.catalog-image-not-found.v1",
            "eitmad.message.catalog-image-not-found.v1",
        ),
        eitmad_catalog_image::ImageError::Unavailable => (
            "eitmad.error.catalog-image-unavailable.v1",
            "eitmad.message.catalog-image-unavailable.v1",
        ),
    };
    if value == eitmad_catalog_image::ImageError::Unavailable && now().0 >= context.deadline.0 {
        return error(
            "eitmad.error.ipc-deadline-exceeded.v1",
            "eitmad.message.ipc-deadline-exceeded.v1",
            context,
            RetryDisposition::SafeAfterDelay(250),
            Some(ErrorDetail::Deadline {
                deadline: context.deadline,
            }),
        );
    }
    let retry = if value == eitmad_catalog_image::ImageError::Unavailable {
        RetryDisposition::SafeAfterDelay(250)
    } else {
        RetryDisposition::Never
    };
    error(code, message, context, retry, None)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use eitmad_contracts::{
        accounts::{AccountPassword, CreateDesktopAccount, DesktopAccountRole},
        authorization::{RelationId, RelationshipSubject},
        commands::{CreateCustomer, GrantScopeRelationship, UpdateConfiguration},
        config::{ConfigChange, ConfigKey, ConfigWriteValue},
        customer::{
            CustomerName, CustomerPhone, CustomerSearchTerm, CustomerSyncState, GetCustomer,
            SearchCustomers,
        },
        events::{
            AuthorizationPolicyChanges, ConfigurationChanges, CustomerChanges, MaterialChanges,
            Subscription,
        },
        identity::{
            AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind, ScopeId,
            ScopeKind, ScopeRef, SessionId, TenantId,
        },
        material::{
            ListMaterials, Material, MaterialUnit, SaveMaterial, SaveMaterialCategory,
            SaveMaterialUnit, UnitDimension,
        },
        part::{ListParts, PartChanges, PartUsage, SavePart, SavePartCategory},
        queries::{GetConfiguration, Query},
        transport::{CorrelationId, IdempotencyKey, PROTOCOL_VERSION, UnixMillis},
    };
    use rusqlite::Connection;
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;

    struct FailOncePublisher {
        broker: EventBroker,
        fail_next: AtomicBool,
    }

    impl ProductEventPublisher for FailOncePublisher {
        fn publish(
            &self,
            scope: eitmad_contracts::identity::ScopeRef,
            event: Event,
        ) -> Result<(), ()> {
            if self.fail_next.swap(false, Ordering::SeqCst) {
                return Err(());
            }
            self.broker
                .publish(scope, event)
                .map(|_| ())
                .map_err(|_| ())
        }

        fn policy_changed(&self, scope: eitmad_contracts::identity::ScopeRef) {
            self.broker.policy_changed(scope);
        }
    }

    fn authorization() -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::from_u128(4)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::from_u128(1)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(Uuid::from_u128(2)),
            workspace_id: None,
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(Uuid::from_u128(2)),
            },
        }
    }

    fn context(idempotency: u128) -> DispatchContext {
        DispatchContext {
            authorization: authorization(),
            correlation_id: CorrelationId::new(Uuid::from_u128(3)),
            causation_id: None,
            idempotency_key: Some(IdempotencyKey::new(Uuid::from_u128(idempotency))),
            protocol_version: PROTOCOL_VERSION,
            deadline: UnixMillis(i64::MAX),
        }
    }

    fn branch_authorization() -> AuthorizationContext {
        let mut authorization = authorization();
        authorization.scope = ScopeRef {
            kind: ScopeKind::parse("branch").unwrap(),
            id: ScopeId::new(Uuid::from_u128(500)),
        };
        authorization
    }

    fn branch_context(idempotency: u128) -> DispatchContext {
        let mut context = context(idempotency);
        context.authorization = branch_authorization();
        context
    }

    fn dispatcher() -> (TempDir, ProductDispatcher, EventBroker) {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let broker = EventBroker::new();
        let dispatcher = ProductDispatcher::new(store, broker.clone());
        let auth = authorization();
        dispatcher
            .authorization()
            .bootstrap_owner(
                &MutationContext {
                    authorization: auth.clone(),
                    correlation_id: CorrelationId::new(Uuid::from_u128(8)),
                    causation_id: None,
                    idempotency_key: IdempotencyKey::new(Uuid::from_u128(9)),
                    occurred_at: UnixMillis(1),
                },
                &RelationshipSubject {
                    principal_id: auth.identity.principal_id,
                    principal_kind: auth.identity.principal_kind,
                },
            )
            .unwrap();
        (directory, dispatcher, broker)
    }

    fn last_audit_outcome(dispatcher: &ProductDispatcher, operation: &str) -> AuditOutcome {
        let connection = Connection::open(dispatcher.store.path()).unwrap();
        let encoded = connection
            .query_row(
                "SELECT outcome FROM mutation_audit WHERE operation = ?1 ORDER BY rowid DESC LIMIT 1",
                [operation],
                |row| row.get::<_, String>(0),
            )
            .unwrap();
        serde_json::from_str(&encoded).unwrap()
    }

    use eitmad_catalog_image::{CatalogImageTransfer, ImageError};
    use eitmad_contracts::catalog_image::{CatalogImageKind, CatalogImageRef, GetCatalogImage};
    struct BlockingImageTransfer {
        entered: std::sync::mpsc::Sender<()>,
        release: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    }
    impl CatalogImageTransfer for BlockingImageTransfer {
        fn upload(
            &self,
            _: &AuthorizationContext,
            _: &CatalogImageRef,
            _: &[u8],
        ) -> Result<(), ImageError> {
            Ok(())
        }
        fn download(
            &self,
            _: &AuthorizationContext,
            _: &CatalogImageRef,
            _: UnixMillis,
        ) -> Result<Vec<u8>, ImageError> {
            self.entered.send(()).unwrap();
            let (lock, ready) = &*self.release;
            let released = lock.lock().unwrap();
            let _released = ready
                .wait_timeout_while(released, std::time::Duration::from_secs(5), |released| {
                    !*released
                })
                .unwrap();
            Err(ImageError::Unavailable)
        }
    }

    #[tokio::test]
    async fn discount_decision_direct_dispatch_denies_receptionist_in_branch_and_organization() {
        use eitmad_contracts::quotation_approval::*;
        let (_directory, dispatcher, _broker) = dispatcher();
        let actor = branch_authorization();
        let mutation = MutationContext {
            authorization: actor.clone(),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            causation_id: None,
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            occurred_at: UnixMillis(1),
        };
        dispatcher
            .authorization
            .bootstrap_owner(
                &mutation,
                &RelationshipSubject {
                    principal_id: actor.identity.principal_id,
                    principal_kind: PrincipalKind::User,
                },
            )
            .unwrap();
        dispatcher
            .authorization
            .grant_relationship(
                &MutationContext {
                    idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                    ..mutation
                },
                &GrantScopeRelationship {
                    expected_policy_version: 1,
                    subject: RelationshipSubject {
                        principal_id: actor.identity.principal_id,
                        principal_kind: PrincipalKind::User,
                    },
                    relation: RelationId::parse(eitmad_authorization::RECEPTIONIST_RELATION)
                        .unwrap(),
                },
            )
            .unwrap();
        let command = Command::DecideDiscountApproval(DecideDiscountApproval {
            draft_id: eitmad_contracts::quotation_draft::QuotationDraftId::new(Uuid::new_v4()),
            request_id: DiscountRequestId::new(Uuid::new_v4()),
            quotation_revision: 1,
            expected_revision: 1,
            fingerprint: "forged".into(),
            decision: DiscountDecision::Approve,
            reason: None,
        });
        for context in [branch_context(9801), context(9802)] {
            let error = dispatcher
                .dispatch_command(context, command.clone())
                .await
                .unwrap_err();
            assert_eq!(error.code.as_str(), "eitmad.error.authorization-denied.v1");
        }
    }

    #[tokio::test]
    async fn abandoned_image_requests_hold_worker_slots_until_blocking_work_finishes() {
        let (_directory, dispatcher, _broker) = dispatcher();
        dispatcher
            .authorization
            .grant_relationship(
                &ProductDispatcher::mutation_context(&context(9001)).unwrap(),
                &GrantScopeRelationship {
                    expected_policy_version: 1,
                    subject: RelationshipSubject {
                        principal_id: authorization().identity.principal_id,
                        principal_kind: PrincipalKind::User,
                    },
                    relation: RelationId::parse(eitmad_authorization::MANAGER_RELATION).unwrap(),
                },
            )
            .unwrap();
        let (entered, received) = std::sync::mpsc::channel();
        let release = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        let dispatcher = dispatcher.with_catalog_image_transfer(Arc::new(BlockingImageTransfer {
            entered,
            release: Arc::clone(&release),
        }));
        let query = Query::CatalogImage(GetCatalogImage {
            reference: CatalogImageRef {
                id: Uuid::new_v4(),
                kind: CatalogImageKind::Product,
                sha256: "00".repeat(32),
            },
            offset: 0,
        });
        let mut requests = Vec::new();
        for _ in 0..2 {
            let worker = dispatcher.clone();
            let query = query.clone();
            requests.push(tokio::spawn(async move {
                worker.dispatch_query(context(9002), query).await
            }));
        }
        tokio::task::spawn_blocking(move || {
            for _ in 0..2 {
                received
                    .recv_timeout(std::time::Duration::from_secs(3))
                    .unwrap();
            }
        })
        .await
        .unwrap();
        for request in requests {
            request.abort();
            let _ = request.await;
        }
        assert_eq!(dispatcher.image_workers.available_permits(), 0);
        let failure = dispatcher
            .dispatch_query(context(9003), query)
            .await
            .unwrap_err();
        assert_eq!(failure.retry, RetryDisposition::SafeAfterDelay(250));
        let import =
            Command::ImportCatalogImage(eitmad_contracts::catalog_image::ImportCatalogImage {
                kind: CatalogImageKind::Product,
                source_path: "synthetic-missing.png".into(),
            });
        let failure = dispatcher
            .dispatch_command(context(9004), import)
            .await
            .unwrap_err();
        assert_eq!(
            failure.code.as_str(),
            "eitmad.error.catalog-image-unavailable.v1"
        );
        *release.0.lock().unwrap() = true;
        release.1.notify_all();
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while dispatcher.image_workers.available_permits() != 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            image_error(ImageError::Invalid, &context(9005)).retry,
            RetryDisposition::Never
        );
    }

    #[tokio::test]
    async fn dispatcher_persists_invalid_command_and_query_outcomes() {
        let (_directory, dispatcher, _broker) = dispatcher();

        let invalid_command = dispatcher
            .dispatch_command(
                context(200),
                Command::UpdateConfiguration(UpdateConfiguration {
                    expected_revision: 0,
                    changes: Vec::new(),
                }),
            )
            .await;
        assert!(invalid_command.is_err());
        assert_eq!(
            last_audit_outcome(&dispatcher, "eitmad.config.update.v1"),
            AuditOutcome::Invalid
        );

        dispatcher
            .dispatch_query(context(202), Query::Configuration(GetConfiguration {}))
            .await
            .unwrap();
        assert_eq!(
            last_audit_outcome(&dispatcher, "eitmad.config.get.v1"),
            AuditOutcome::Succeeded
        );

        let mut denied_context = context(203);
        denied_context.authorization.identity.principal_id = PrincipalId::new(Uuid::from_u128(204));
        let denied = dispatcher
            .dispatch_query(denied_context, Query::Configuration(GetConfiguration {}))
            .await;
        assert_eq!(
            denied.unwrap_err().code.as_str(),
            "eitmad.error.authorization-denied.v1"
        );
        assert_eq!(
            last_audit_outcome(&dispatcher, "eitmad.config.get.v1"),
            AuditOutcome::Denied
        );

        let failed = dispatcher
            .dispatch_query(
                context(205),
                Query::Customer(GetCustomer {
                    customer_id: eitmad_contracts::customer::CustomerId::new(Uuid::from_u128(206)),
                }),
            )
            .await;
        assert!(failed.is_err());
        assert_eq!(
            last_audit_outcome(&dispatcher, "eitmad.customer.get.v1"),
            AuditOutcome::Failed
        );
    }

    #[tokio::test]
    async fn audit_store_failure_withholds_the_original_query_result() {
        let (_directory, dispatcher, _broker) = dispatcher();
        Connection::open(dispatcher.store.path())
            .unwrap()
            .execute_batch("DROP TABLE mutation_audit")
            .unwrap();

        let error = dispatcher
            .dispatch_query(context(210), Query::Configuration(GetConfiguration {}))
            .await
            .unwrap_err();
        assert_eq!(
            error.code.as_str(),
            "eitmad.error.authorization-unavailable.v1"
        );
    }

    #[tokio::test]
    async fn routes_configuration_query_patch_and_post_commit_event() {
        let (_directory, dispatcher, broker) = dispatcher();
        let snapshot = dispatcher
            .dispatch_query(context(10), Query::Configuration(GetConfiguration {}))
            .await
            .unwrap();
        assert!(matches!(snapshot, QueryResult::Configuration(_)));
        let (_, mut events) = broker
            .subscribe(
                authorization().scope,
                Subscription::Configuration(ConfigurationChanges {}),
                None,
            )
            .unwrap();
        let result = dispatcher
            .dispatch_command(
                context(11),
                Command::UpdateConfiguration(UpdateConfiguration {
                    expected_revision: 0,
                    changes: vec![ConfigChange {
                        key: ConfigKey::parse("eitmad.config.locale.primary.v1").unwrap(),
                        value: ConfigWriteValue::Text("en-US".to_owned()),
                    }],
                }),
            )
            .await
            .unwrap();
        let CommandResult::ConfigurationUpdated(snapshot) = result else {
            panic!("configuration result expected")
        };
        assert_eq!(snapshot.revision, 1);
        assert!(matches!(
            events.recv().await.unwrap().event,
            Event::ConfigurationChanged(_)
        ));
    }

    #[tokio::test]
    async fn routes_customer_create_search_and_compact_event() {
        let (_directory, dispatcher, broker) = dispatcher();
        let authorization = branch_authorization();
        authorize_customer_branch(&dispatcher, &authorization);
        let (_, mut events) = broker
            .subscribe(
                authorization.scope.clone(),
                Subscription::Customers(CustomerChanges {}),
                None,
            )
            .unwrap();

        let result = dispatcher
            .dispatch_command(
                branch_context(505),
                Command::CreateCustomer(CreateCustomer {
                    name: CustomerName::parse("إعـتماد القيسي").unwrap(),
                    phone: CustomerPhone::parse("+٩٦٧ ٧٧٧ ١٢٣ ٤٥٦").unwrap(),
                    address: None,
                    notes: None,
                }),
            )
            .await
            .unwrap();
        let CommandResult::CustomerCreated(created) = result else {
            panic!("customer result expected")
        };
        let Event::CustomerChanged(notice) = events.recv().await.unwrap().event else {
            panic!("customer event expected")
        };
        assert_eq!(notice.customer_id, created.customer.id);

        let result = dispatcher
            .dispatch_query(
                branch_context(506),
                Query::Customers(
                    SearchCustomers::new(CustomerSearchTerm::parse("اعتماد").unwrap(), None, 10)
                        .unwrap(),
                ),
            )
            .await
            .unwrap();
        let QueryResult::Customers(page) = result else {
            panic!("customer page expected")
        };
        assert_eq!(page.items, vec![created.customer]);
        assert_eq!(
            last_audit_outcome(&dispatcher, "eitmad.customer.search.v1"),
            AuditOutcome::Succeeded
        );
        let change = dispatcher
            .customers
            .sync_batch(&authorization.scope, 1)
            .unwrap()
            .remove(0);
        dispatcher
            .customers
            .project_confirmed(
                &authorization,
                &change,
                CorrelationId::new(Uuid::from_u128(507)),
            )
            .unwrap();
        dispatcher.drain_pending_publications().unwrap();
        let Event::CustomerChanged(confirmed_notice) = events.recv().await.unwrap().event else {
            panic!("confirmed customer event expected")
        };
        assert_eq!(confirmed_notice.change_id, change.change_id);
        assert_eq!(
            dispatcher
                .customers
                .get(
                    &authorization,
                    &GetCustomer {
                        customer_id: notice.customer_id,
                    },
                )
                .unwrap()
                .sync_state,
            CustomerSyncState::Confirmed
        );
    }

    /// Exercises typed material and part routes, committed notices, and Receptionist write denial.
    #[tokio::test]
    async fn routes_material_and_part_mutations_and_denies_receptionist_write() {
        let (_directory, dispatcher, broker) = dispatcher();
        grant_material_roles(&dispatcher);
        let (_, mut events) = broker
            .subscribe(
                authorization().scope,
                Subscription::Materials(MaterialChanges {}),
                None,
            )
            .unwrap();
        let CommandResult::MaterialCategorySaved(category) = dispatcher
            .dispatch_command(
                material_actor(610, 3),
                Command::SaveMaterialCategory(SaveMaterialCategory {
                    id: None,
                    expected_revision: None,
                    name: "أخشاب طبيعية".to_owned(),
                    archived: false,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("category expected")
        };
        let Event::MaterialChanged(category_notice) = events.recv().await.unwrap().event else {
            panic!("material event expected")
        };
        assert_eq!(category_notice.id, category.id.value());
        let CommandResult::MaterialUnitSaved(unit) = dispatcher
            .dispatch_command(
                material_actor(611, 3),
                Command::SaveMaterialUnit(SaveMaterialUnit {
                    id: None,
                    expected_revision: None,
                    name: "متر".to_owned(),
                    symbol: "م".to_owned(),
                    dimension: UnitDimension::Length,
                    numerator: 1,
                    denominator: 1,
                    archived: false,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("unit expected")
        };
        let _ = events.recv().await.unwrap();
        let CommandResult::MaterialSaved(material) = dispatcher
            .dispatch_command(
                material_actor(612, 3),
                Command::SaveMaterial(SaveMaterial {
                    id: None,
                    expected_revision: None,
                    name: "خشب زان".to_owned(),
                    category_id: category.id,
                    unit_id: unit.id,
                    current_cost_yer: 8_000,
                    archived: false,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("material expected")
        };
        let Event::MaterialChanged(notice) = events.recv().await.unwrap().event else {
            panic!("material event expected")
        };
        assert_eq!(notice.id, material.id.value());
        let QueryResult::Materials(page) = dispatcher
            .dispatch_query(
                material_actor(613, 3),
                Query::Materials(ListMaterials::new("اخشاب".to_owned(), None, 20).unwrap()),
            )
            .await
            .unwrap()
        else {
            panic!("page expected")
        };
        assert_eq!(page.items.len(), 1);
        let denied = dispatcher
            .dispatch_command(
                material_actor(614, 4),
                Command::SaveMaterialCategory(SaveMaterialCategory {
                    id: None,
                    expected_revision: None,
                    name: "ممنوع".to_owned(),
                    archived: false,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(denied.code.as_str(), "eitmad.error.authorization-denied.v1");
        assert_part_routes(&dispatcher, &broker, &material, &unit).await;
    }

    /// Checks part category and composition saves, scoped search, current costs, and write denial through dispatch.
    async fn assert_part_routes(
        dispatcher: &ProductDispatcher,
        broker: &EventBroker,
        material: &Material,
        unit: &MaterialUnit,
    ) {
        let (_, mut part_events) = broker
            .subscribe(
                authorization().scope,
                Subscription::Parts(PartChanges {}),
                None,
            )
            .unwrap();
        let CommandResult::PartCategorySaved(part_category) = dispatcher
            .dispatch_command(
                material_actor(621, 3),
                Command::SavePartCategory(SavePartCategory {
                    id: None,
                    expected_revision: None,
                    name: "خزانة".into(),
                    archived: false,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("part category expected")
        };
        let input = SavePart {
            id: None,
            expected_revision: None,
            name: "جانب خزانة".into(),
            category_id: part_category.id,
            description: String::new(),
            archived: false,
            usages: vec![PartUsage {
                material_id: material.id,
                material_revision: material.revision,
                unit_id: unit.id,
                unit_revision: unit.revision,
                quantity: eitmad_contracts::material::MaterialQuantity::parse("1.2".into())
                    .unwrap(),
            }],
        };
        let CommandResult::PartSaved(part) = dispatcher
            .dispatch_command(material_actor(622, 3), Command::SavePart(input.clone()))
            .await
            .unwrap()
        else {
            panic!("part expected")
        };
        let _category_event = part_events.recv().await.unwrap();
        let Event::PartChanged(notice) = part_events.recv().await.unwrap().event else {
            panic!("part event expected")
        };
        assert_eq!(notice.id, part.id.value());
        let QueryResult::Parts(parts) = dispatcher
            .dispatch_query(
                material_actor(623, 3),
                Query::Parts(ListParts {
                    term: "خزانه".into(),
                    after: None,
                    limit: 20,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("parts expected")
        };
        assert_eq!(parts.items[0].part.id, part.id);
        assert_eq!(parts.items[0].current_cost.total_cost_yer, 9600);
        let denied_part = dispatcher
            .dispatch_command(material_actor(624, 4), Command::SavePart(input))
            .await
            .unwrap_err();
        assert_eq!(
            denied_part.code.as_str(),
            "eitmad.error.authorization-denied.v1"
        );
    }

    fn grant_material_roles(dispatcher: &ProductDispatcher) {
        for (key, principal, relation, version) in [
            (601, 3, "eitmad.relation.organization.manager.v1", 1),
            (602, 4, "eitmad.relation.organization.receptionist.v1", 2),
        ] {
            dispatcher
                .authorization()
                .grant_relationship(
                    &ProductDispatcher::mutation_context(&context(key)).unwrap(),
                    &GrantScopeRelationship {
                        expected_policy_version: version,
                        subject: RelationshipSubject {
                            principal_id: PrincipalId::new(Uuid::from_u128(principal)),
                            principal_kind: PrincipalKind::User,
                        },
                        relation: RelationId::parse(relation).unwrap(),
                    },
                )
                .unwrap();
        }
    }

    fn material_actor(key: u128, principal: u128) -> DispatchContext {
        let mut actor = context(key);
        actor.authorization.identity.principal_id = PrincipalId::new(Uuid::from_u128(principal));
        actor
    }

    fn authorize_customer_branch(
        dispatcher: &ProductDispatcher,
        authorization: &AuthorizationContext,
    ) {
        dispatcher
            .authorization()
            .bootstrap_owner(
                &MutationContext {
                    authorization: authorization.clone(),
                    correlation_id: CorrelationId::new(Uuid::from_u128(501)),
                    causation_id: None,
                    idempotency_key: IdempotencyKey::new(Uuid::from_u128(502)),
                    occurred_at: UnixMillis(1),
                },
                &RelationshipSubject {
                    principal_id: authorization.identity.principal_id,
                    principal_kind: authorization.identity.principal_kind,
                },
            )
            .unwrap();
        dispatcher
            .authorization()
            .grant_relationship(
                &MutationContext {
                    authorization: authorization.clone(),
                    correlation_id: CorrelationId::new(Uuid::from_u128(503)),
                    causation_id: None,
                    idempotency_key: IdempotencyKey::new(Uuid::from_u128(504)),
                    occurred_at: UnixMillis(2),
                },
                &GrantScopeRelationship {
                    expected_policy_version: 1,
                    subject: RelationshipSubject {
                        principal_id: authorization.identity.principal_id,
                        principal_kind: authorization.identity.principal_kind,
                    },
                    relation: RelationId::parse(eitmad_authorization::MANAGER_RELATION).unwrap(),
                },
            )
            .unwrap();
    }

    #[tokio::test]
    async fn no_op_and_failed_patches_publish_no_event() {
        let (_directory, dispatcher, broker) = dispatcher();
        let (_, mut events) = broker
            .subscribe(
                authorization().scope,
                Subscription::Configuration(ConfigurationChanges {}),
                None,
            )
            .unwrap();
        dispatcher
            .dispatch_command(
                context(20),
                Command::UpdateConfiguration(UpdateConfiguration {
                    expected_revision: 0,
                    changes: vec![ConfigChange {
                        key: ConfigKey::parse("eitmad.config.locale.primary.v1").unwrap(),
                        value: ConfigWriteValue::Text("ar-YE".to_owned()),
                    }],
                }),
            )
            .await
            .unwrap();
        let failed = dispatcher
            .dispatch_command(
                context(21),
                Command::UpdateConfiguration(UpdateConfiguration {
                    expected_revision: 9,
                    changes: vec![ConfigChange {
                        key: ConfigKey::parse("eitmad.config.locale.primary.v1").unwrap(),
                        value: ConfigWriteValue::Text("en-US".to_owned()),
                    }],
                }),
            )
            .await;
        assert!(failed.is_err());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), events.recv())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn relationship_mutation_publishes_one_policy_event_not_on_replay() {
        let (_directory, dispatcher, broker) = dispatcher();
        let (_, mut events) = broker
            .subscribe(
                authorization().scope,
                Subscription::AuthorizationPolicy(AuthorizationPolicyChanges {}),
                None,
            )
            .unwrap();
        let command = Command::GrantScopeRelationship(GrantScopeRelationship {
            expected_policy_version: 1,
            subject: RelationshipSubject {
                principal_id: PrincipalId::new(Uuid::from_u128(55)),
                principal_kind: PrincipalKind::Service,
            },
            relation: RelationId::parse("eitmad.relation.organization.member.v1").unwrap(),
        });
        let first = dispatcher
            .dispatch_command(context(70), command.clone())
            .await
            .unwrap();
        let CommandResult::RelationshipGranted(first) = first else {
            panic!("relationship result expected")
        };
        assert!(first.changed);
        let published = events.recv().await.unwrap();
        let Event::AuthorizationPolicyChanged(notice) = published.event else {
            panic!("policy event expected")
        };
        assert_eq!(notice.policy_version, 2);

        let replay = dispatcher
            .dispatch_command(context(70), command)
            .await
            .unwrap();
        let CommandResult::RelationshipGranted(replay) = replay else {
            panic!("relationship result expected")
        };
        assert!(!replay.changed);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), events.recv())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn desktop_account_access_mutation_publishes_policy_event_not_on_replay() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let auth = store.local_authorization_context(UnixMillis(1)).unwrap();
        let broker = EventBroker::new();
        let dispatcher = ProductDispatcher::new(store, broker.clone());
        let command_context = |idempotency| DispatchContext {
            authorization: auth.clone(),
            correlation_id: CorrelationId::new(Uuid::from_u128(3)),
            causation_id: None,
            idempotency_key: Some(IdempotencyKey::new(Uuid::from_u128(idempotency))),
            protocol_version: PROTOCOL_VERSION,
            deadline: UnixMillis(i64::MAX),
        };
        dispatcher
            .dispatch_command(
                command_context(74),
                Command::GrantScopeRelationship(GrantScopeRelationship {
                    expected_policy_version: 1,
                    subject: RelationshipSubject {
                        principal_id: auth.identity.principal_id,
                        principal_kind: auth.identity.principal_kind,
                    },
                    relation: RelationId::parse("eitmad.relation.organization.manager.v1").unwrap(),
                }),
            )
            .await
            .unwrap();
        let (_, mut events) = broker
            .subscribe(
                auth.scope.clone(),
                Subscription::AuthorizationPolicy(AuthorizationPolicyChanges {}),
                None,
            )
            .unwrap();
        let command = Command::CreateDesktopAccount(CreateDesktopAccount {
            display_name: "سارة أحمد".to_owned(),
            username: "reception".to_owned(),
            password: AccountPassword::new("reception-password-1"),
            role: DesktopAccountRole::Receptionist,
        });

        let first = dispatcher
            .dispatch_command(command_context(75), command.clone())
            .await
            .unwrap();
        assert!(matches!(first, CommandResult::DesktopAccountCreated(_)));
        let published = tokio::time::timeout(std::time::Duration::from_secs(1), events.recv())
            .await
            .expect("account mutation did not publish a policy event")
            .unwrap();
        let Event::AuthorizationPolicyChanged(notice) = published.event else {
            panic!("policy event expected")
        };
        assert_eq!(notice.policy_version, 3);

        let replay = dispatcher
            .dispatch_command(command_context(75), command)
            .await
            .unwrap();
        assert_eq!(replay, first);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), events.recv())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn runtime_drain_replays_pending_publication_after_post_commit_failure() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let broker = EventBroker::new();
        let publisher = Arc::new(FailOncePublisher {
            broker: broker.clone(),
            fail_next: AtomicBool::new(true),
        });
        let dispatcher = ProductDispatcher::with_event_publisher(store.clone(), publisher);
        let auth = authorization();
        dispatcher
            .authorization()
            .bootstrap_owner(
                &MutationContext {
                    authorization: auth.clone(),
                    correlation_id: CorrelationId::new(Uuid::from_u128(8)),
                    causation_id: None,
                    idempotency_key: IdempotencyKey::new(Uuid::from_u128(9)),
                    occurred_at: UnixMillis(1),
                },
                &RelationshipSubject {
                    principal_id: auth.identity.principal_id,
                    principal_kind: auth.identity.principal_kind,
                },
            )
            .unwrap();
        let (_, mut events) = broker
            .subscribe(
                auth.scope.clone(),
                Subscription::Configuration(ConfigurationChanges {}),
                None,
            )
            .unwrap();
        let command = Command::UpdateConfiguration(UpdateConfiguration {
            expected_revision: 0,
            changes: vec![ConfigChange {
                key: ConfigKey::parse("eitmad.config.locale.primary.v1").unwrap(),
                value: ConfigWriteValue::Text("en-US".to_owned()),
            }],
        });
        let key = context(80).idempotency_key.unwrap();

        let first = dispatcher
            .dispatch_command(context(80), command.clone())
            .await;
        assert!(first.is_err());
        assert_eq!(store.read_configuration(&auth.scope).unwrap().revision, 1);
        assert!(
            store
                .pending_publication(&auth.scope, key)
                .unwrap()
                .is_some()
        );

        dispatcher.drain_pending_publications().unwrap();
        let retry = dispatcher
            .dispatch_command(context(80), command.clone())
            .await
            .unwrap();
        assert!(matches!(retry, CommandResult::ConfigurationUpdated(_)));
        assert!(matches!(
            events.recv().await.unwrap().event,
            Event::ConfigurationChanged(_)
        ));
        assert!(
            store
                .pending_publication(&auth.scope, key)
                .unwrap()
                .is_none()
        );

        dispatcher
            .dispatch_command(context(80), command)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), events.recv())
                .await
                .is_err()
        );
    }
    /// Proves transient sync failures cannot strand other actors or committed notifications.
    #[tokio::test]
    async fn catalog_retry_continues_other_actors_and_drains_after_failure() {
        use eitmad_observability_audit::{AuditTarget, MutationAuditRecord};
        use eitmad_pricing::PricingError;

        #[derive(Default)]
        struct InterruptedReplication(std::sync::Mutex<Vec<PrincipalId>>);
        impl eitmad_pricing::CatalogReplication for InterruptedReplication {
            fn synchronize(
                &self,
                actor: &AuthorizationContext,
                _: UnixMillis,
            ) -> Result<usize, PricingError> {
                let mut actors = self.0.lock().unwrap();
                actors.push(actor.identity.principal_id);
                match actors.len() {
                    1 => Err(PricingError::Unconfirmed),
                    2 => Err(PricingError::Invalid),
                    _ => Ok(2),
                }
            }
        }

        let (_directory, mut dispatcher, broker) = dispatcher();
        dispatcher.events = Arc::new(FailOncePublisher {
            broker: broker.clone(),
            fail_next: AtomicBool::new(true),
        });
        let auth = authorization();
        let (_, mut events) = broker
            .subscribe(
                auth.scope.clone(),
                Subscription::Configuration(ConfigurationChanges {}),
                None,
            )
            .unwrap();
        let mutation = context(910);
        let key = mutation.idempotency_key.unwrap();
        assert!(
            dispatcher
                .dispatch_command(
                    mutation,
                    Command::UpdateConfiguration(UpdateConfiguration {
                        expected_revision: 0,
                        changes: vec![ConfigChange {
                            key: ConfigKey::parse("eitmad.config.locale.primary.v1").unwrap(),
                            value: ConfigWriteValue::Text("en-US".into()),
                        }],
                    }),
                )
                .await
                .is_err()
        );
        assert!(
            dispatcher
                .store
                .pending_publication(&auth.scope, key)
                .unwrap()
                .is_some()
        );
        let expected = [1, 3, 4].map(|id| PrincipalId::new(Uuid::from_u128(id)));
        for principal in expected {
            let mut actor = auth.clone();
            actor.identity.principal_id = principal;
            let audit = MutationAuditRecord::from_authorization(
                &actor,
                now(),
                CorrelationId::new(Uuid::new_v4()),
                "eitmad.catalog.sync.register.v1",
                AuditTarget {
                    kind: "catalog-sync".into(),
                    identifiers: vec![],
                },
            );
            dispatcher
                .store
                .register_catalog_client(&actor, &audit)
                .unwrap();
        }
        let replication = Arc::new(InterruptedReplication::default());
        let dispatcher = dispatcher.with_catalog_replication(replication.clone());
        assert_eq!(
            dispatcher.retry_catalog_replication(),
            Err(PricingError::Unconfirmed)
        );
        let mut attempted = replication.0.lock().unwrap().clone();
        attempted.sort_by_key(|id| id.value());
        assert_eq!(attempted, expected);
        assert!(
            dispatcher
                .store
                .pending_publication(&auth.scope, key)
                .unwrap()
                .is_none()
        );
        let notice = tokio::time::timeout(std::time::Duration::from_secs(1), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(notice.event, Event::ConfigurationChanged(_)));
    }

    struct TestPriceServer;
    impl eitmad_pricing::PriceConfirmation for TestPriceServer {
        /// Accepts catalog transfer so dispatcher tests can isolate routing and receipt handling.
        fn synchronize_catalog(
            &self,
            _: &AuthorizationContext,
            _: &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions,
            _: UnixMillis,
        ) -> Result<(), eitmad_pricing::PricingError> {
            Ok(())
        }
        fn status(
            &self,
            _: &AuthorizationContext,
            _: &eitmad_contracts::pricing::ConfirmPrice,
            _: UnixMillis,
        ) -> Result<Option<eitmad_contracts::pricing::PublishedPrice>, eitmad_pricing::PricingError>
        {
            Ok(None)
        }

        fn read(
            &self,
            _: &AuthorizationContext,
            _: &eitmad_contracts::pricing::ReadPublishedPrices,
            _: UnixMillis,
        ) -> Result<eitmad_contracts::pricing::PublishedPricePage, eitmad_pricing::PricingError>
        {
            Ok(eitmad_contracts::pricing::PublishedPricePage {
                items: vec![],
                next: None,
            })
        }
        fn confirm(
            &self,
            _: &AuthorizationContext,
            input: &eitmad_contracts::pricing::ConfirmPrice,
            _: UnixMillis,
        ) -> Result<eitmad_contracts::pricing::PublishedPrice, eitmad_pricing::PricingError>
        {
            Ok(eitmad_contracts::pricing::PublishedPrice {
                target: input.command.target.clone(),
                currency: "YER".into(),
                selling_price_yer: input.command.selling_price_yer,
                colors: input.colors.clone(),
                handles: input.handles.clone(),
                revision: input.command.expected_revision.unwrap_or(0) + 1,
                confirmed_at: UnixMillis(1000),
            })
        }
    }
    struct SlowPriceServer;
    impl eitmad_pricing::PriceConfirmation for SlowPriceServer {
        /// Accepts catalog transfer without adding latency to the simulated slow price read.
        fn synchronize_catalog(
            &self,
            _: &AuthorizationContext,
            _: &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions,
            _: UnixMillis,
        ) -> Result<(), eitmad_pricing::PricingError> {
            Ok(())
        }
        fn read(
            &self,
            _: &AuthorizationContext,
            _: &eitmad_contracts::pricing::ReadPublishedPrices,
            deadline: UnixMillis,
        ) -> Result<eitmad_contracts::pricing::PublishedPricePage, eitmad_pricing::PricingError>
        {
            let remaining = u64::try_from(deadline.0.saturating_sub(now().0)).unwrap_or(0);
            std::thread::sleep(std::time::Duration::from_millis(remaining));
            Err(eitmad_pricing::PricingError::Unconfirmed)
        }
        fn status(
            &self,
            actor: &AuthorizationContext,
            input: &eitmad_contracts::pricing::ConfirmPrice,
            deadline: UnixMillis,
        ) -> Result<Option<eitmad_contracts::pricing::PublishedPrice>, eitmad_pricing::PricingError>
        {
            eitmad_pricing::PriceConfirmation::status(&TestPriceServer, actor, input, deadline)
        }
        fn confirm(
            &self,
            actor: &AuthorizationContext,
            input: &eitmad_contracts::pricing::ConfirmPrice,
            deadline: UnixMillis,
        ) -> Result<eitmad_contracts::pricing::PublishedPrice, eitmad_pricing::PricingError>
        {
            eitmad_pricing::PriceConfirmation::confirm(&TestPriceServer, actor, input, deadline)
        }
    }

    #[tokio::test]
    async fn pricing_slow_refresh_returns_confirmed_cache_before_query_deadline() {
        use eitmad_contracts::pricing::{ListPrices, PublishPrice};
        let (_directory, dispatcher, _) = dispatcher();
        grant_material_roles(&dispatcher);
        let dispatcher = dispatcher.with_price_confirmation(Arc::new(TestPriceServer));
        let target = pricing_fixture(&dispatcher).await;
        dispatcher
            .dispatch_command(
                material_actor(720, 3),
                Command::PublishPrice(PublishPrice {
                    target: target.clone(),
                    expected_revision: None,
                    selling_price_yer: 70000,
                    confirm_below_cost: false,
                }),
            )
            .await
            .unwrap();
        let dispatcher = dispatcher.with_price_confirmation(Arc::new(SlowPriceServer));
        for budget in [1_300, 30_000] {
            let mut context = material_actor(721, 4);
            context.deadline = UnixMillis(now().0 + budget);
            let timeout =
                std::time::Duration::from_millis(u64::try_from(budget.min(3_000)).unwrap());
            let result = tokio::time::timeout(
                timeout,
                dispatcher.dispatch_query(
                    context,
                    Query::Prices(ListPrices {
                        term: String::new(),
                        after: None,
                        limit: 100,
                    }),
                ),
            )
            .await
            .expect("Refresh must leave time for the confirmed-cache response")
            .unwrap();
            let QueryResult::Prices(page) = result else {
                panic!("prices")
            };
            assert!(!page.server_available);
            assert_eq!(page.items.len(), 1);
            assert_eq!(page.items[0].target, target);
            assert_eq!(
                page.items[0].published.as_ref().unwrap().selling_price_yer,
                70000
            );
        }
    }

    struct FailedCatalogReplication(eitmad_pricing::PricingError);
    impl eitmad_pricing::CatalogReplication for FailedCatalogReplication {
        fn synchronize(
            &self,
            _: &AuthorizationContext,
            _: UnixMillis,
        ) -> Result<usize, eitmad_pricing::PricingError> {
            Err(self.0)
        }
    }

    #[tokio::test]
    async fn pricing_replication_failures_preserve_authorized_confirmed_cache() {
        use eitmad_contracts::pricing::{ListPrices, PublishPrice};
        let (_directory, dispatcher, _) = dispatcher();
        grant_material_roles(&dispatcher);
        let mut dispatcher = dispatcher.with_price_confirmation(Arc::new(TestPriceServer));
        let target = pricing_fixture(&dispatcher).await;
        dispatcher
            .dispatch_command(
                material_actor(820, 3),
                Command::PublishPrice(PublishPrice {
                    target: target.clone(),
                    expected_revision: None,
                    selling_price_yer: 70_000,
                    confirm_below_cost: false,
                }),
            )
            .await
            .unwrap();
        for failure in [
            eitmad_pricing::PricingError::Reference,
            eitmad_pricing::PricingError::Denied,
            eitmad_pricing::PricingError::Unconfirmed,
        ] {
            dispatcher =
                dispatcher.with_catalog_replication(Arc::new(FailedCatalogReplication(failure)));
            let query = Query::Prices(ListPrices {
                term: String::new(),
                after: None,
                limit: 100,
            });
            let QueryResult::Prices(page) = dispatcher
                .dispatch_query(material_actor(821, 3), query.clone())
                .await
                .unwrap()
            else {
                panic!("prices")
            };
            assert!(!page.server_available);
            assert_eq!(
                page.items[0].published.as_ref().unwrap().selling_price_yer,
                70_000
            );
            assert_eq!(
                dispatcher
                    .dispatch_query(material_actor(822, 999), query)
                    .await
                    .unwrap_err()
                    .code
                    .as_str(),
                "eitmad.error.authorization-denied.v1"
            );
        }
    }

    async fn pricing_fixture(
        dispatcher: &ProductDispatcher,
    ) -> eitmad_contracts::pricing::PriceTarget {
        use eitmad_contracts::{
            pricing::PriceTarget,
            product::{
                ProductReference, ProductVariantId, SaveProduct, SaveProductCategory,
                SaveProductVariant,
            },
        };
        let CommandResult::ProductCategorySaved(category) = dispatcher
            .dispatch_command(
                material_actor(710, 3),
                Command::SaveProductCategory(SaveProductCategory {
                    id: None,
                    expected_revision: None,
                    name: "مراتب".into(),
                    archived: false,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("category")
        };
        let CommandResult::ProductSaved(product) = dispatcher
            .dispatch_command(
                material_actor(711, 3),
                Command::SaveProduct(SaveProduct {
                    id: None,
                    expected_revision: None,
                    image: None,
                    name: "مرتبة".into(),
                    category_id: category.id,
                    description: "جاهزة".into(),
                    notes: "ملاحظة داخلية".into(),
                    archived: false,
                    variants: vec![SaveProductVariant {
                        id: ProductVariantId::new(Uuid::from_u128(712)),
                        name: "مفرد".into(),
                        purchase_cost_yer: 55000,
                        archived: false,
                    }],
                }),
            )
            .await
            .unwrap()
        else {
            panic!("product")
        };
        PriceTarget::Product(ProductReference {
            scope: product.scope,
            product_id: product.id,
            variant_id: product.variants[0].id,
            revision: product.revision,
            schema_version: 1,
        })
    }
    /// Projects a synthetic server-confirmed public revision into the separate receptionist client store.
    fn project_catalog_entry(
        receiver: &ProductDispatcher,
        entry: &eitmad_contracts::catalog_revision::CatalogEntry,
        actor: &AuthorizationContext,
    ) {
        use eitmad_contracts::{
            sync::{ChangeId, ChangeOperation, ChangeRecord, Checkpoint, RecordId},
            transport::SchemaId,
        };
        use eitmad_observability_audit::{AuditTarget, MutationAuditRecord};
        let change = ChangeRecord {
            change_id: ChangeId::new(Uuid::new_v4()),
            record_id: RecordId::new(Uuid::new_v4()),
            scope: actor.scope.clone(),
            operation: ChangeOperation::Upsert,
            base_revision: None,
            revision: 1,
            changed_at: now(),
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            payload: None,
            merge: None,
        };
        let audit = MutationAuditRecord::from_authorization(
            actor,
            now(),
            CorrelationId::new(Uuid::new_v4()),
            "eitmad.catalog.sync.project.v1",
            AuditTarget {
                kind: "catalog-sync".into(),
                identifiers: vec![],
            },
        );
        receiver
            .store
            .project_catalog_page(
                actor,
                &SchemaId::parse("eitmad.schema.catalog-public.v1").unwrap(),
                Checkpoint::new(Uuid::new_v4()),
                &eitmad_storage::CatalogSyncProjection {
                    private: &[],
                    public: &[(change, Some(entry.clone()))],
                    normalize_name: eitmad_material::normalize_search,
                },
                &audit,
            )
            .unwrap();
    }

    /// Verifies receptionist reads and configuration checks work from another client publication without private definitions.
    #[tokio::test]
    async fn sales_catalog_dispatch_reads_another_client_publication_without_private_definitions() {
        use eitmad_contracts::{
            catalog_revision::CatalogRevision,
            pricing::{PriceSelection, PublishPrice},
            sales_catalog::{CheckSalesConfiguration, ListSalesCatalog},
        };
        let (_manager_dir, manager_dispatcher, _) = dispatcher();
        grant_material_roles(&manager_dispatcher);
        let manager_dispatcher =
            manager_dispatcher.with_price_confirmation(Arc::new(TestPriceServer));
        let target = pricing_fixture(&manager_dispatcher).await;
        let CommandResult::PricePublished(price) = manager_dispatcher
            .dispatch_command(
                material_actor(730, 3),
                Command::PublishPrice(PublishPrice {
                    target: target.clone(),
                    expected_revision: None,
                    selling_price_yer: 70000,
                    confirm_below_cost: false,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("price")
        };
        let eitmad_contracts::pricing::PriceTarget::Product(reference) = &target else {
            panic!("product")
        };
        let product = manager_dispatcher
            .products
            .revision(
                &material_actor(731, 3).authorization,
                &eitmad_contracts::product::GetProductRevision {
                    reference: reference.clone(),
                    for_new_work: true,
                },
            )
            .unwrap();
        let entry =
            eitmad_pricing::public_entry(&CatalogRevision::Product(Box::new(product)), &price)
                .unwrap();
        let (_reader_dir, reader_dispatcher, _) = dispatcher();
        grant_material_roles(&reader_dispatcher);
        let actor = material_actor(732, 4).authorization;
        project_catalog_entry(&reader_dispatcher, &entry, &actor);
        let browse = ListSalesCatalog {
            term: "مرتبه".into(),
            category: None,
            after: None,
            limit: 30,
        };
        let QueryResult::SalesCatalog(page) = reader_dispatcher
            .dispatch_query(material_actor(733, 4), Query::SalesCatalog(browse.clone()))
            .await
            .unwrap()
        else {
            panic!("catalog")
        };
        assert_eq!(page.items, vec![entry]);
        assert!(!page.server_available);
        let result = reader_dispatcher
            .dispatch_query(
                material_actor(734, 4),
                Query::SalesConfiguration(CheckSalesConfiguration {
                    selection: PriceSelection {
                        target: target.clone(),
                        price_revision: price.revision,
                        color_id: None,
                        handle_id: None,
                        quantity: 2,
                    },
                    dimensions: None,
                }),
            )
            .await
            .unwrap();
        let QueryResult::SalesConfiguration(ref checked) = result else {
            panic!("configuration")
        };
        assert_eq!(checked.price.total_yer, 140_000);
        assert_public_quotation_evaluation(&reader_dispatcher, &target, price.revision).await;
        let encoded = serde_json::to_string(&result).unwrap();
        for field in ["cost", "margin", "parts", "notes", "55000"] {
            assert!(!encoded.contains(field), "{field}");
        }
        assert_eq!(
            reader_dispatcher
                .dispatch_query(material_actor(735, 8), Query::SalesCatalog(browse))
                .await
                .unwrap_err()
                .code
                .as_str(),
            "eitmad.error.authorization-denied.v1"
        );
    }

    async fn assert_public_quotation_evaluation(
        reader_dispatcher: &ProductDispatcher,
        target: &eitmad_contracts::pricing::PriceTarget,
        price_revision: u64,
    ) {
        use eitmad_contracts::{pricing::PriceSelection, sales_catalog::CheckSalesConfiguration};
        // The same receptionist evaluates a complete quotation from public-only catalog data.
        let mut branch = material_actor(736, 4);
        branch.authorization.scope = branch_authorization().scope;
        authorize_customer_branch(reader_dispatcher, &branch.authorization);
        let CommandResult::CustomerCreated(customer) = reader_dispatcher
            .dispatch_command(
                branch.clone(),
                Command::CreateCustomer(CreateCustomer {
                    name: CustomerName::parse("عميل تجريبي").unwrap(),
                    phone: CustomerPhone::parse("777123456").unwrap(),
                    address: None,
                    notes: None,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("customer");
        };
        let QueryResult::QuotationEvaluation(evaluation) = reader_dispatcher
            .dispatch_query(
                branch,
                Query::QuotationEvaluation(eitmad_contracts::quotation::EvaluateQuotation {
                    customer: Some(eitmad_contracts::quotation::QuotationCustomerIntent {
                        id: customer.customer.id,
                        revision: customer.customer.revision,
                    }),
                    lines: vec![eitmad_contracts::quotation::QuotationLineIntent {
                        id: Uuid::new_v4(),
                        configuration: CheckSalesConfiguration {
                            selection: PriceSelection {
                                target: target.clone(),
                                price_revision,
                                color_id: None,
                                handle_id: None,
                                quantity: 2,
                            },
                            dimensions: None,
                        },
                    }],
                    discount_basis_points: 501,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("evaluation");
        };
        assert!(evaluation.errors.is_empty());
        assert_eq!(
            evaluation.totals.unwrap(),
            eitmad_contracts::pricing::DiscountTotal {
                subtotal_yer: 140_000,
                discount_yer: 7014,
                total_yer: 132_986,
                approval_required: true,
            }
        );
        assert_eq!(
            last_audit_outcome(reader_dispatcher, "eitmad.quotation.evaluate.v1"),
            AuditOutcome::Succeeded
        );
    }

    #[tokio::test]
    async fn pricing_dispatch_returns_public_receipts_and_denies_receptionist_internal_routes() {
        use eitmad_contracts::pricing::{ListPrices, PublishPrice, ReviewPrice};
        let (_directory, dispatcher, broker) = dispatcher();
        grant_material_roles(&dispatcher);
        let dispatcher = dispatcher.with_price_confirmation(Arc::new(TestPriceServer));
        let target = pricing_fixture(&dispatcher).await;
        let (_, mut events) = broker
            .subscribe(
                authorization().scope,
                Subscription::Prices(eitmad_contracts::pricing::PriceChanges {}),
                None,
            )
            .unwrap();
        let input = PublishPrice {
            target: target.clone(),
            expected_revision: None,
            selling_price_yer: 70000,
            confirm_below_cost: false,
        };
        let result = dispatcher
            .dispatch_command(material_actor(713, 3), Command::PublishPrice(input.clone()))
            .await
            .unwrap();
        assert!(matches!(result, CommandResult::PricePublished(_)));
        let event = serde_json::to_string(&events.recv().await.unwrap().event).unwrap();
        assert!(!event.contains("cost") && !event.contains("margin"));
        let page = dispatcher
            .dispatch_query(
                material_actor(714, 4),
                Query::Prices(ListPrices {
                    term: String::new(),
                    after: None,
                    limit: 100,
                }),
            )
            .await
            .unwrap();
        let QueryResult::Prices(ref public) = page else {
            panic!("page")
        };
        assert_eq!(public.items.len(), 1);
        let payload = serde_json::to_string(&page).unwrap();
        for forbidden in ["costYer", "marginYer", "purchaseCostYer", "55000", "notes"] {
            assert!(!payload.contains(forbidden), "leaked {forbidden}");
        }
        let denied = dispatcher
            .dispatch_query(
                material_actor(715, 4),
                Query::PriceReview(ReviewPrice {
                    target,
                    selling_price_yer: 70000,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(denied.code.as_str(), "eitmad.error.authorization-denied.v1");
        let denied = dispatcher
            .dispatch_command(material_actor(716, 4), Command::PublishPrice(input.clone()))
            .await
            .unwrap_err();
        assert_eq!(denied.code.as_str(), "eitmad.error.authorization-denied.v1");
        let stale = dispatcher
            .dispatch_command(material_actor(717, 3), Command::PublishPrice(input))
            .await
            .unwrap_err();
        assert_eq!(
            stale.code.as_str(),
            "eitmad.error.pricing-revision-conflict.v1"
        );
    }
}
