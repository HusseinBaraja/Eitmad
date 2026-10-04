//! Server-confirmed price publication through stored credentials and pinned TLS.
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
                15,
            ),
        }
    }
}
impl PriceConfirmation for DirectPriceClient {
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
