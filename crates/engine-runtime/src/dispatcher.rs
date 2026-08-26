//! Engine composition dispatcher for Rust-owned product verticals.

use std::sync::Arc;

use async_trait::async_trait;
use eitmad_authorization::{
    AUTHORIZATION_MANAGE_PERMISSION, AccessAuditContext, AuthorizationError, AuthorizationService,
    CONFIG_READ_PERMISSION, MutationContext, PERMISSIONS_READ_PERMISSION, now,
};
use eitmad_configuration::{ConfigurationError, ConfigurationService};
use eitmad_contracts::{
    commands::{Command, CommandResult},
    errors::{ContractError, ErrorCode, ErrorDetail, MessageId, RetryDisposition},
    events::{Event, Subscription},
    queries::{Query, QueryResult},
};
use eitmad_observability_audit::AuditOutcome;
use eitmad_reference_marker::{
    REFERENCE_MARKER_READ_PERMISSION, ReferenceMarkerError, ReferenceMarkerService,
};
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
    reference_markers: ReferenceMarkerService,
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
    pub fn new(
        store: AuthorityStore,
        events: EventBroker,
        development_ephemeral_owner: bool,
    ) -> Self {
        Self::with_event_publisher(store, Arc::new(events), development_ephemeral_owner)
    }

    fn with_event_publisher(
        store: AuthorityStore,
        events: Arc<dyn ProductEventPublisher>,
        development_ephemeral_owner: bool,
    ) -> Self {
        let authorization = AuthorizationService::new(store.clone())
            .with_development_ephemeral_owner(development_ephemeral_owner);
        let configuration = ConfigurationService::new(store.clone(), authorization.clone());
        let reference_markers = ReferenceMarkerService::new(store.clone(), authorization.clone());
        Self {
            store,
            authorization,
            configuration,
            reference_markers,
            events,
        }
    }

    #[must_use]
    pub const fn authorization(&self) -> &AuthorizationService {
        &self.authorization
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
    async fn dispatch_command(
        &self,
        context: DispatchContext,
        command: Command,
    ) -> Result<CommandResult, ContractError> {
        let operation = command.kind();
        let mutation = Self::mutation_context(&context).map_err(|error| *error)?;
        match command {
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
                require_protocol_1_2(&context).map_err(|error| *error)?;
                let result = self
                    .authorization
                    .grant_relationship(&mutation, &command)
                    .map_err(|error| authorization_error(error, &context))?;
                self.publish_pending(&context, mutation.idempotency_key)
                    .map_err(|()| authorization_error(AuthorizationError::Unavailable, &context))?;
                Ok(CommandResult::RelationshipGranted(result))
            }
            Command::RevokeScopeRelationship(command) => {
                require_protocol_1_2(&context).map_err(|error| *error)?;
                let result = self
                    .authorization
                    .revoke_relationship(&mutation, &command)
                    .map_err(|error| authorization_error(error, &context))?;
                self.publish_pending(&context, mutation.idempotency_key)
                    .map_err(|()| authorization_error(AuthorizationError::Unavailable, &context))?;
                Ok(CommandResult::RelationshipRevoked(result))
            }
            Command::UpsertReferenceMarker(command) => {
                let outcome = self
                    .reference_markers
                    .upsert(&mutation, &command)
                    .map_err(|error| reference_marker_error(error, &context))?;
                self.publish_pending(&context, mutation.idempotency_key)
                    .map_err(|()| {
                        reference_marker_error(ReferenceMarkerError::Unavailable, &context)
                    })?;
                Ok(CommandResult::ReferenceMarkerUpserted(outcome.marker))
            }
            Command::CancelOperation(_) | Command::ReportInstallerOutcome(_) => self
                .authorization
                .audit_access_result(
                    &AccessAuditContext {
                        authorization: context.authorization.clone(),
                        correlation_id: context.correlation_id,
                        causation_id: context.causation_id,
                        occurred_at: now(),
                    },
                    operation,
                    "command-scope",
                    AuditOutcome::Invalid,
                    Some("eitmad.error.contract-invalid.v1"),
                    Vec::new(),
                )
                .map_err(|error| authorization_error(error, &context))
                .and(Err(unsupported(&context))),
        }
    }
}

