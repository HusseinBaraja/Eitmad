//! Customer contact contracts.

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    identity::ScopeRef,
    sync::ChangeId,
    transport::{PageSizeError, UnixMillis},
};

pub const MAX_CUSTOMER_NAME_BYTES: usize = 256;
pub const MAX_CUSTOMER_PHONE_BYTES: usize = 64;
pub const MAX_CUSTOMER_ADDRESS_BYTES: usize = 512;
pub const MAX_CUSTOMER_NOTES_BYTES: usize = 2_048;
pub const MAX_CUSTOMER_SEARCH_BYTES: usize = 256;
pub const MAX_CUSTOMER_PAGE_SIZE: u32 = 100;

uuid_id!(CustomerId);

macro_rules! required_customer_text {
    ($name:ident, $error:ident, $maximum:expr, $message:literal) => {
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Validates bounded customer text without rewriting it.
            ///
            /// # Errors
            ///
            /// Returns an error for empty, padded, oversized, control, or
            /// bidirectional-formatting input.
            pub fn parse(value: impl Into<String>) -> Result<Self, $error> {
                let value = value.into();
                valid_required_text(&value, $maximum)
                    .then_some(Self(value))
                    .ok_or($error)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $error;

        impl std::fmt::Display for $error {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str($message)
            }
        }

        impl std::error::Error for $error {}
    };
}

required_customer_text!(
    CustomerName,
    CustomerNameError,
    MAX_CUSTOMER_NAME_BYTES,
    "customer name is invalid"
);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct CustomerPhone(String);

impl CustomerPhone {
    /// Validates a phone display value without rewriting it.
    ///
    /// # Errors
    ///
    /// Returns an error unless the value contains at least one accepted digit,
    /// uses only accepted display separators, and has at most one leading `+`.
    pub fn parse(value: impl Into<String>) -> Result<Self, CustomerPhoneError> {
        let value = value.into();
        let mut digits = 0_u32;
        let valid = valid_required_text(&value, MAX_CUSTOMER_PHONE_BYTES)
            && value
                .chars()
                .enumerate()
                .all(|(index, character)| match character {
                    '0'..='9' | '\u{0660}'..='\u{0669}' | '\u{06f0}'..='\u{06f9}' => {
                        digits += 1;
                        true
                    }
                    '+' => index == 0,
                    ' ' | '-' | '(' | ')' => true,
                    _ => false,
                })
            && digits > 0;
        valid.then_some(Self(value)).ok_or(CustomerPhoneError)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CustomerPhone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomerPhoneError;

impl std::fmt::Display for CustomerPhoneError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("customer phone is invalid")
    }
}

impl std::error::Error for CustomerPhoneError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct CustomerAddress(String);

impl CustomerAddress {
    /// Validates optional address text without rewriting it.
    ///
    /// # Errors
    ///
    /// Returns an error for empty, padded, oversized, or unsafe input.
    pub fn parse(value: impl Into<String>) -> Result<Self, CustomerAddressError> {
        let value = value.into();
        valid_optional_field(&value, MAX_CUSTOMER_ADDRESS_BYTES)
            .then_some(Self(value))
            .ok_or(CustomerAddressError)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CustomerAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomerAddressError;

impl std::fmt::Display for CustomerAddressError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("customer address is invalid")
    }
}

impl std::error::Error for CustomerAddressError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct CustomerNotes(String);

impl CustomerNotes {
    /// Validates optional notes without rewriting them.
    ///
    /// # Errors
    ///
    /// Returns an error for empty, padded, oversized, or unsafe input.
    pub fn parse(value: impl Into<String>) -> Result<Self, CustomerNotesError> {
        let value = value.into();
        valid_optional_field(&value, MAX_CUSTOMER_NOTES_BYTES)
            .then_some(Self(value))
            .ok_or(CustomerNotesError)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CustomerNotes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomerNotesError;

impl std::fmt::Display for CustomerNotesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("customer notes are invalid")
    }
}

impl std::error::Error for CustomerNotesError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct CustomerSearchTerm(String);

