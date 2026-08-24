//! Authenticated, tenant-isolated WAN relay coordination.
//!
//! The coordinator owns only connection metadata. Route hooks receive no
//! business payload and cannot change synchronization semantics.

use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use eitmad_contracts::{
    relay::{
        OpenRelaySession, RelayFailureId, RelayFailureReport, RelayHealth, RelayHealthState,
        RelayRoute, RelaySessionId, RelaySessionMetadata, RelaySessionState,
    },
    server::AuthenticatedServerSession,
    transport::{CorrelationId, UnixMillis},
};
use uuid::Uuid;

pub const DEFAULT_RELAY_SESSION_TTL_MS: u64 = 15 * 60 * 1_000;
pub const MAX_RELAY_SESSION_TTL_MS: u64 = 60 * 60 * 1_000;
pub const MAX_RECONNECT_ATTEMPTS: u32 = 8;
pub const MAX_FAILURE_REPORTS: usize = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayAction {
    Open,
    Heartbeat,
    Reconnect,
    Close,
    AdministrativeClose,
    ReportFailure,
    ReadHealth,
    ReadSession,
}

impl RelayAction {
    #[must_use]
    pub const fn operation(self) -> &'static str {
        match self {
            Self::Open => "eitmad.relay.session.open.v1",
            Self::Heartbeat => "eitmad.relay.session.heartbeat.v1",
            Self::Reconnect => "eitmad.relay.session.reconnect.v1",
            Self::Close => "eitmad.relay.session.close.v1",
            Self::AdministrativeClose => "eitmad.relay.session.admin-close.v1",
            Self::ReportFailure => "eitmad.relay.failure.report.v1",
            Self::ReadHealth => "eitmad.relay.health.read.v1",
            Self::ReadSession => "eitmad.relay.session.read.v1",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayAuditOutcome {
    Succeeded,
    Denied,
    Failed,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RelayError {
    #[error("relay action is denied")]
    Denied,
    #[error("relay request is invalid")]
    Invalid,
    #[error("relay session was not found")]
    NotFound,
    #[error("relay route is unavailable")]
    RouteUnavailable,
    #[error("relay retry is not due")]
    RetryNotDue,
    #[error("relay authority is unavailable")]
    Unavailable,
}

#[async_trait]
pub trait RelaySecurity: Send + Sync {
    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        action: RelayAction,
        tenant_id: eitmad_contracts::identity::TenantId,
        route: Option<&RelayRoute>,
    ) -> Result<(), RelayError>;

    async fn audit(
        &self,
        actor: &AuthenticatedServerSession,
        action: RelayAction,
        outcome: RelayAuditOutcome,
        correlation_id: CorrelationId,
        target: Option<RelaySessionId>,
        now: UnixMillis,
    ) -> Result<(), RelayError>;
}

#[async_trait]
pub trait RelayRouter: Send + Sync {
    async fn connect_peer(&self, session: &RelaySessionMetadata) -> Result<(), RelayError>;
    async fn connect_server(&self, session: &RelaySessionMetadata) -> Result<(), RelayError>;
    async fn disconnect(&self, session: &RelaySessionMetadata) -> Result<(), RelayError>;
}

#[derive(Default)]
struct RelayState {
    sessions: BTreeMap<RelaySessionId, RelaySessionMetadata>,
    failures: VecDeque<RelayFailureReport>,
    accepting_sessions: bool,
}

#[derive(Clone)]
pub struct RelayCoordinator {
    security: Arc<dyn RelaySecurity>,
    router: Arc<dyn RelayRouter>,
    state: Arc<RwLock<RelayState>>,
}

#[derive(Clone, Copy)]
struct RelayActionContext {
    correlation_id: CorrelationId,
    target: Option<RelaySessionId>,
    now: UnixMillis,
}

impl RelayCoordinator {
    #[must_use]
    pub fn new(security: Arc<dyn RelaySecurity>, router: Arc<dyn RelayRouter>) -> Self {
        Self {
            security,
            router,
            state: Arc::new(RwLock::new(RelayState {
                accepting_sessions: true,
                ..RelayState::default()
            })),
        }
    }

    /// Opens an authorized relay route and records only bounded metadata.
    ///
    /// # Errors
    ///
    /// Returns a stable denial, validation, route, or availability error.
    pub async fn open(
        &self,
        actor: &AuthenticatedServerSession,
        request: &OpenRelaySession,
        now: UnixMillis,
    ) -> Result<RelaySessionMetadata, RelayError> {
        if let Err(error) = self
            .authorize_actor(
                actor,
                RelayAction::Open,
                request.tenant_id,
                Some(&request.route),
            )
            .await
        {
            self.audit(
                actor,
                RelayAction::Open,
                outcome_for(error),
                request.correlation_id,
                None,
                now,
            )
            .await?;
            return Err(error);
        }
        if request.source_device_id != actor.device_id
            || request.requested_ttl_ms == 0
            || request.requested_ttl_ms > MAX_RELAY_SESSION_TTL_MS
        {
            self.audit(
                actor,
                RelayAction::Open,
                RelayAuditOutcome::Invalid,
                request.correlation_id,
                None,
                now,
            )
            .await?;
            return Err(RelayError::Invalid);
        }
        if !self
            .state
            .read()
            .map_err(|_| RelayError::Unavailable)?
            .accepting_sessions
        {
            self.audit(
                actor,
                RelayAction::Open,
                RelayAuditOutcome::Failed,
                request.correlation_id,
                None,
                now,
            )
            .await?;
            return Err(RelayError::RouteUnavailable);
        }
        let expires_at = session_expiry(now, request.requested_ttl_ms)?;
        let mut metadata = RelaySessionMetadata {
            relay_session_id: RelaySessionId::new(Uuid::new_v4()),
            tenant_id: request.tenant_id,
            source_device_id: request.source_device_id,
            route: request.route.clone(),
            state: RelaySessionState::Connecting,
            connected_at: now,
            last_seen_at: now,
            expires_at,
            reconnect_attempt: 0,
            next_reconnect_at: None,
        };
        if let Err(error) = self.connect_route(&metadata).await {
            self.audit(
                actor,
                RelayAction::Open,
                RelayAuditOutcome::Failed,
                request.correlation_id,
                Some(metadata.relay_session_id),
                now,
            )
            .await?;
            return Err(error);
        }
        metadata.state = RelaySessionState::Active;
        if let Err(error) = self
            .audit(
                actor,
                RelayAction::Open,
                RelayAuditOutcome::Succeeded,
                request.correlation_id,
                Some(metadata.relay_session_id),
                now,
            )
            .await
        {
            let _ = self.router.disconnect(&metadata).await;
            return Err(error);
        }
        self.state
            .write()
            .map_err(|_| RelayError::Unavailable)?
            .sessions
            .insert(metadata.relay_session_id, metadata.clone());
        Ok(metadata)
    }

    /// Records an authorized session heartbeat.
    ///
    /// # Errors
    ///
    /// Returns a denial, invalid-state, missing-session, or audit error.
    pub async fn heartbeat(
        &self,
        actor: &AuthenticatedServerSession,
        session_id: RelaySessionId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<RelaySessionMetadata, RelayError> {
        let mut session = self
            .owned_session_for_action(
                actor,
                RelayAction::Heartbeat,
                session_id,
                correlation_id,
                now,
            )
            .await?;
        self.authorize_for_action(
            actor,
            RelayAction::Heartbeat,
            session.tenant_id,
            Some(&session.route),
            RelayActionContext {
                correlation_id,
                target: Some(session_id),
                now,
            },
        )
        .await?;
        if session.state != RelaySessionState::Active || now.0 > session.expires_at.0 {
            self.audit(
                actor,
                RelayAction::Heartbeat,
                RelayAuditOutcome::Invalid,
                correlation_id,
                Some(session_id),
                now,
            )
            .await?;
            return Err(RelayError::Invalid);
        }
        session.last_seen_at = now;
        self.audit(
            actor,
            RelayAction::Heartbeat,
            RelayAuditOutcome::Succeeded,
            correlation_id,
            Some(session_id),
            now,
        )
        .await?;
        self.replace_session(session.clone())?;
        Ok(session)
    }

    /// Moves a failed session to bounded exponential reconnect scheduling.
    ///
    /// # Errors
    ///
    /// Returns a denial, expired-session, retry-limit, or audit error.
    pub async fn schedule_reconnect(
        &self,
        actor: &AuthenticatedServerSession,
        session_id: RelaySessionId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<RelaySessionMetadata, RelayError> {
        let mut session = self
            .owned_session_for_action(
                actor,
                RelayAction::Reconnect,
                session_id,
                correlation_id,
                now,
            )
            .await?;
        self.authorize_for_action(
            actor,
            RelayAction::Reconnect,
            session.tenant_id,
            Some(&session.route),
            RelayActionContext {
                correlation_id,
                target: Some(session_id),
                now,
            },
        )
        .await?;
        if session.state == RelaySessionState::Closed
            || session.reconnect_attempt >= MAX_RECONNECT_ATTEMPTS
            || now.0 > session.expires_at.0
        {
            session.state = RelaySessionState::Failed;
            session.next_reconnect_at = None;
            self.audit(
                actor,
                RelayAction::Reconnect,
                RelayAuditOutcome::Failed,
                correlation_id,
                Some(session_id),
                now,
            )
            .await?;
            self.replace_session(session)?;
            return Err(RelayError::RouteUnavailable);
        }
        session.reconnect_attempt += 1;
        session.state = RelaySessionState::Reconnecting;
        session.next_reconnect_at = Some(UnixMillis(
            now.0
                .checked_add(reconnect_delay_ms(session.reconnect_attempt))
                .ok_or(RelayError::Invalid)?,
        ));
        self.audit(
            actor,
            RelayAction::Reconnect,
            RelayAuditOutcome::Succeeded,
            correlation_id,
            Some(session_id),
            now,
        )
        .await?;
        self.replace_session(session.clone())?;
        Ok(session)
    }

    /// Attempts a scheduled reconnect when its delay has elapsed.
    ///
    /// # Errors
    ///
    /// Returns a denial, early-retry, route, or audit error.
    pub async fn reconnect_due(
        &self,
        actor: &AuthenticatedServerSession,
        session_id: RelaySessionId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<RelaySessionMetadata, RelayError> {
        let mut session = self
            .owned_session_for_action(
                actor,
                RelayAction::Reconnect,
                session_id,
                correlation_id,
                now,
            )
            .await?;
        self.authorize_for_action(
            actor,
            RelayAction::Reconnect,
            session.tenant_id,
            Some(&session.route),
            RelayActionContext {
                correlation_id,
                target: Some(session_id),
                now,
            },
        )
        .await?;
        if session.state != RelaySessionState::Reconnecting
            || session.next_reconnect_at.is_none_or(|due| now.0 < due.0)
        {
            self.audit(
                actor,
                RelayAction::Reconnect,
                RelayAuditOutcome::Invalid,
                correlation_id,
                Some(session_id),
                now,
            )
            .await?;
            return Err(RelayError::RetryNotDue);
        }
        let routed = match &session.route {
            RelayRoute::Peer { .. } => self.router.connect_peer(&session).await,
            RelayRoute::Server { .. } => self.router.connect_server(&session).await,
        };
        if routed.is_err() {
            self.audit(
                actor,
                RelayAction::Reconnect,
                RelayAuditOutcome::Failed,
                correlation_id,
                Some(session_id),
                now,
            )
            .await?;
            return self
                .schedule_reconnect(actor, session_id, correlation_id, now)
                .await;
        }
        session.state = RelaySessionState::Active;
        session.last_seen_at = now;
        session.next_reconnect_at = None;
        self.audit(
            actor,
            RelayAction::Reconnect,
            RelayAuditOutcome::Succeeded,
            correlation_id,
            Some(session_id),
            now,
        )
        .await?;
        self.replace_session(session.clone())?;
        Ok(session)
    }

    /// Closes an authorized relay session idempotently.
    ///
    /// # Errors
    ///
    /// Returns a denial, missing-session, route, or audit error.
    pub async fn close(
        &self,
        actor: &AuthenticatedServerSession,
        session_id: RelaySessionId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<RelaySessionMetadata, RelayError> {
        let mut session = self
            .owned_session_for_action(actor, RelayAction::Close, session_id, correlation_id, now)
            .await?;
        self.authorize_for_action(
            actor,
            RelayAction::Close,
            session.tenant_id,
            Some(&session.route),
            RelayActionContext {
                correlation_id,
                target: Some(session_id),
                now,
            },
        )
        .await?;
        if session.state != RelaySessionState::Closed {
            if let Err(error) = self.router.disconnect(&session).await {
                self.audit(
                    actor,
                    RelayAction::Close,
                    RelayAuditOutcome::Failed,
                    correlation_id,
                    Some(session_id),
                    now,
                )
                .await?;
                return Err(error);
            }
            session.state = RelaySessionState::Closed;
            session.next_reconnect_at = None;
        }
        self.audit(
            actor,
            RelayAction::Close,
            RelayAuditOutcome::Succeeded,
            correlation_id,
            Some(session_id),
            now,
        )
        .await?;
        self.replace_session(session.clone())?;
        Ok(session)
    }

    /// Closes any relay session in the actor's tenant after owner-level
    /// authorization supplied by the server security adapter.
    ///
    /// # Errors
    ///
    /// Returns a denial, missing-session, route, or audit error.
    pub async fn administrative_close(
        &self,
        actor: &AuthenticatedServerSession,
        session_id: RelaySessionId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<RelaySessionMetadata, RelayError> {
        let found = {
            let state = self.state.read().map_err(|_| RelayError::Unavailable)?;
            state
                .sessions
                .get(&session_id)
                .filter(|session| session.tenant_id == actor.tenant_id)
                .cloned()
        };
        let Some(mut session) = found else {
            self.audit(
                actor,
                RelayAction::AdministrativeClose,
                RelayAuditOutcome::Failed,
                correlation_id,
                Some(session_id),
                now,
            )
            .await?;
            return Err(RelayError::NotFound);
        };
        self.authorize_for_action(
            actor,
            RelayAction::AdministrativeClose,
            session.tenant_id,
            Some(&session.route),
            RelayActionContext {
                correlation_id,
                target: Some(session_id),
                now,
            },
        )
        .await?;
        if session.state != RelaySessionState::Closed {
            if let Err(error) = self.router.disconnect(&session).await {
                self.audit(
                    actor,
                    RelayAction::AdministrativeClose,
                    RelayAuditOutcome::Failed,
                    correlation_id,
                    Some(session_id),
                    now,
                )
                .await?;
                return Err(error);
            }
            session.state = RelaySessionState::Closed;
            session.next_reconnect_at = None;
        }
        self.audit(
            actor,
            RelayAction::AdministrativeClose,
            RelayAuditOutcome::Succeeded,
            correlation_id,
            Some(session_id),
            now,
        )
        .await?;
        self.replace_session(session.clone())?;
        Ok(session)
    }

    /// Persists one redacted and bounded relay failure report.
    ///
    /// # Errors
    ///
    /// Returns a denial, invalid-source, or audit availability error.
    pub async fn report_failure(
        &self,
        actor: &AuthenticatedServerSession,
        mut report: RelayFailureReport,
    ) -> Result<(), RelayError> {
        self.authorize_for_action(
            actor,
            RelayAction::ReportFailure,
            report.tenant_id,
            None,
            RelayActionContext {
                correlation_id: report.correlation_id,
                target: report.relay_session_id,
                now: report.occurred_at,
            },
        )
        .await?;
        if report.source_device_id != actor.device_id {
            self.audit(
                actor,
                RelayAction::ReportFailure,
                RelayAuditOutcome::Denied,
                report.correlation_id,
                report.relay_session_id,
                report.occurred_at,
            )
            .await?;
            return Err(RelayError::Denied);
        }
        report.failure_id = RelayFailureId::new(Uuid::new_v4());
        self.audit(
            actor,
            RelayAction::ReportFailure,
            RelayAuditOutcome::Succeeded,
            report.correlation_id,
            report.relay_session_id,
            report.occurred_at,
        )
        .await?;
        let mut state = self.state.write().map_err(|_| RelayError::Unavailable)?;
        if state.failures.len() == MAX_FAILURE_REPORTS {
            state.failures.pop_front();
        }
        state.failures.push_back(report);
        Ok(())
    }

    /// Returns tenant-scoped health after authorization and audit.
    ///
    /// # Errors
    ///
    /// Returns a denial or an audit/state availability error.
    pub async fn health(
        &self,
        actor: &AuthenticatedServerSession,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<RelayHealth, RelayError> {
        self.authorize_for_action(
            actor,
            RelayAction::ReadHealth,
            actor.tenant_id,
            None,
            RelayActionContext {
                correlation_id,
                target: None,
                now,
            },
        )
        .await?;
        let health = {
            let state = self.state.read().map_err(|_| RelayError::Unavailable)?;
            let tenant_sessions = state
                .sessions
                .values()
                .filter(|session| session.tenant_id == actor.tenant_id);
            let (mut active, mut reconnecting, mut failed) = (0_u32, 0_u32, 0_u32);
            for session in tenant_sessions {
                match session.state {
                    RelaySessionState::Active => active += 1,
                    RelaySessionState::Reconnecting => reconnecting += 1,
                    RelaySessionState::Failed => failed += 1,
                    RelaySessionState::Connecting | RelaySessionState::Closed => {}
                }
            }
            let health_state = if !state.accepting_sessions {
                RelayHealthState::Unavailable
            } else if reconnecting > 0 || failed > 0 {
                RelayHealthState::Degraded
            } else {
                RelayHealthState::Healthy
            };
            RelayHealth {
                tenant_id: actor.tenant_id,
                state: health_state,
                checked_at: now,
                active_sessions: active,
                reconnecting_sessions: reconnecting,
                failed_sessions: failed,
                accepting_sessions: state.accepting_sessions,
            }
        };
        self.audit(
            actor,
            RelayAction::ReadHealth,
            RelayAuditOutcome::Succeeded,
            correlation_id,
            None,
            now,
        )
        .await?;
        Ok(health)
    }

    #[cfg(test)]
    fn failure_reports_for_tenant(
        &self,
        tenant_id: eitmad_contracts::identity::TenantId,
    ) -> Vec<RelayFailureReport> {
        self.state.read().map_or_else(
            |_| Vec::new(),
            |state| {
                state
                    .failures
                    .iter()
                    .filter(|report| report.tenant_id == tenant_id)
                    .cloned()
                    .collect()
            },
        )
    }

    async fn authorize_actor(
        &self,
        actor: &AuthenticatedServerSession,
        action: RelayAction,
        tenant_id: eitmad_contracts::identity::TenantId,
        route: Option<&RelayRoute>,
    ) -> Result<(), RelayError> {
        if actor.tenant_id != tenant_id {
            return Err(RelayError::Denied);
        }
        self.security
            .authorize(actor, action, tenant_id, route)
            .await
    }

    async fn connect_route(&self, session: &RelaySessionMetadata) -> Result<(), RelayError> {
        match &session.route {
            RelayRoute::Peer { .. } => self.router.connect_peer(session).await,
            RelayRoute::Server { .. } => self.router.connect_server(session).await,
        }
    }

    async fn authorize_for_action(
        &self,
        actor: &AuthenticatedServerSession,
        action: RelayAction,
        tenant_id: eitmad_contracts::identity::TenantId,
        route: Option<&RelayRoute>,
        context: RelayActionContext,
    ) -> Result<(), RelayError> {
        if let Err(error) = self.authorize_actor(actor, action, tenant_id, route).await {
            self.audit(
                actor,
                action,
                outcome_for(error),
                context.correlation_id,
                context.target,
                context.now,
            )
            .await?;
            return Err(error);
        }
        Ok(())
    }

    async fn audit(
        &self,
        actor: &AuthenticatedServerSession,
        action: RelayAction,
        outcome: RelayAuditOutcome,
        correlation_id: CorrelationId,
        target: Option<RelaySessionId>,
        now: UnixMillis,
    ) -> Result<(), RelayError> {
        self.security
            .audit(actor, action, outcome, correlation_id, target, now)
            .await
    }

    fn owned_session(
        &self,
        actor: &AuthenticatedServerSession,
        session_id: RelaySessionId,
    ) -> Result<RelaySessionMetadata, RelayError> {
        let state = self.state.read().map_err(|_| RelayError::Unavailable)?;
        let session = state
            .sessions
            .get(&session_id)
            .ok_or(RelayError::NotFound)?;
        if session.tenant_id != actor.tenant_id || session.source_device_id != actor.device_id {
            return Err(RelayError::Denied);
        }
        Ok(session.clone())
    }

    async fn owned_session_for_action(
        &self,
        actor: &AuthenticatedServerSession,
        action: RelayAction,
        session_id: RelaySessionId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<RelaySessionMetadata, RelayError> {
        match self.owned_session(actor, session_id) {
            Ok(session) => Ok(session),
            Err(error) => {
                self.audit(
                    actor,
                    action,
                    outcome_for(error),
                    correlation_id,
                    Some(session_id),
                    now,
                )
                .await?;
                Err(error)
            }
        }
    }

    fn replace_session(&self, session: RelaySessionMetadata) -> Result<(), RelayError> {
        self.state
            .write()
            .map_err(|_| RelayError::Unavailable)?
            .sessions
            .insert(session.relay_session_id, session);
        Ok(())
    }
}

const fn reconnect_delay_ms(attempt: u32) -> i64 {
    let exponent = if attempt > 6 { 6 } else { attempt };
    1_000_i64 << exponent
}

fn session_expiry(now: UnixMillis, ttl_ms: u64) -> Result<UnixMillis, RelayError> {
    let ttl = i64::try_from(ttl_ms).map_err(|_| RelayError::Invalid)?;
    now.0
        .checked_add(ttl)
        .map(UnixMillis)
        .ok_or(RelayError::Invalid)
}

const fn outcome_for(error: RelayError) -> RelayAuditOutcome {
    match error {
        RelayError::Denied => RelayAuditOutcome::Denied,
        RelayError::Invalid | RelayError::RetryNotDue => RelayAuditOutcome::Invalid,
        RelayError::NotFound | RelayError::RouteUnavailable | RelayError::Unavailable => {
            RelayAuditOutcome::Failed
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    use eitmad_contracts::{
        identity::{AccountId, DeviceId, TenantId, UserId},
        relay::{RelayFailureCode, RelayFailurePhase, RelayServerId},
        server::AuthenticatedServerSession,
        transport::CorrelationId,
    };

    use super::*;

    #[derive(Default)]
    struct TestSecurity {
        allow: AtomicBool,
        audits: Mutex<Vec<(RelayAction, RelayAuditOutcome)>>,
    }

    #[async_trait]
    impl RelaySecurity for TestSecurity {
        async fn authorize(
            &self,
            _: &AuthenticatedServerSession,
            _: RelayAction,
            _: TenantId,
            _: Option<&RelayRoute>,
        ) -> Result<(), RelayError> {
            self.allow
                .load(Ordering::SeqCst)
                .then_some(())
                .ok_or(RelayError::Denied)
        }

        async fn audit(
            &self,
            _: &AuthenticatedServerSession,
            action: RelayAction,
            outcome: RelayAuditOutcome,
            _: CorrelationId,
            _: Option<RelaySessionId>,
            _: UnixMillis,
        ) -> Result<(), RelayError> {
            self.audits.lock().unwrap().push((action, outcome));
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestRouter {
        fail: AtomicBool,
    }

    #[async_trait]
    impl RelayRouter for TestRouter {
        async fn connect_peer(&self, _: &RelaySessionMetadata) -> Result<(), RelayError> {
            self.connect()
        }
        async fn connect_server(&self, _: &RelaySessionMetadata) -> Result<(), RelayError> {
            self.connect()
        }
        async fn disconnect(&self, _: &RelaySessionMetadata) -> Result<(), RelayError> {
            self.connect()
        }
    }

    impl TestRouter {
        fn connect(&self) -> Result<(), RelayError> {
            (!self.fail.load(Ordering::SeqCst))
                .then_some(())
                .ok_or(RelayError::RouteUnavailable)
        }
    }

    fn actor(tenant: u128, device: u128) -> AuthenticatedServerSession {
        AuthenticatedServerSession {
            session_id: eitmad_contracts::identity::SessionId::new(Uuid::new_v4()),
            account_id: AccountId::new(Uuid::new_v4()),
            user_id: UserId::new(Uuid::new_v4()),
            device_id: DeviceId::new(Uuid::from_u128(device)),
            tenant_id: TenantId::new(Uuid::from_u128(tenant)),
            issued_at: UnixMillis(0),
            expires_at: UnixMillis(i64::MAX),
        }
    }

    fn request(actor: &AuthenticatedServerSession) -> OpenRelaySession {
        OpenRelaySession {
            tenant_id: actor.tenant_id,
            source_device_id: actor.device_id,
            route: RelayRoute::Server {
                target_server_id: RelayServerId::parse("primary").unwrap(),
            },
            requested_ttl_ms: DEFAULT_RELAY_SESSION_TTL_MS,
            correlation_id: CorrelationId::new(Uuid::new_v4()),
        }
    }

    fn coordinator(allow: bool) -> (RelayCoordinator, Arc<TestSecurity>, Arc<TestRouter>) {
        let security = Arc::new(TestSecurity::default());
        security.allow.store(allow, Ordering::SeqCst);
        let router = Arc::new(TestRouter::default());
        (
            RelayCoordinator::new(security.clone(), router.clone()),
            security,
            router,
        )
    }

    #[tokio::test]
    async fn relay_lifecycle_opens_heartbeats_reconnects_and_closes() {
        let (coordinator, security, _) = coordinator(true);
        let actor = actor(1, 2);
        let request = request(&actor);
        let opened = coordinator
            .open(&actor, &request, UnixMillis(10))
            .await
            .unwrap();
        let heartbeat = coordinator
            .heartbeat(
                &actor,
                opened.relay_session_id,
                request.correlation_id,
                UnixMillis(20),
            )
            .await
            .unwrap();
        assert_eq!(heartbeat.last_seen_at, UnixMillis(20));
        let reconnecting = coordinator
            .schedule_reconnect(
                &actor,
                opened.relay_session_id,
                request.correlation_id,
                UnixMillis(30),
            )
            .await
            .unwrap();
        assert_eq!(reconnecting.state, RelaySessionState::Reconnecting);
        assert_eq!(
            coordinator
                .reconnect_due(
                    &actor,
                    opened.relay_session_id,
                    request.correlation_id,
                    UnixMillis(31)
                )
                .await,
            Err(RelayError::RetryNotDue)
        );
        let reconnected = coordinator
            .reconnect_due(
                &actor,
                opened.relay_session_id,
                request.correlation_id,
                reconnecting.next_reconnect_at.unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reconnected.state, RelaySessionState::Active);
        let closed = coordinator
            .close(
                &actor,
                opened.relay_session_id,
                request.correlation_id,
                UnixMillis(100),
            )
            .await
            .unwrap();
        assert_eq!(closed.state, RelaySessionState::Closed);
        assert!(security.audits.lock().unwrap().len() >= 6);
    }

    #[tokio::test]
    async fn denied_sessions_never_reach_the_router_and_are_audited() {
        let (coordinator, security, router) = coordinator(false);
        let actor = actor(1, 2);
        assert_eq!(
            coordinator
                .open(&actor, &request(&actor), UnixMillis(10))
                .await,
            Err(RelayError::Denied)
        );
        assert!(!router.fail.load(Ordering::SeqCst));
        assert_eq!(
            security.audits.lock().unwrap().as_slice(),
            &[(RelayAction::Open, RelayAuditOutcome::Denied)]
        );
    }

    #[tokio::test]
    async fn route_close_failures_are_reported_and_audited() {
        let (coordinator, security, router) = coordinator(true);
        let actor = actor(1, 2);
        let request = request(&actor);
        let opened = coordinator
            .open(&actor, &request, UnixMillis(10))
            .await
            .unwrap();
        router.fail.store(true, Ordering::SeqCst);
        assert_eq!(
            coordinator
                .close(
                    &actor,
                    opened.relay_session_id,
                    request.correlation_id,
                    UnixMillis(20)
                )
                .await,
            Err(RelayError::RouteUnavailable)
        );
        assert_eq!(
            security.audits.lock().unwrap().last(),
            Some(&(RelayAction::Close, RelayAuditOutcome::Failed))
        );
    }

    #[tokio::test]
    async fn tenant_isolation_hides_sessions_and_failure_reports() {
        let (coordinator, _, _) = coordinator(true);
        let first = actor(1, 2);
        let second = actor(3, 4);
        let opened = coordinator
            .open(&first, &request(&first), UnixMillis(10))
            .await
            .unwrap();
        assert_eq!(
            coordinator
                .heartbeat(
                    &second,
                    opened.relay_session_id,
                    CorrelationId::new(Uuid::new_v4()),
                    UnixMillis(20)
                )
                .await,
            Err(RelayError::Denied)
        );
        let report = RelayFailureReport {
            failure_id: RelayFailureId::new(Uuid::nil()),
            relay_session_id: Some(opened.relay_session_id),
            tenant_id: first.tenant_id,
            source_device_id: first.device_id,
            phase: RelayFailurePhase::Route,
            code: RelayFailureCode::parse("eitmad.relay.route-unavailable.v1").unwrap(),
            retryable: true,
            retry_after_ms: Some(1_000),
            occurred_at: UnixMillis(20),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
        };
        coordinator.report_failure(&first, report).await.unwrap();
        assert_eq!(
            coordinator
                .failure_reports_for_tenant(first.tenant_id)
                .len(),
            1
        );
        assert!(
            coordinator
                .failure_reports_for_tenant(second.tenant_id)
                .is_empty()
        );
        assert_eq!(
            coordinator
                .health(&second, CorrelationId::new(Uuid::new_v4()), UnixMillis(30))
                .await
                .unwrap()
                .active_sessions,
            0
        );
    }
}
