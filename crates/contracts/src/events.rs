use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    authorization::AuthorizationPolicyChangeNotice,
    config::ConfigSnapshot,
    customer::CustomerChangeNotice,
    furniture::{FurnitureChangeNotice, FurnitureChanges},
    material::MaterialChangeNotice,
    part::{PartChangeNotice, PartChanges},
    permissions::EffectivePermissions,
    product::{ProductChangeNotice, ProductChanges},
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ConfigurationChanges {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PermissionChanges {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AuthorizationPolicyChanges {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CustomerChanges {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MaterialChanges {}

tagged_contract! {
    /// Resumable streams requested by clients.
    pub enum Subscription {
        Prices(crate::pricing::PriceChanges) => "eitmad.pricing.changed.subscribe.v1",
        Configuration(ConfigurationChanges) => "eitmad.config.changed.subscribe.v1",
        Permissions(PermissionChanges) => "eitmad.permissions.changed.subscribe.v1",
        AuthorizationPolicy(AuthorizationPolicyChanges) => "eitmad.authorization.policy.changed.subscribe.v1",
        QuotationDrafts(crate::quotation_draft::QuotationDraftChanges) => "eitmad.quotation-draft.changed.subscribe.v1",
        Customers(CustomerChanges) => "eitmad.customer.changed.subscribe.v1",
        Materials(MaterialChanges) => "eitmad.material.changed.subscribe.v1",
        Furnitures(FurnitureChanges) => "eitmad.furniture.changed.subscribe.v1",
        Products(ProductChanges) => "eitmad.product.changed.subscribe.v1",
        Parts(PartChanges) => "eitmad.part.changed.subscribe.v1"
    }
}

tagged_contract! {
    /// Ordered values emitted by subscriptions.
    pub enum Event {
        PriceChanged(crate::pricing::PriceChangeNotice) => "eitmad.pricing.changed.event.v1",
        ConfigurationChanged(ConfigSnapshot) => "eitmad.config.changed.event.v1",
        PermissionsChanged(EffectivePermissions) => "eitmad.permissions.changed.event.v1",
        AuthorizationPolicyChanged(AuthorizationPolicyChangeNotice) => "eitmad.authorization.policy.changed.event.v1",
        QuotationDraftChanged(crate::quotation_draft::QuotationDraftChangeNotice) => "eitmad.quotation-draft.changed.event.v1",
        CustomerChanged(CustomerChangeNotice) => "eitmad.customer.changed.event.v1",
        MaterialChanged(MaterialChangeNotice) => "eitmad.material.changed.event.v1",
        FurnitureChanged(FurnitureChangeNotice) => "eitmad.furniture.changed.event.v1",
        ProductChanged(ProductChangeNotice) => "eitmad.product.changed.event.v1",
        PartChanged(PartChangeNotice) => "eitmad.part.changed.event.v1"
    }
}

impl Event {
    #[must_use]
    pub const fn is_coalescible(&self) -> bool {
        matches!(
            self,
            Self::ConfigurationChanged(_)
                | Self::PermissionsChanged(_)
                | Self::AuthorizationPolicyChanged(_)
        )
    }

    /// Maps a typed event to its subscription route, including Furniture changes.
    #[must_use]
    pub const fn subscription_kind(&self) -> &'static str {
        match self {
            Self::PriceChanged(_) => "eitmad.pricing.changed.subscribe.v1",
            Self::ConfigurationChanged(_) => "eitmad.config.changed.subscribe.v1",
            Self::PermissionsChanged(_) => "eitmad.permissions.changed.subscribe.v1",
            Self::AuthorizationPolicyChanged(_) => {
                "eitmad.authorization.policy.changed.subscribe.v1"
            }
            Self::QuotationDraftChanged(_) => "eitmad.quotation-draft.changed.subscribe.v1",
            Self::CustomerChanged(_) => "eitmad.customer.changed.subscribe.v1",
            Self::MaterialChanged(_) => "eitmad.material.changed.subscribe.v1",
            Self::FurnitureChanged(_) => "eitmad.furniture.changed.subscribe.v1",
            Self::ProductChanged(_) => "eitmad.product.changed.subscribe.v1",
            Self::PartChanged(_) => "eitmad.part.changed.subscribe.v1",
        }
    }
}

impl Subscription {
    #[must_use]
    pub const fn is_coalescible(&self) -> bool {
        matches!(
            self,
            Self::Configuration(_) | Self::Permissions(_) | Self::AuthorizationPolicy(_)
        )
    }
}
