//! Engine composition dispatcher for Rust-owned product verticals.

use std::sync::Arc;

use async_trait::async_trait;
use eitmad_authorization::{
    AUTHORIZATION_MANAGE_PERMISSION, AuthorizationError, AuthorizationService,
    CONFIG_READ_PERMISSION, MutationContext, PERMISSIONS_READ_PERMISSION, now,
};
use eitmad_configuration::{ConfigurationError, ConfigurationService};
use eitmad_contracts::{
    commands::{Command, CommandResult},
    errors::{ContractError, ErrorCode, ErrorDetail, MessageId, RetryDisposition},
    events::{Event, Subscription},
    queries::{Query, QueryResult},
};
use eitmad_storage::AuthorityStore;

use crate::local_ipc::{
    CommandDispatcher, DispatchContext, EventBroker, QueryDispatcher, SubscriptionContext,
};

#[derive(Clone)]
pub struct ProductDispatcher {
    store: AuthorityStore,
    authorization: AuthorizationService,
    configuration: ConfigurationService,
    events: Arc<dyn ProductEventPublisher>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicationRecoveryError;

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
        Self {
            store,
            authorization,
            configuration,
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
        for publication in self
            .store
            .pending_publications()
            .map_err(|_| PublicationRecoveryError)?
        {
            self.events
                .publish(publication.scope.clone(), publication.event)
                .map_err(|()| PublicationRecoveryError)?;
            if publication.policy_changed {
                self.events.policy_changed(publication.scope.clone());
            }
            self.store
                .complete_publication(&publication.scope, publication.idempotency_key)
                .map_err(|_| PublicationRecoveryError)?;
        }
        Ok(())
    }
}

#[async_trait]
impl CommandDispatcher for ProductDispatcher {
    async fn dispatch_command(
        &self,
        context: DispatchContext,
        command: Command,
    ) -> Result<CommandResult, ContractError> {
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
            Command::CancelOperation(_) | Command::ReportInstallerOutcome(_) => {
                Err(unsupported(&context))
            }
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
        match query {
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
            Query::UpdateState(_) | Query::SyncStatus(_) => Err(unsupported(&context)),
        }
    }

    async fn authorize_subscription(
        &self,
        context: SubscriptionContext,
        subscription: &Subscription,
    ) -> Result<(), ContractError> {
        let permission = match subscription {
            Subscription::Configuration(_) => CONFIG_READ_PERMISSION,
            Subscription::Permissions(_) => PERMISSIONS_READ_PERMISSION,
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
        commands::{GrantScopeRelationship, UpdateConfiguration},
        config::{ConfigChange, ConfigKey, ConfigWriteValue},
        events::{AuthorizationPolicyChanges, ConfigurationChanges, Subscription},
        identity::{
            AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind, ScopeId,
            ScopeKind, ScopeRef, SessionId,
        },
        queries::{GetConfiguration, Query},
        transport::{CorrelationId, IdempotencyKey, PROTOCOL_VERSION, UnixMillis},
    };
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
