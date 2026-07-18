//! Rust-owned configuration registry, validation, persistence, and interchange.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use eitmad_authorization::{
    AuthorizationError, AuthorizationService, CONFIG_EXPORT_PERMISSION, CONFIG_IMPORT_PERMISSION,
    CONFIG_READ_PERMISSION, CONFIG_WRITE_PERMISSION, MutationContext,
};
use eitmad_contracts::{
    commands::UpdateConfiguration,
    config::{
        ConfigChange, ConfigEntry, ConfigKey, ConfigReadValue, ConfigSensitivity, ConfigSnapshot,
        ConfigWriteValue, RestartRequirement,
    },
    identity::{AuthorizationContext, ScopeRef},
};
use eitmad_observability_audit::{AuditOutcome, MutationAuditRecord};
use eitmad_storage::{AuthorityStore, ConfigurationCommitOutcome, DurableIdempotency};
use language_tags::LanguageTag;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

pub const CONFIGURATION_SCHEMA_VERSION: u32 = 1;
pub const CONFIGURATION_EXPORT_FORMAT_VERSION: u32 = 1;
pub const MAX_PATCH_CHANGES: usize = 64;
pub const MAX_IMPORT_BYTES: usize = 64 * 1024;
pub const PRIMARY_LOCALE_KEY: &str = "eitmad.config.locale.primary.v1";
pub const PRIMARY_LOCALE_DEFAULT: &str = "ar-YE";

