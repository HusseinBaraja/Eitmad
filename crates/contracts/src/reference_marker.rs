//! Product-neutral reference marker contracts.

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    identity::ScopeRef,
    sync::ChangeId,
    transport::{PageSizeError, UnixMillis},
};

pub const MAX_REFERENCE_MARKER_LABEL_BYTES: usize = 256;
pub const MAX_REFERENCE_MARKER_PAGE_SIZE: u32 = 100;

uuid_id!(ReferenceMarkerId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct ReferenceMarkerLabel(String);

impl ReferenceMarkerLabel {
    /// Creates a non-empty, bounded marker label without changing Unicode text.
    ///
    /// # Errors
    ///
    /// Returns an error for surrounding whitespace, control or bidirectional
    /// formatting characters, or a UTF-8 representation larger than
    /// [`MAX_REFERENCE_MARKER_LABEL_BYTES`].
    pub fn parse(value: impl Into<String>) -> Result<Self, ReferenceMarkerLabelError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_REFERENCE_MARKER_LABEL_BYTES
            && value.trim() == value
            && !value
                .chars()
                .any(|character| character.is_control() || is_bidi_format_control(character));
        valid
            .then_some(Self(value))
            .ok_or(ReferenceMarkerLabelError)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_bidi_format_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}' | '\u{200e}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
    )
}

impl<'de> Deserialize<'de> for ReferenceMarkerLabel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceMarkerLabelError;

impl std::fmt::Display for ReferenceMarkerLabelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("reference marker label is invalid")
    }
}

impl std::error::Error for ReferenceMarkerLabelError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ReferenceMarkerSyncState {
    Pending,
    Confirmed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceMarker {
    pub id: ReferenceMarkerId,
    pub scope: ScopeRef,
    pub label: ReferenceMarkerLabel,
    pub revision: u64,
    pub updated_at: UnixMillis,
    pub sync_state: ReferenceMarkerSyncState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceMarkerPage {
    pub items: Vec<ReferenceMarker>,
    pub next: Option<ReferenceMarkerId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceMarkerChangeNotice {
    pub marker_id: ReferenceMarkerId,
    pub scope: ScopeRef,
    pub revision: u64,
    pub changed_at: UnixMillis,
    pub change_id: ChangeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListReferenceMarkers {
    pub after: Option<ReferenceMarkerId>,
    #[schemars(range(min = 1, max = 100))]
    limit: u32,
}

impl ListReferenceMarkers {
    /// Creates a bounded marker page query.
    ///
    /// # Errors
    ///
    /// Returns [`PageSizeError`] for a zero or oversized page.
    pub fn new(after: Option<ReferenceMarkerId>, limit: u32) -> Result<Self, PageSizeError> {
        if (1..=MAX_REFERENCE_MARKER_PAGE_SIZE).contains(&limit) {
            Ok(Self { after, limit })
        } else {
            Err(PageSizeError { limit })
        }
    }

    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.limit
    }
}

impl<'de> Deserialize<'de> for ListReferenceMarkers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawQuery {
            after: Option<ReferenceMarkerId>,
            limit: u32,
        }

        let raw = RawQuery::deserialize(deserializer)?;
        Self::new(raw.after, raw.limit).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_preserve_arabic_and_mixed_direction_text() {
        let label = ReferenceMarkerLabel::parse("مرجع REF-١٢").unwrap();
        assert_eq!(label.as_str(), "مرجع REF-١٢");
        assert!(ReferenceMarkerLabel::parse("مر\u{200d}جع").is_ok());
        assert!(ReferenceMarkerLabel::parse(" مرجع").is_err());
        assert!(ReferenceMarkerLabel::parse("x\n").is_err());
        assert!(ReferenceMarkerLabel::parse("REF-12\u{202e}txt").is_err());
        assert!(ReferenceMarkerLabel::parse("أ".repeat(129)).is_err());
    }

    #[test]
    fn pages_are_bounded_during_deserialization() {
        assert!(
            serde_json::from_str::<ListReferenceMarkers>(r#"{"after":null,"limit":100}"#).is_ok()
        );
        assert!(
            serde_json::from_str::<ListReferenceMarkers>(r#"{"after":null,"limit":0}"#).is_err()
        );
        assert!(
            serde_json::from_str::<ListReferenceMarkers>(r#"{"after":null,"limit":101}"#).is_err()
        );
    }
}
