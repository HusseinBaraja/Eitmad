use crate::{
    DirectServerConfig,
    authenticated_http::{AuthenticatedHttpClient, HttpError},
};
use eitmad_contracts::{
    identity::AuthorizationContext,
    pricing::{ConfirmPrice, PriceTarget, PublishedPrice},
    secrets::SecretId,
    transport::UnixMillis,
};
use eitmad_pricing::{PriceConfirmation, PricingError};
use std::time::{Duration, Instant};
pub struct DirectPriceClient {
    http: AuthenticatedHttpClient,
}
/// Maps only engine-owned scopes to the registered remote organization, including nested references.
pub(crate) fn remap_catalog_scope(
    record: &mut eitmad_contracts::catalog_revision::CatalogRevision,
    local_scope: &eitmad_contracts::identity::ScopeRef,
    scope: &eitmad_contracts::identity::ScopeRef,
) -> Result<(), PricingError> {
    use eitmad_contracts::catalog_revision::CatalogRevision;
    if record.identity().3 != local_scope {
        return Err(PricingError::Denied);
    }
    match record {
        CatalogRevision::MaterialCategory(v) => v.scope = scope.clone(),
        CatalogRevision::PartCategory(v) => v.scope = scope.clone(),
        CatalogRevision::Unit(v) => v.scope = scope.clone(),
        CatalogRevision::Material(v) => v.scope = scope.clone(),
        CatalogRevision::ProductCategory(v) => v.scope = scope.clone(),
        CatalogRevision::FurnitureCategory(v) => v.scope = scope.clone(),
        CatalogRevision::Product(v) => v.scope = scope.clone(),
        CatalogRevision::Furniture(v) => {
            if v.parts.iter().any(|u| &u.reference.scope != local_scope) {
                return Err(PricingError::Denied);
            }
            v.scope = scope.clone();
            for usage in &mut v.parts {
                usage.reference.scope = scope.clone();
            }
        }
        CatalogRevision::Part(v) => {
            if &v.composition.scope != local_scope
                || v.cost.rows.iter().any(|r| {
                    &r.material.scope != local_scope
                        || &r.unit.scope != local_scope
                        || &r.cost_unit.scope != local_scope
                })
            {
                return Err(PricingError::Denied);
            }
            v.scope = scope.clone();
            v.composition.scope = scope.clone();
            for row in &mut v.cost.rows {
                row.material.scope = scope.clone();
                row.unit.scope = scope.clone();
                row.cost_unit.scope = scope.clone();
            }
        }
    }
    Ok(())
}
impl DirectPriceClient {
    #[must_use]
    pub fn from_config(
        config: DirectServerConfig,
        secrets: eitmad_secret_storage::SecretStore,
        credential: SecretId,
    ) -> Self {
        Self {
            http: AuthenticatedHttpClient::from_config(
                config,
                secrets,
                credential,
                "eitmad.capability.pricing.v1",
                17,
            ),
        }
    }
}
impl PriceConfirmation for DirectPriceClient {
    /// Transfers a catalog batch after mapping its local organization and nested references.
    /// # Errors
    /// Rejects foreign scopes, server denial, invalid catalog data, and expired or unavailable requests.
    fn synchronize_catalog(
        &self,
        actor: &AuthorizationContext,
        input: &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions,
        deadline: UnixMillis,
    ) -> Result<(), PricingError> {
        if input.scope != actor.scope {
            return Err(PricingError::Denied);
        }
        let mut request = input.clone();
        request.scope = self.http.remote_scope();
        for record in &mut request.records {
            remap_catalog_scope(record, &actor.scope, &request.scope)?;
        }
        self.http
            .request::<()>(
                actor,
                "/v1/catalog-revisions/synchronize",
                &request,
                request_budget(deadline)?,
            )
            .map_err(|e| match e {
                HttpError::Denied => PricingError::Denied,
                HttpError::Invalid => PricingError::Reference,
                _ => PricingError::Unconfirmed,
            })
    }
    fn read(
        &self,
        actor: &AuthorizationContext,
        input: &eitmad_contracts::pricing::ReadPublishedPrices,
        deadline: UnixMillis,
    ) -> Result<eitmad_contracts::pricing::PublishedPricePage, PricingError> {
        let mut request = input.clone();
        request.scope = self.http.remote_scope();
        let budget = request_budget(deadline)?;
        let mut page: eitmad_contracts::pricing::PublishedPricePage = self
            .http
            .request(actor, "/v1/pricing/read", &request, budget)
            .map_err(|e| match e {
                HttpError::Denied => PricingError::Denied,
                _ => PricingError::Unconfirmed,
            })?;
        for price in &mut page.items {
            if price.target.scope() != &request.scope {
                return Err(PricingError::Unconfirmed);
            }
            match &mut price.target {
                PriceTarget::Product(r) => r.scope = actor.scope.clone(),
                PriceTarget::Furniture(r) => r.scope = actor.scope.clone(),
            }
        }
        Ok(page)
    }
    fn status(
        &self,
        actor: &AuthorizationContext,
        input: &ConfirmPrice,
        deadline: UnixMillis,
    ) -> Result<Option<PublishedPrice>, PricingError> {
        let mut request = input.clone();
        match &mut request.command.target {
            PriceTarget::Product(r) => r.scope = self.http.remote_scope(),
            PriceTarget::Furniture(r) => r.scope = self.http.remote_scope(),
        }
        let budget = request_budget(deadline)?;
        let mut result: Option<PublishedPrice> = self
            .http
            .request(actor, "/v1/pricing/status", &request, budget)
            .map_err(|e| match e {
                HttpError::Denied => PricingError::Denied,
                HttpError::Invalid => PricingError::Invalid,
                _ => PricingError::Unconfirmed,
            })?;
        if let Some(price) = &mut result {
            if price.target != request.command.target {
                return Err(PricingError::Unconfirmed);
            }
            price.target = input.command.target.clone();
        }
        Ok(result)
    }
    fn confirm(
        &self,
        actor: &AuthorizationContext,
        input: &ConfirmPrice,
        deadline: UnixMillis,
    ) -> Result<PublishedPrice, PricingError> {
        eitmad_pricing::validate_publication(input)?;
        let budget = request_budget(deadline)?;
        let mut request = input.clone();
        let scope = self.http.remote_scope();
        match &mut request.command.target {
            PriceTarget::Product(r) => r.scope = scope,
            PriceTarget::Furniture(r) => r.scope = scope,
        }
        let mut result: PublishedPrice = self
            .http
            .request(actor, "/v1/pricing/publish", &request, budget)
            .map_err(|e| match e {
                HttpError::Denied => PricingError::Denied,
                HttpError::Invalid => PricingError::Invalid,
                HttpError::Conflict => PricingError::Conflict {
                    expected: input.command.expected_revision,
                    actual: None,
                },
                _ => PricingError::Unconfirmed,
            })?;
        if result.target != request.command.target {
            return Err(PricingError::Unconfirmed);
        }
        result.target = input.command.target.clone();
        Ok(result)
    }
}
fn request_budget(deadline: UnixMillis) -> Result<Instant, PricingError> {
    let remaining = deadline
        .0
        .checked_sub(now().0)
        .filter(|v| *v > 0)
        .ok_or(PricingError::Unconfirmed)?;
    let millis = u64::try_from(remaining)
        .map_err(|_| PricingError::Unconfirmed)?
        .min(10_000);
    Ok(Instant::now() + Duration::from_millis(millis))
}

fn now() -> UnixMillis {
    UnixMillis(
        i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        )
        .unwrap_or(i64::MAX),
    )
}