const ORGANIZATION_SCOPE: &str = "organization";
const UPDATE_OPERATION: &str = "eitmad.config.update.v1";
const IMPORT_OPERATION: &str = "eitmad.config.import.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigurationError {
    Denied,
    UnsupportedScope,
    EmptyPatch,
    TooManyChanges,
    DuplicateKey,
    UnknownKey,
    WrongValueKind,
    InvalidValue,
    NonCanonicalValue,
    RevisionConflict {
        expected_revision: u64,
        actual_revision: u64,
    },
    IdempotencyMismatch,
    ImportTooLarge,
    ImportMalformed,
    FutureFormatVersion,
    FutureSchemaVersion,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigurationMutation {
    pub snapshot: ConfigSnapshot,
    pub changed: bool,
    pub replayed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigurationExportV1 {
    pub format_version: u32,
    pub configuration_schema_version: u32,
    pub source_revision: u64,
    pub entries: Vec<ConfigChange>,
    pub redacted_keys: Vec<ConfigKey>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Validator {
    CanonicalLanguageTag,
}

#[derive(Clone, Debug)]
struct SettingDefinition {
    key: ConfigKey,
    default: ConfigWriteValue,
    sensitivity: ConfigSensitivity,
    restart_requirement: RestartRequirement,
    validator: Option<Validator>,
}

#[derive(Clone, Debug)]
pub struct ConfigurationService {
    store: AuthorityStore,
    authorization: AuthorizationService,
    definitions: Arc<[SettingDefinition]>,
}

impl ConfigurationService {
    #[must_use]
    pub fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self::with_definitions(store, authorization, vec![primary_locale_definition()])
    }

    fn with_definitions(
        store: AuthorityStore,
        authorization: AuthorizationService,
        mut definitions: Vec<SettingDefinition>,
    ) -> Self {
        definitions.sort_by(|left, right| left.key.as_str().cmp(right.key.as_str()));
        Self {
            store,
            authorization,
            definitions: definitions.into(),
        }
    }

    /// Returns the authorized, stable, redacted configuration snapshot.
    ///
    /// # Errors
    ///
    /// Returns a structured authorization, scope, schema, or availability error.
    pub fn snapshot(
        &self,
        context: &AuthorizationContext,
    ) -> Result<ConfigSnapshot, ConfigurationError> {
        validate_scope(&context.scope)?;
        self.authorize(context, CONFIG_READ_PERMISSION)?;
        self.read_snapshot(&context.scope)
    }

    /// Validates and atomically applies a typed optimistic patch.
    ///
    /// # Errors
    ///
    /// Returns a structured authorization, validation, conflict, idempotency,
    /// schema, or availability error.
    pub fn update(
        &self,
        context: &MutationContext,
        command: &UpdateConfiguration,
    ) -> Result<ConfigurationMutation, ConfigurationError> {
        self.apply(context, command, CONFIG_WRITE_PERMISSION, UPDATE_OPERATION)
    }

    /// Produces deterministic JSON without scope identifiers or protected values.
    ///
    /// # Errors
    ///
    /// Returns a structured authorization, scope, schema, or availability error.
    pub fn export(&self, context: &AuthorizationContext) -> Result<Vec<u8>, ConfigurationError> {
        validate_scope(&context.scope)?;
        self.authorize(context, CONFIG_EXPORT_PERMISSION)?;
        let snapshot = self.read_snapshot(&context.scope)?;
        let mut entries = Vec::new();
        let mut redacted_keys = Vec::new();
        for entry in snapshot.entries {
            match read_to_write(entry.value) {
                Some(value) => entries.push(ConfigChange {
                    key: entry.key,
                    value,
                }),
                None => redacted_keys.push(entry.key),
            }
        }
        serde_json::to_vec(&ConfigurationExportV1 {
            format_version: CONFIGURATION_EXPORT_FORMAT_VERSION,
            configuration_schema_version: CONFIGURATION_SCHEMA_VERSION,
            source_revision: snapshot.revision,
            entries,
            redacted_keys,
        })
        .map_err(|_| ConfigurationError::Unavailable)
    }

    /// Merges public entries from deterministic JSON into the authorized target scope.
    ///
    /// # Errors
    ///
    /// Returns a structured import, authorization, validation, conflict,
    /// idempotency, schema, or availability error.
    pub fn import(
        &self,
        context: &MutationContext,
        expected_revision: u64,
        document: &[u8],
    ) -> Result<ConfigurationMutation, ConfigurationError> {
        if let Err(error) = self.authorize(&context.authorization, CONFIG_IMPORT_PERMISSION) {
            if error == ConfigurationError::Denied {
                self.audit_failure(
                    context,
                    IMPORT_OPERATION,
                    AuditOutcome::Denied,
                    "eitmad.error.authorization-denied.v1",
                    Vec::new(),
                )?;
            }
            return Err(error);
        }
        if let Err(error) = validate_scope(&context.authorization.scope) {
            self.audit_failure(
                context,
                IMPORT_OPERATION,
                AuditOutcome::Invalid,
                "eitmad.error.contract-invalid.v1",
                Vec::new(),
            )?;
            return Err(error);
        }
        if document.len() > MAX_IMPORT_BYTES {
            self.audit_invalid_import(context)?;
            return Err(ConfigurationError::ImportTooLarge);
        }
        let export: ConfigurationExportV1 = if let Ok(export) = serde_json::from_slice(document) {
            export
        } else {
            self.audit_invalid_import(context)?;
            return Err(ConfigurationError::ImportMalformed);
        };
        if export.format_version > CONFIGURATION_EXPORT_FORMAT_VERSION {
            self.audit_invalid_import(context)?;
            return Err(ConfigurationError::FutureFormatVersion);
        }
        if export.configuration_schema_version > CONFIGURATION_SCHEMA_VERSION {
            self.audit_invalid_import(context)?;
            return Err(ConfigurationError::FutureSchemaVersion);
        }
        if export.format_version != CONFIGURATION_EXPORT_FORMAT_VERSION
            || export.configuration_schema_version == 0
        {
            self.audit_invalid_import(context)?;
            return Err(ConfigurationError::ImportMalformed);
        }
        self.apply(
            context,
            &UpdateConfiguration {
                expected_revision,
                changes: export.entries,
            },
            CONFIG_IMPORT_PERMISSION,
            IMPORT_OPERATION,
        )
    }

    fn apply(
        &self,
        context: &MutationContext,
        command: &UpdateConfiguration,
        permission: &str,
        operation: &str,
    ) -> Result<ConfigurationMutation, ConfigurationError> {
        self.validate_mutation_request(context, command, permission, operation)?;
        let current = self.read_unredacted(&context.authorization.scope)?;
        let effective_changes: Vec<_> = command
            .changes
            .iter()
            .filter(|change| current.values.get(change.key.as_str()) != Some(&change.value))
            .cloned()
            .collect();
        let resulting_revision = if effective_changes.is_empty() {
            command.expected_revision
        } else {
            command
                .expected_revision
                .checked_add(1)
                .ok_or(ConfigurationError::Unavailable)?
        };
        let mut resulting_values = current.values;
        for change in &effective_changes {
            resulting_values.insert(change.key.as_str().to_owned(), change.value.clone());
        }
        let expected_snapshot = self.build_snapshot(
            context.authorization.scope.clone(),
            resulting_revision,
            &resulting_values,
        )?;
        let response_json =
            serde_json::to_vec(&expected_snapshot).map_err(|_| ConfigurationError::Unavailable)?;
        let idempotency = DurableIdempotency {
            key: context.idempotency_key,
            request_hash: request_hash(operation, command)?,
            response_json,
        };
        let changed_identifiers = effective_changes
            .iter()
            .map(|change| change.key.as_str().to_owned())
            .collect();
        let audit = audit_record(context, operation, changed_identifiers);
        self.commit_patch(
            context,
            PatchCommit {
                command,
                operation,
                effective_changes: &effective_changes,
                expected_snapshot: &expected_snapshot,
                idempotency: &idempotency,
                audit: &audit,
            },
        )
    }

    fn validate_mutation_request(
        &self,
        context: &MutationContext,
        command: &UpdateConfiguration,
        permission: &str,
        operation: &str,
    ) -> Result<(), ConfigurationError> {
        if let Err(error) = self.authorize(&context.authorization, permission) {
            if error == ConfigurationError::Denied {
                self.audit_failure(
                    context,
                    operation,
                    AuditOutcome::Denied,
                    "eitmad.error.authorization-denied.v1",
                    Vec::new(),
                )?;
            }
            return Err(error);
        }
        if let Err(error) = validate_scope(&context.authorization.scope) {
            self.audit_failure(
                context,
                operation,
                AuditOutcome::Invalid,
                "eitmad.error.contract-invalid.v1",
                Vec::new(),
            )?;
            return Err(error);
        }
        if let Err(error) = self.validate_patch(&command.changes) {
            self.audit_failure(
                context,
                operation,
                AuditOutcome::Invalid,
                "eitmad.error.config-value-invalid.v1",
                command
                    .changes
                    .iter()
                    .map(|change| change.key.as_str().to_owned())
                    .collect(),
            )?;
            return Err(error);
        }
        Ok(())
    }

    fn commit_patch(
        &self,
        context: &MutationContext,
        commit: PatchCommit<'_>,
    ) -> Result<ConfigurationMutation, ConfigurationError> {
        match self.store.commit_configuration(
            &context.authorization.scope,
            commit.command.expected_revision,
            CONFIGURATION_SCHEMA_VERSION,
            commit.effective_changes,
            !commit.effective_changes.is_empty(),
            commit.operation,
            commit.idempotency,
            commit.audit,
        ) {
            Ok(ConfigurationCommitOutcome::Committed { revision }) => {
                debug_assert_eq!(revision, commit.expected_snapshot.revision);
                Ok(ConfigurationMutation {
                    snapshot: commit.expected_snapshot.clone(),
                    changed: !commit.effective_changes.is_empty(),
                    replayed: false,
                })
            }
            Ok(ConfigurationCommitOutcome::Replayed { response_json }) => {
                let snapshot = serde_json::from_slice(&response_json)
                    .map_err(|_| ConfigurationError::Unavailable)?;
                Ok(ConfigurationMutation {
                    snapshot,
                    changed: false,
                    replayed: true,
                })
            }
            Ok(ConfigurationCommitOutcome::RevisionConflict { actual_revision }) => {
                Err(ConfigurationError::RevisionConflict {
                    expected_revision: commit.command.expected_revision,
                    actual_revision,
                })
            }
            Ok(ConfigurationCommitOutcome::IdempotencyMismatch) => {
                Err(ConfigurationError::IdempotencyMismatch)
            }
            Err(_) => Err(ConfigurationError::Unavailable),
        }
    }

    fn authorize(
        &self,
        context: &AuthorizationContext,
        permission: &str,
    ) -> Result<(), ConfigurationError> {
        self.authorization
            .authorize(context, permission)
            .map_err(|error| match error {
                AuthorizationError::Denied => ConfigurationError::Denied,
                _ => ConfigurationError::Unavailable,
            })
    }

    fn read_snapshot(&self, scope: &ScopeRef) -> Result<ConfigSnapshot, ConfigurationError> {
        validate_scope(scope)?;
        let current = self.read_unredacted(scope)?;
        self.build_snapshot(scope.clone(), current.revision, &current.values)
    }

    fn read_unredacted(
        &self,
        scope: &ScopeRef,
    ) -> Result<CurrentConfiguration, ConfigurationError> {
        let stored = self
            .store
            .read_configuration(scope)
            .map_err(|_| ConfigurationError::Unavailable)?;
        if stored.schema_version > CONFIGURATION_SCHEMA_VERSION {
            return Err(ConfigurationError::FutureSchemaVersion);
        }
        let mut values = self
            .definitions
            .iter()
            .map(|definition| {
                (
                    definition.key.as_str().to_owned(),
                    definition.default.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for change in stored.values {
            self.validate_change(&change)?;
            values.insert(change.key.as_str().to_owned(), change.value);
        }
        Ok(CurrentConfiguration {
            revision: stored.revision,
            values,
        })
    }

    fn build_snapshot(
        &self,
        scope: ScopeRef,
        revision: u64,
        values: &BTreeMap<String, ConfigWriteValue>,
    ) -> Result<ConfigSnapshot, ConfigurationError> {
        let entries = self
            .definitions
            .iter()
            .map(|definition| {
                let value = values
                    .get(definition.key.as_str())
                    .ok_or(ConfigurationError::Unavailable)?;
                Ok(ConfigEntry {
                    key: definition.key.clone(),
                    value: redact(value, definition.sensitivity),
                    sensitivity: definition.sensitivity,
                    restart_requirement: definition.restart_requirement,
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(ConfigSnapshot {
            schema_version: CONFIGURATION_SCHEMA_VERSION,
            revision,
            scope,
            entries,
        })
    }

    fn validate_patch(&self, changes: &[ConfigChange]) -> Result<(), ConfigurationError> {
        if changes.is_empty() {
            return Err(ConfigurationError::EmptyPatch);
        }
        if changes.len() > MAX_PATCH_CHANGES {
            return Err(ConfigurationError::TooManyChanges);
        }
        let mut seen = BTreeSet::new();
        for change in changes {
            if !seen.insert(change.key.as_str()) {
                return Err(ConfigurationError::DuplicateKey);
            }
            self.validate_change(change)?;
        }
        Ok(())
    }

    fn validate_change(&self, change: &ConfigChange) -> Result<(), ConfigurationError> {
        let definition = self
            .definitions
            .iter()
            .find(|candidate| candidate.key == change.key)
            .ok_or(ConfigurationError::UnknownKey)?;
        let kind_matches = same_value_kind(&definition.default, &change.value);
        if !kind_matches {
            return Err(ConfigurationError::WrongValueKind);
        }
        if definition.sensitivity == ConfigSensitivity::Secret
            && !matches!(change.value, ConfigWriteValue::SecretReference(_))
        {
            return Err(ConfigurationError::WrongValueKind);
        }
        match (definition.validator, &change.value) {
            (Some(Validator::CanonicalLanguageTag), ConfigWriteValue::Text(value)) => {
                validate_language_tag(value)
            }
            _ => Ok(()),
        }
    }

    fn audit_failure(
        &self,
        context: &MutationContext,
        operation: &str,
        outcome: AuditOutcome,
        error_code: &str,
        changed_identifiers: Vec<String>,
    ) -> Result<(), ConfigurationError> {
        let record = audit_record(context, operation, changed_identifiers)
            .with_outcome(outcome, Some(error_code.to_owned()));
        self.store
            .append_audit(&record)
            .map_err(|_| ConfigurationError::Unavailable)
    }

    fn audit_invalid_import(&self, context: &MutationContext) -> Result<(), ConfigurationError> {
        self.audit_failure(
            context,
            IMPORT_OPERATION,
            AuditOutcome::Invalid,
            "eitmad.error.config-import-invalid.v1",
            Vec::new(),
        )
    }
}

#[derive(Clone, Debug)]
struct CurrentConfiguration {
    revision: u64,
    values: BTreeMap<String, ConfigWriteValue>,
}

#[derive(Clone, Copy)]
struct PatchCommit<'a> {
    command: &'a UpdateConfiguration,
    operation: &'a str,
    effective_changes: &'a [ConfigChange],
    expected_snapshot: &'a ConfigSnapshot,
    idempotency: &'a DurableIdempotency,
    audit: &'a MutationAuditRecord,
}

fn validate_scope(scope: &ScopeRef) -> Result<(), ConfigurationError> {
    (scope.kind.as_str() == ORGANIZATION_SCOPE)
        .then_some(())
        .ok_or(ConfigurationError::UnsupportedScope)
}

fn primary_locale_definition() -> SettingDefinition {
    SettingDefinition {
        key: ConfigKey::parse(PRIMARY_LOCALE_KEY).expect("static configuration key is valid"),
        default: ConfigWriteValue::Text(PRIMARY_LOCALE_DEFAULT.to_owned()),
        sensitivity: ConfigSensitivity::Public,
        restart_requirement: RestartRequirement::Application,
        validator: Some(Validator::CanonicalLanguageTag),
    }
}

fn validate_language_tag(value: &str) -> Result<(), ConfigurationError> {
    let parsed = LanguageTag::parse(value).map_err(|_| ConfigurationError::InvalidValue)?;
    parsed
        .validate()
        .map_err(|_| ConfigurationError::InvalidValue)?;
    let canonical = parsed
        .canonicalize()
        .map_err(|_| ConfigurationError::InvalidValue)?;
    (canonical.as_str() == value)
        .then_some(())
        .ok_or(ConfigurationError::NonCanonicalValue)
}

fn redact(value: &ConfigWriteValue, sensitivity: ConfigSensitivity) -> ConfigReadValue {
    if sensitivity != ConfigSensitivity::Public {
        return ConfigReadValue::Redacted;
    }
    match value {
        ConfigWriteValue::Boolean(value) => ConfigReadValue::Boolean(*value),
        ConfigWriteValue::Integer(value) => ConfigReadValue::Integer(*value),
        ConfigWriteValue::Decimal(value) => ConfigReadValue::Decimal(value.clone()),
        ConfigWriteValue::Text(value) => ConfigReadValue::Text(value.clone()),
        ConfigWriteValue::TextList(value) => ConfigReadValue::TextList(value.clone()),
        ConfigWriteValue::SecretReference(value) => ConfigReadValue::SecretReference(*value),
    }
}

fn same_value_kind(left: &ConfigWriteValue, right: &ConfigWriteValue) -> bool {
    matches!(
        (left, right),
        (ConfigWriteValue::Boolean(_), ConfigWriteValue::Boolean(_))
            | (ConfigWriteValue::Integer(_), ConfigWriteValue::Integer(_))
            | (ConfigWriteValue::Decimal(_), ConfigWriteValue::Decimal(_))
            | (ConfigWriteValue::Text(_), ConfigWriteValue::Text(_))
            | (ConfigWriteValue::TextList(_), ConfigWriteValue::TextList(_))
            | (
                ConfigWriteValue::SecretReference(_),
                ConfigWriteValue::SecretReference(_)
            )
    )
}

fn read_to_write(value: ConfigReadValue) -> Option<ConfigWriteValue> {
    match value {
        ConfigReadValue::Boolean(value) => Some(ConfigWriteValue::Boolean(value)),
        ConfigReadValue::Integer(value) => Some(ConfigWriteValue::Integer(value)),
        ConfigReadValue::Decimal(value) => Some(ConfigWriteValue::Decimal(value)),
        ConfigReadValue::Text(value) => Some(ConfigWriteValue::Text(value)),
        ConfigReadValue::TextList(value) => Some(ConfigWriteValue::TextList(value)),
        ConfigReadValue::SecretReference(value) => Some(ConfigWriteValue::SecretReference(value)),
        ConfigReadValue::Redacted => None,
    }
}

fn request_hash(
    operation: &str,
    command: &UpdateConfiguration,
) -> Result<[u8; 32], ConfigurationError> {
    let canonical =
        serde_json::to_vec(&(operation, command)).map_err(|_| ConfigurationError::Unavailable)?;
    Ok(Sha256::digest(canonical).into())
}

fn audit_record(
    context: &MutationContext,
    operation: &str,
    changed_identifiers: Vec<String>,
) -> MutationAuditRecord {
    MutationAuditRecord {
        audit_id: Uuid::new_v4(),
        occurred_at: context.occurred_at,
        principal_id: context.authorization.identity.principal_id,
        principal_kind: context.authorization.identity.principal_kind,
        scope: context.authorization.scope.clone(),
        correlation_id: context.correlation_id,
        causation_id: context.causation_id,
        idempotency_key: Some(context.idempotency_key),
        operation: operation.to_owned(),
        outcome: AuditOutcome::Succeeded,
        previous_revision: None,
        resulting_revision: None,
        changed_identifiers,
        error_code: None,
    }
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::{
        authorization::RelationshipSubject,
        config::SecretReferenceId,
        identity::{
            AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId,
        },
        transport::{CorrelationId, IdempotencyKey, UnixMillis},
    };
    use tempfile::TempDir;

    use super::*;

    struct Fixture {
        _directory: TempDir,
        store: AuthorityStore,
        service: ConfigurationService,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = TempDir::new().unwrap();
            let store = AuthorityStore::open(directory.path()).unwrap();
            let authorization = AuthorizationService::new(store.clone());
            let service = ConfigurationService::new(store.clone(), authorization.clone());
            let context = mutation(1, 10, 1);
            authorization
                .bootstrap_owner(
                    &context,
                    &RelationshipSubject {
                        principal_id: context.authorization.identity.principal_id,
                        principal_kind: PrincipalKind::User,
                    },
                )
                .unwrap();
            Self {
                _directory: directory,
                store,
                service,
            }
        }
    }

    fn authorization(principal: u128, scope: u128) -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::from_u128(principal + 1_000)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::from_u128(principal)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            scope: ScopeRef {
                kind: ScopeKind::parse(ORGANIZATION_SCOPE).unwrap(),
                id: ScopeId::new(Uuid::from_u128(scope)),
            },
        }
    }

    fn mutation(principal: u128, scope: u128, idempotency: u128) -> MutationContext {
        MutationContext {
            authorization: authorization(principal, scope),
            correlation_id: CorrelationId::new(Uuid::from_u128(idempotency + 2_000)),
            causation_id: None,
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(idempotency)),
            occurred_at: UnixMillis(100),
        }
    }

    fn locale(value: &str) -> ConfigChange {
        ConfigChange {
            key: ConfigKey::parse(PRIMARY_LOCALE_KEY).unwrap(),
            value: ConfigWriteValue::Text(value.to_owned()),
        }
    }

    #[test]
    fn returns_default_snapshot_in_stable_key_order() {
        let fixture = Fixture::new();
        let snapshot = fixture.service.snapshot(&authorization(1, 10)).unwrap();
        assert_eq!(snapshot.schema_version, 1);
        assert_eq!(snapshot.revision, 0);
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].key.as_str(), PRIMARY_LOCALE_KEY);
        assert_eq!(
            snapshot.entries[0].value,
            ConfigReadValue::Text(PRIMARY_LOCALE_DEFAULT.to_owned())
        );
        assert_eq!(
            snapshot.entries[0].restart_requirement,
            RestartRequirement::Application
        );
    }

    #[test]
    fn valid_patch_is_atomic_revisioned_and_idempotent() {
        let fixture = Fixture::new();
        let context = mutation(1, 10, 2);
        let command = UpdateConfiguration {
            expected_revision: 0,
            changes: vec![locale("en-US")],
        };
        let first = fixture.service.update(&context, &command).unwrap();
        assert!(first.changed);
        assert!(!first.replayed);
        assert_eq!(first.snapshot.revision, 1);
        assert_eq!(
            first.snapshot.entries[0].value,
            ConfigReadValue::Text("en-US".to_owned())
        );
        let replay = fixture.service.update(&context, &command).unwrap();
        assert!(!replay.changed);
        assert!(replay.replayed);
        assert_eq!(replay.snapshot, first.snapshot);
        assert_eq!(
            fixture
                .service
                .snapshot(&context.authorization)
                .unwrap()
                .revision,
            1
        );
    }

    #[test]
    fn rejects_invalid_patches_without_partial_persistence() {
        let fixture = Fixture::new();
        let cases = [
            (
                UpdateConfiguration {
                    expected_revision: 0,
                    changes: Vec::new(),
                },
                ConfigurationError::EmptyPatch,
            ),
            (
                UpdateConfiguration {
                    expected_revision: 0,
                    changes: vec![locale("en-US"), locale("ar-YE")],
                },
                ConfigurationError::DuplicateKey,
            ),
            (
                UpdateConfiguration {
                    expected_revision: 0,
                    changes: vec![ConfigChange {
                        key: ConfigKey::parse("eitmad.config.unknown.v1").unwrap(),
                        value: ConfigWriteValue::Text("value".to_owned()),
                    }],
                },
                ConfigurationError::UnknownKey,
            ),
            (
                UpdateConfiguration {
                    expected_revision: 0,
                    changes: vec![ConfigChange {
                        key: ConfigKey::parse(PRIMARY_LOCALE_KEY).unwrap(),
                        value: ConfigWriteValue::Boolean(true),
                    }],
                },
                ConfigurationError::WrongValueKind,
            ),
            (
                UpdateConfiguration {
                    expected_revision: 0,
                    changes: vec![locale("ar_YE")],
                },
                ConfigurationError::InvalidValue,
            ),
            (
                UpdateConfiguration {
                    expected_revision: 0,
                    changes: vec![locale("en-us")],
                },
                ConfigurationError::NonCanonicalValue,
            ),
        ];
        for (index, (command, expected)) in cases.into_iter().enumerate() {
            assert_eq!(
                fixture
                    .service
                    .update(&mutation(1, 10, 10 + index as u128), &command),
                Err(expected)
            );
        }
        let stored = fixture
            .store
            .read_configuration(&authorization(1, 10).scope)
            .unwrap();
        assert_eq!(stored.revision, 0);
        assert!(stored.values.is_empty());
    }

    #[test]
    fn no_op_succeeds_without_revision_change_and_conflicts_are_reported() {
        let fixture = Fixture::new();
        let no_op = fixture
            .service
            .update(
                &mutation(1, 10, 30),
                &UpdateConfiguration {
                    expected_revision: 0,
                    changes: vec![locale(PRIMARY_LOCALE_DEFAULT)],
                },
            )
            .unwrap();
        assert!(!no_op.changed);
        assert_eq!(no_op.snapshot.revision, 0);
        assert_eq!(
            fixture.service.update(
                &mutation(1, 10, 31),
                &UpdateConfiguration {
                    expected_revision: 7,
                    changes: vec![locale("en-US")],
                }
            ),
            Err(ConfigurationError::RevisionConflict {
                expected_revision: 7,
                actual_revision: 0
            })
        );
    }

    #[test]
    fn redacts_synthetic_sensitive_and_secret_settings() {
        let fixture = Fixture::new();
        let definitions = vec![
            SettingDefinition {
                key: ConfigKey::parse("eitmad.config.synthetic.public.v1").unwrap(),
                default: ConfigWriteValue::Text("اسم Model".to_owned()),
                sensitivity: ConfigSensitivity::Public,
                restart_requirement: RestartRequirement::None,
                validator: None,
            },
            SettingDefinition {
                key: ConfigKey::parse("eitmad.config.synthetic.sensitive.v1").unwrap(),
                default: ConfigWriteValue::Text("sensitive plaintext".to_owned()),
                sensitivity: ConfigSensitivity::Sensitive,
                restart_requirement: RestartRequirement::None,
                validator: None,
            },
            SettingDefinition {
                key: ConfigKey::parse("eitmad.config.synthetic.secret.v1").unwrap(),
                default: ConfigWriteValue::SecretReference(SecretReferenceId::new(
                    Uuid::from_u128(55),
                )),
                sensitivity: ConfigSensitivity::Secret,
                restart_requirement: RestartRequirement::Engine,
                validator: None,
            },
        ];
        let store = fixture.store.clone();
        let service = ConfigurationService::with_definitions(
            store.clone(),
            AuthorizationService::new(store),
            definitions,
        );
        let context = authorization(1, 10);
        let snapshot = service.snapshot(&context).unwrap();
        assert_eq!(
            snapshot.entries[0].value,
            ConfigReadValue::Text("اسم Model".to_owned())
        );
        assert_eq!(snapshot.entries[1].value, ConfigReadValue::Redacted);
        assert_eq!(snapshot.entries[2].value, ConfigReadValue::Redacted);
        let encoded = serde_json::to_string(&snapshot).unwrap();
        assert!(encoded.contains("اسم Model"));
        assert!(!encoded.contains("sensitive plaintext"));
        assert!(!encoded.contains(&Uuid::from_u128(55).to_string()));
        let exported: ConfigurationExportV1 =
            serde_json::from_slice(&service.export(&context).unwrap()).unwrap();
        assert_eq!(exported.entries.len(), 1);
        assert_eq!(
            exported.entries[0].value,
            ConfigWriteValue::Text("اسم Model".to_owned())
        );
        assert_eq!(exported.redacted_keys.len(), 2);
    }

    #[test]
    fn export_import_round_trip_is_deterministic_and_scope_free() {
        let fixture = Fixture::new();
        fixture
            .service
            .update(
                &mutation(1, 10, 40),
                &UpdateConfiguration {
                    expected_revision: 0,
                    changes: vec![locale("ar-Latn-YE")],
                },
            )
            .unwrap();
        let exported = fixture.service.export(&authorization(1, 10)).unwrap();
        assert_eq!(
            exported,
            fixture.service.export(&authorization(1, 10)).unwrap()
        );
        let text = String::from_utf8(exported.clone()).unwrap();
        assert!(!text.contains(&Uuid::from_u128(10).to_string()));

        let target_bootstrap = mutation(1, 11, 41);
        AuthorizationService::new(fixture.store.clone())
            .bootstrap_owner(
                &target_bootstrap,
                &RelationshipSubject {
                    principal_id: target_bootstrap.authorization.identity.principal_id,
                    principal_kind: PrincipalKind::User,
                },
            )
            .unwrap();
        let imported = fixture
            .service
            .import(&mutation(1, 11, 42), 0, &exported)
            .unwrap();
        assert!(imported.changed);
        assert_eq!(imported.snapshot.revision, 1);
        assert_eq!(
            imported.snapshot.entries[0].value,
            ConfigReadValue::Text("ar-Latn-YE".to_owned())
        );
    }

    #[test]
    fn import_rejects_malformed_future_and_oversized_documents() {
        let fixture = Fixture::new();
        assert_eq!(
            fixture.service.import(&mutation(1, 10, 50), 0, b"{"),
            Err(ConfigurationError::ImportMalformed)
        );
        let future = br#"{"formatVersion":2,"configurationSchemaVersion":1,"sourceRevision":0,"entries":[],"redactedKeys":[]}"#;
        assert_eq!(
            fixture.service.import(&mutation(1, 10, 51), 0, future),
            Err(ConfigurationError::FutureFormatVersion)
        );
        let future_schema = br#"{"formatVersion":1,"configurationSchemaVersion":2,"sourceRevision":0,"entries":[],"redactedKeys":[]}"#;
        assert_eq!(
            fixture
                .service
                .import(&mutation(1, 10, 53), 0, future_schema),
            Err(ConfigurationError::FutureSchemaVersion)
        );
        assert_eq!(
            fixture
                .service
                .import(&mutation(1, 10, 52), 0, &vec![b' '; MAX_IMPORT_BYTES + 1]),
            Err(ConfigurationError::ImportTooLarge)
        );
    }

    #[test]
    fn rejects_unsupported_scope_and_unauthorized_access() {
        let fixture = Fixture::new();
        let mut unsupported = authorization(1, 10);
        unsupported.scope.kind = ScopeKind::parse("site").unwrap();
        assert_eq!(
            fixture.service.snapshot(&unsupported),
            Err(ConfigurationError::UnsupportedScope)
        );
        assert_eq!(
            fixture.service.snapshot(&authorization(2, 10)),
            Err(ConfigurationError::Denied)
        );
    }

    #[test]
    fn rejects_patches_over_the_change_limit() {
        let fixture = Fixture::new();
        assert_eq!(
            fixture.service.update(
                &mutation(1, 10, 60),
                &UpdateConfiguration {
                    expected_revision: 0,
                    changes: vec![locale("en-US"); MAX_PATCH_CHANGES + 1],
                }
            ),
            Err(ConfigurationError::TooManyChanges)
        );
    }
}