impl CustomerSearchTerm {
    /// Creates a bounded search term. Empty text requests the first scoped page.
    ///
    /// # Errors
    ///
    /// Returns an error for oversized or unsafe input.
    pub fn parse(value: impl Into<String>) -> Result<Self, CustomerSearchTermError> {
        let value = value.into();
        (value.len() <= MAX_CUSTOMER_SEARCH_BYTES
            && value.trim() == value
            && !value.chars().any(is_unsafe_character))
        .then_some(Self(value))
        .ok_or(CustomerSearchTermError)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CustomerSearchTerm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomerSearchTermError;

impl std::fmt::Display for CustomerSearchTermError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("customer search term is invalid")
    }
}

impl std::error::Error for CustomerSearchTermError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CustomerStatus {
    Active,
    Archived,
    Merged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CustomerSyncState {
    Pending,
    Confirmed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub id: CustomerId,
    pub scope: ScopeRef,
    pub name: CustomerName,
    pub phone: CustomerPhone,
    pub address: Option<CustomerAddress>,
    pub notes: Option<CustomerNotes>,
    pub status: CustomerStatus,
    pub revision: u64,
    pub updated_at: UnixMillis,
    pub sync_state: CustomerSyncState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomerMutationResult {
    pub customer: Customer,
    pub potential_duplicate_ids: Vec<CustomerId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomerPage {
    pub items: Vec<Customer>,
    pub next: Option<CustomerId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomerChangeNotice {
    pub customer_id: CustomerId,
    pub scope: ScopeRef,
    pub revision: u64,
    pub changed_at: UnixMillis,
    pub change_id: ChangeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetCustomer {
    pub customer_id: CustomerId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchCustomers {
    pub term: CustomerSearchTerm,
    pub after: Option<CustomerId>,
    #[schemars(range(min = 1, max = 100))]
    limit: u32,
}

impl SearchCustomers {
    /// Creates a bounded customer search query.
    ///
    /// # Errors
    ///
    /// Returns [`PageSizeError`] for a zero or oversized page.
    pub fn new(
        term: CustomerSearchTerm,
        after: Option<CustomerId>,
        limit: u32,
    ) -> Result<Self, PageSizeError> {
        if (1..=MAX_CUSTOMER_PAGE_SIZE).contains(&limit) {
            Ok(Self { term, after, limit })
        } else {
            Err(PageSizeError { limit })
        }
    }

    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.limit
    }
}

impl<'de> Deserialize<'de> for SearchCustomers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawQuery {
            term: CustomerSearchTerm,
            after: Option<CustomerId>,
            limit: u32,
        }

        let query = RawQuery::deserialize(deserializer)?;
        Self::new(query.term, query.after, query.limit).map_err(serde::de::Error::custom)
    }
}

fn valid_required_text(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.trim() == value
        && !value.chars().any(is_unsafe_character)
}

fn valid_optional_field(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.trim() == value
        && value.chars().all(|character| {
            (!character.is_control() || matches!(character, '\n' | '\r' | '\t'))
                && !matches!(
                    character,
                    '\u{061c}'
                        | '\u{200e}'..='\u{200f}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2066}'..='\u{2069}'
                )
        })
}

fn is_unsafe_character(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}' | '\u{200e}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entered_arabic_customer_text_is_preserved() {
        let name = CustomerName::parse("أحمد A. القيسي").unwrap();
        let phone = CustomerPhone::parse("+٩٦٧ (٧٧٧) ١٢٣-٤٥٦").unwrap();
        let address = CustomerAddress::parse("عدن، المنصورة").unwrap();
        assert_eq!(name.as_str(), "أحمد A. القيسي");
        assert_eq!(phone.as_str(), "+٩٦٧ (٧٧٧) ١٢٣-٤٥٦");
        assert_eq!(address.as_str(), "عدن، المنصورة");
    }

    #[test]
    fn customer_pages_are_bounded_during_deserialization() {
        assert!(
            serde_json::from_str::<SearchCustomers>(r#"{"term":"احمد","after":null,"limit":100}"#)
                .is_ok()
        );
        assert!(
            serde_json::from_str::<SearchCustomers>(r#"{"term":"احمد","after":null,"limit":101}"#)
                .is_err()
        );
    }
}
