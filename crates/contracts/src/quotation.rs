//! Read-only quotation intent and public evaluation; no issuance or approval grant.
use crate::{
    customer::CustomerId,
    furniture::FurnitureDimensions,
    identity::ScopeRef,
    pricing::{DiscountTotal, SellingPrice},
    sales_catalog::CheckSalesConfiguration,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluateQuotation {
    pub customer: Option<QuotationCustomerIntent>,
    pub lines: Vec<QuotationLineIntent>,
    pub discount_basis_points: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotationCustomerIntent {
    pub id: CustomerId,
    pub revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotationLineIntent {
    pub id: uuid::Uuid,
    pub configuration: CheckSalesConfiguration,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationCustomerSnapshot {
    pub id: CustomerId,
    pub revision: u64,
    pub name: String,
    pub phone: String,
    pub address: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvaluatedQuotationLine {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub variant_name: String,
    pub color_id: Option<uuid::Uuid>,
    pub color_name: Option<String>,
    pub handle_id: Option<uuid::Uuid>,
    pub handle_name: Option<String>,
    pub dimensions: Option<FurnitureDimensions>,
    pub quantity: u32,
    pub price: SellingPrice,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QuotationField {
    Customer,
    CustomerRevision,
    Lines,
    LineId,
    Target,
    PriceRevision,
    Quantity,
    Dimensions,
    ColorId,
    HandleId,
    DiscountBasisPoints,
    Total,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QuotationIssue {
    Required,
    Invalid,
    Duplicate,
    Stale,
    Unavailable,
    Overflow,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationFieldError {
    pub line_id: Option<uuid::Uuid>,
    pub field: QuotationField,
    pub issue: QuotationIssue,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationEvaluation {
    pub scope: ScopeRef,
    pub customer: Option<QuotationCustomerSnapshot>,
    pub lines: Vec<EvaluatedQuotationLine>,
    pub currency: String,
    pub discount_basis_points: u32,
    /// Present only when every field and checked calculation is valid.
    pub totals: Option<DiscountTotal>,
    pub errors: Vec<QuotationFieldError>,
    /// Cache evaluation is never evidence that issuance can succeed online.
    pub server_available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        pricing::{PriceSelection, PriceTarget},
        product::{ProductId, ProductReference, ProductVariantId},
        queries::Query,
    };

    fn request() -> serde_json::Value {
        serde_json::to_value(Query::QuotationEvaluation(EvaluateQuotation {
            customer: Some(QuotationCustomerIntent {
                id: CustomerId::new(uuid::Uuid::from_u128(1)),
                revision: 1,
            }),
            lines: vec![QuotationLineIntent {
                id: uuid::Uuid::from_u128(2),
                configuration: CheckSalesConfiguration {
                    selection: PriceSelection {
                        target: PriceTarget::Product(ProductReference {
                            scope: ScopeRef {
                                kind: crate::identity::ScopeKind::parse("organization").unwrap(),
                                id: crate::identity::ScopeId::new(uuid::Uuid::from_u128(3)),
                            },
                            product_id: ProductId::new(uuid::Uuid::from_u128(4)),
                            variant_id: ProductVariantId::new(uuid::Uuid::from_u128(5)),
                            revision: 1,
                            schema_version: 1,
                        }),
                        price_revision: 1,
                        quantity: 1,
                        color_id: None,
                        handle_id: None,
                    },
                    dimensions: None,
                },
            }],
            discount_basis_points: 500,
        }))
        .unwrap()
    }

    #[test]
    fn quotation_intent_round_trips_and_rejects_client_authority_at_each_input_boundary() {
        let valid = request();
        let parsed: Query = serde_json::from_value(valid.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), valid);
        for (pointer, key, value) in [
            ("/payload", "subtotalYer", serde_json::json!(1)),
            ("/payload", "totalYer", serde_json::json!(1)),
            ("/payload", "approvalRequired", serde_json::json!(false)),
            ("/payload/customer", "name", serde_json::json!("اسم مزور")),
            ("/payload/lines/0", "price", serde_json::json!(1)),
            (
                "/payload/lines/0/configuration",
                "totalYer",
                serde_json::json!(1),
            ),
            (
                "/payload/lines/0/configuration/selection",
                "unitPriceYer",
                serde_json::json!(1),
            ),
        ] {
            let mut forged = valid.clone();
            forged
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(key.into(), value);
            assert!(
                serde_json::from_value::<Query>(forged).is_err(),
                "{pointer}/{key}"
            );
        }
    }

    #[test]
    fn quotation_wire_intent_rejects_fractional_negative_and_out_of_representation_numbers() {
        for path in [
            "/payload/discountBasisPoints",
            "/payload/lines/0/configuration/selection/quantity",
            "/payload/customer/revision",
            "/payload/lines/0/configuration/selection/priceRevision",
        ] {
            for number in [
                serde_json::json!(-1),
                serde_json::json!(1.5),
                serde_json::json!("1.5"),
            ] {
                let mut invalid = request();
                *invalid.pointer_mut(path).unwrap() = number;
                assert!(serde_json::from_value::<Query>(invalid).is_err(), "{path}");
            }
        }
    }
}