#[async_trait]
impl QueryDispatcher for ProductDispatcher {
    async fn dispatch_query(
        &self,
        context: DispatchContext,
        query: Query,
    ) -> Result<QueryResult, ContractError> {
        let operation = query.kind();
        let result = match query {
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
            Query::ScopeRelationships(query) => {
                require_protocol_1_2(&context).map_err(|error| *error)?;
                self.authorization
                    .list_relationships(&context.authorization, &query)
                    .map(QueryResult::ScopeRelationships)
                    .map_err(|error| authorization_error(error, &context))
            }
            Query::ReferenceMarkers(query) => self
                .reference_markers
                .list(&context.authorization, &query)
                .map(QueryResult::ReferenceMarkers)
                .map_err(|error| reference_marker_error(error, &context)),
            Query::UpdateState(_) | Query::SyncStatus(_) => Err(unsupported(&context)),
        };
        let (outcome, error_code) = match &result {
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
            .map_err(|error| authorization_error(error, &context))?;
        result
    }

    async fn authorize_subscription(
        &self,
        context: SubscriptionContext,
        subscription: &Subscription,
    ) -> Result<(), ContractError> {
        let permission = match subscription {
            Subscription::Configuration(_) => CONFIG_READ_PERMISSION,
            Subscription::Permissions(_) => PERMISSIONS_READ_PERMISSION,
            Subscription::ReferenceMarkers(_) => REFERENCE_MARKER_READ_PERMISSION,
            Subscription::AuthorizationPolicy(_) if context.protocol_version.minor >= 2 => {
                AUTHORIZATION_MANAGE_PERMISSION
            }
            Subscription::AuthorizationPolicy(_)
            | Subscription::UpdateState(_)
            | Subscription::SyncStatus(_)
            | Subscription::RecordChanges(_)
            | Subscription::BackgroundJobs(_)
            | Subscription::Notifications(_)
            | Subscription::Errors(_) => {
                return Err(contract_error(
                    "eitmad.error.ipc-subscription-unsupported.v1",
                    "eitmad.message.ipc-subscription-unsupported.v1",
                    context.correlation_id,
                    RetryDisposition::Never,
                    None,
                ));
            }
        };
        self.authorization
            .authorize(&context.authorization, permission)
            .map_err(|error| authorization_contract_error(error, context.correlation_id, None))
    }
}

fn reference_marker_error(
    error_value: ReferenceMarkerError,
    context: &DispatchContext,
) -> ContractError {
    match error_value {
        ReferenceMarkerError::Denied => contract_error(
            "eitmad.error.authorization-denied.v1",
            "eitmad.message.authorization-denied.v1",
            context.correlation_id,
            RetryDisposition::Never,
            None,
        ),
        ReferenceMarkerError::RevisionConflict {
            expected_revision,
            actual_revision,
        } => contract_error(
            "eitmad.error.reference-marker-revision-conflict.v1",
            "eitmad.message.reference-marker-revision-conflict.v1",
            context.correlation_id,
            RetryDisposition::SafeImmediately,
            Some(ErrorDetail::RevisionConflict {
                expected: expected_revision.unwrap_or(0),
                actual: actual_revision.unwrap_or(0),
            }),
        ),
        ReferenceMarkerError::Unavailable => contract_error(
            "eitmad.error.reference-marker-unavailable.v1",
            "eitmad.message.reference-marker-unavailable.v1",
            context.correlation_id,
            RetryDisposition::SafeAfterDelay(1_000),
            None,
        ),
        ReferenceMarkerError::UnsupportedScope | ReferenceMarkerError::IdempotencyMismatch => {
            unsupported(context)
        }
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

fn require_protocol_1_2(context: &DispatchContext) -> Result<(), Box<ContractError>> {
    (context.protocol_version.minor >= 2)
        .then_some(())
        .ok_or_else(|| Box::new(unsupported(context)))
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use eitmad_contracts::{
        authorization::{RelationId, RelationshipSubject},
        commands::{
            CancelOperation, GrantScopeRelationship, UpdateConfiguration, UpsertReferenceMarker,
        },
        config::{ConfigChange, ConfigKey, ConfigWriteValue},
        events::{
            AuthorizationPolicyChanges, ConfigurationChanges, ReferenceMarkerChanges, Subscription,
        },
        identity::{
            AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind, ScopeId,
            ScopeKind, ScopeRef, SessionId, TenantId,
        },
        queries::{GetConfiguration, GetSyncStatus, Query},
        reference_marker::{ListReferenceMarkers, ReferenceMarkerId, ReferenceMarkerLabel},
        transport::{CorrelationId, IdempotencyKey, OperationId, PROTOCOL_VERSION, UnixMillis},
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

    fn dispatcher() -> (TempDir, ProductDispatcher, EventBroker) {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let broker = EventBroker::new();
        let dispatcher = ProductDispatcher::new(store, broker.clone(), false);
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

    #[tokio::test]
    async fn dispatcher_persists_invalid_command_and_query_outcomes() {
        let (_directory, dispatcher, _broker) = dispatcher();

        let invalid_command = dispatcher
            .dispatch_command(
                context(200),
                Command::CancelOperation(CancelOperation {
                    operation_id: OperationId::new(Uuid::from_u128(201)),
                }),
            )
            .await;
        assert!(invalid_command.is_err());
        assert_eq!(
            last_audit_outcome(&dispatcher, "eitmad.operation.cancel.v1"),
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
            .dispatch_query(context(205), Query::SyncStatus(GetSyncStatus {}))
            .await;
        assert!(failed.is_err());
        assert_eq!(
            last_audit_outcome(&dispatcher, "eitmad.sync.get-status.v1"),
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
            .dispatch_query(context(210), Query::SyncStatus(GetSyncStatus {}))
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
    async fn routes_reference_marker_command_paged_query_and_compact_event() {
        let (_directory, dispatcher, broker) = dispatcher();
        let (_, mut events) = broker
            .subscribe(
                authorization().scope,
                Subscription::ReferenceMarkers(ReferenceMarkerChanges {}),
                None,
            )
            .unwrap();
        let marker_id = ReferenceMarkerId::new(Uuid::from_u128(90));
        let result = dispatcher
            .dispatch_command(
                context(91),
                Command::UpsertReferenceMarker(UpsertReferenceMarker {
                    marker_id,
                    expected_revision: None,
                    label: ReferenceMarkerLabel::parse("مرجع REF-١٢").unwrap(),
                }),
            )
            .await
            .unwrap();
        let CommandResult::ReferenceMarkerUpserted(marker) = result else {
            panic!("reference marker result expected")
        };
        assert_eq!(marker.label.as_str(), "مرجع REF-١٢");
        let published = events.recv().await.unwrap();
        let Event::ReferenceMarkerChanged(notice) = published.event else {
            panic!("reference marker event expected")
        };
        assert_eq!(notice.marker_id, marker_id);
        assert_eq!(notice.revision, 1);

        let page = dispatcher
            .dispatch_query(
                context(92),
                Query::ReferenceMarkers(ListReferenceMarkers::new(None, 10).unwrap()),
            )
            .await
            .unwrap();
        let QueryResult::ReferenceMarkers(page) = page else {
            panic!("reference marker page expected")
        };
        assert_eq!(page.items, vec![marker]);
        assert_eq!(
            last_audit_outcome(&dispatcher, "eitmad.reference-marker.list.v1"),
            AuditOutcome::Succeeded
        );
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
    async fn development_dispatcher_treats_asserted_principal_as_ephemeral_owner() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let dispatcher = ProductDispatcher::new(store, EventBroker::new(), true);
        assert!(
            dispatcher
                .authorize_subscription(
                    SubscriptionContext {
                        authorization: authorization(),
                        correlation_id: CorrelationId::new(Uuid::from_u128(30)),
                        protocol_version: PROTOCOL_VERSION,
                    },
                    &Subscription::Configuration(ConfigurationChanges {})
                )
                .await
                .is_ok()
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
    async fn runtime_drain_replays_pending_publication_after_post_commit_failure() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let broker = EventBroker::new();
        let publisher = Arc::new(FailOncePublisher {
            broker: broker.clone(),
            fail_next: AtomicBool::new(true),
        });
        let dispatcher = ProductDispatcher::with_event_publisher(store.clone(), publisher, false);
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
}
