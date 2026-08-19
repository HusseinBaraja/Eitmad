use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::SecretReferenceId;

open_id!(SecretKind, "secret kind");

/// Typed, non-secret reference to secret material owned by Rust.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretId {
    kind: SecretKind,
    reference: SecretReferenceId,
}

impl SecretId {
    #[must_use]
    pub const fn new(kind: SecretKind, reference: SecretReferenceId) -> Self {
        Self { kind, reference }
    }

    #[must_use]
    pub const fn kind(&self) -> &SecretKind {
        &self.kind
    }

    #[must_use]
    pub const fn reference(&self) -> SecretReferenceId {
        self.reference
    }

    #[must_use]
    pub fn canonical_key(&self) -> String {
        format!("{}:{}", self.kind, self.reference.value())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn secret_identifiers_are_typed_and_canonical() {
        let identifier = SecretId::new(
            SecretKind::parse("external-api-token").unwrap(),
            SecretReferenceId::new(Uuid::from_u128(7)),
        );

        assert_eq!(
            identifier.canonical_key(),
            "external-api-token:00000000-0000-0000-0000-000000000007"
        );
    }

    #[test]
    fn invalid_secret_kinds_are_rejected() {
        assert!(SecretKind::parse("External API token").is_err());
        assert!(SecretKind::parse("external..token").is_err());
        assert!(serde_json::from_str::<SecretKind>(r#""../../token""#).is_err());
    }
}
