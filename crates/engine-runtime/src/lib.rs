//! Authoritative engine process lifecycle and request orchestration.
//!
//! This crate owns process lifecycle behavior. External wire shapes remain in
//! `eitmad-contracts`, and launchers remain thin adapters over this API.

mod authority;
pub mod local_ipc;

use std::{
    env,
    error::Error,
    fmt,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    process,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use authority::{AuthorityLock, AuthorityLockError};
use eitmad_contracts::{
    errors::{ContractError, ErrorCode, ErrorDetail, MessageId, RetryDisposition},
    runtime::{
        DiagnosticReport, EngineInstanceId, EngineMode, EngineProcessIdentity, HealthCheckId,
        HealthCheckImpact, HealthCheckResult, HealthStatus, LifecycleSnapshot, LifecycleStage,
        LifecycleState,
    },
    transport::{CorrelationId, UnixMillis},
    updates::ReleaseVersion,
};
use tokio::{sync::broadcast, time::Instant};
use uuid::Uuid;

pub const DEFAULT_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

pub type ComponentFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), ComponentFailure>> + Send + 'a>>;
pub type HealthCheckFuture<'a> = Pin<Box<dyn Future<Output = HealthStatus> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComponentFailure;

impl ComponentFailure {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ComponentFailure {
    fn default() -> Self {
        Self::new()
    }
}

pub trait RuntimeComponent: Send {
    fn start(&mut self) -> ComponentFuture<'_>;
    fn stop(&mut self) -> ComponentFuture<'_>;
}

pub trait HealthCheck: Send + Sync {
    fn id(&self) -> HealthCheckId;
    fn impact(&self) -> HealthCheckImpact;
    fn check(&self) -> HealthCheckFuture<'_>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownReason {
    Explicit,
    Interrupt,
    SupervisorLost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeFailure {
    contract_error: Box<ContractError>,
}

impl RuntimeFailure {
    #[must_use]
    pub const fn contract_error(&self) -> &ContractError {
        &self.contract_error
    }

    #[must_use]
    pub fn runtime_directory_unavailable() -> Self {
        startup_failure(LifecycleStage::AuthorityLock)
    }
}

impl fmt::Display for RuntimeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.contract_error.code)
    }
}

impl Error for RuntimeFailure {}

pub struct RuntimeBuilder {
    mode: EngineMode,
    runtime_directory: PathBuf,
    supervisor_process_id: Option<u32>,
    startup_timeout: Duration,
    shutdown_timeout: Duration,
    components: Vec<Box<dyn RuntimeComponent>>,
    checks: Vec<Box<dyn HealthCheck>>,
}

impl RuntimeBuilder {
    #[must_use]
    pub fn new(mode: EngineMode, runtime_directory: impl Into<PathBuf>) -> Self {
        Self {
            mode,
            runtime_directory: runtime_directory.into(),
            supervisor_process_id: None,
            startup_timeout: DEFAULT_STARTUP_TIMEOUT,
            shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT,
            components: Vec::new(),
            checks: Vec::new(),
        }
    }

    #[must_use]
    pub const fn supervisor_process_id(mut self, process_id: u32) -> Self {
        self.supervisor_process_id = Some(process_id);
        self
    }

    #[must_use]
    pub const fn startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    #[must_use]
    pub const fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    #[must_use]
    pub fn component(mut self, component: impl RuntimeComponent + 'static) -> Self {
        self.components.push(Box::new(component));
        self
    }

    #[must_use]
    pub fn health_check(mut self, check: impl HealthCheck + 'static) -> Self {
        self.checks.push(Box::new(check));
        self
    }

    #[must_use]
    pub fn build(self) -> EngineRuntime {
        EngineRuntime::from_builder(self)
    }

    pub async fn diagnose(mut self) -> DiagnosticReport {
        self.mode = EngineMode::Diagnostic;
        self.supervisor_process_id = None;
        let mut runtime = EngineRuntime::from_builder(self);
        let checks = runtime.run_checks(None).await;
        let status = overall_health(&checks);
        DiagnosticReport {
            identity: runtime.snapshot.identity.clone(),
            status,
            ready_to_start: required_checks_pass(&checks),
            checks,
            observed_at: now(),
        }
    }
}

pub struct EngineRuntime {
    runtime_directory: PathBuf,
    supervisor_process_id: Option<u32>,
    startup_timeout: Duration,
    shutdown_timeout: Duration,
    components: Vec<Box<dyn RuntimeComponent>>,
    started_components: usize,
    checks: Vec<Box<dyn HealthCheck>>,
    authority_lock: Option<AuthorityLock>,
    snapshot: LifecycleSnapshot,
    events: broadcast::Sender<LifecycleSnapshot>,
    terminal_error: Option<RuntimeFailure>,
}

impl EngineRuntime {
    fn from_builder(builder: RuntimeBuilder) -> Self {
        let identity = EngineProcessIdentity::new(
            EngineInstanceId::new(Uuid::new_v4()),
            process::id(),
            builder.mode,
            now(),
            ReleaseVersion::new(
                semver::Version::parse(env!("CARGO_PKG_VERSION"))
                    .expect("Cargo package version must be valid semver"),
            ),
        );
        let snapshot = LifecycleSnapshot {
            identity,
            state: LifecycleState::Starting,
            live: true,
            ready: false,
            health: HealthStatus::Healthy,
            checks: Vec::new(),
            observed_at: now(),
            error: None,
        };
        let (events, _) = broadcast::channel(32);

        Self {
            runtime_directory: builder.runtime_directory,
            supervisor_process_id: builder.supervisor_process_id,
            startup_timeout: builder.startup_timeout,
            shutdown_timeout: builder.shutdown_timeout,
            components: builder.components,
            started_components: 0,
            checks: builder.checks,
            authority_lock: None,
            snapshot,
            events,
            terminal_error: None,
        }
    }

    #[must_use]
    pub const fn snapshot(&self) -> &LifecycleSnapshot {
        &self.snapshot
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<LifecycleSnapshot> {
        self.events.subscribe()
    }

    /// Starts the authoritative runtime and evaluates readiness.
    ///
    /// # Errors
    ///
    /// Returns a sanitized structured failure when identity validation,
    /// authority locking, component startup, or a required health check fails.
    pub async fn start(&mut self) -> Result<(), RuntimeFailure> {
        if let Err(failure) = self.validate_process_identity() {
            return self.fail(failure);
        }

        match AuthorityLock::acquire(&self.runtime_directory, &self.snapshot.identity) {
            Ok(lock) => self.authority_lock = Some(lock),
            Err(AuthorityLockError::AlreadyRunning) => {
                return self.fail(failure(
                    "eitmad.error.engine-already-running.v1",
                    "eitmad.message.engine-already-running.v1",
                    RetryDisposition::SafeAfterDelay(1_000),
                    LifecycleStage::AuthorityLock,
                ));
            }
            Err(AuthorityLockError::Unavailable) => {
                return self.fail(startup_failure(LifecycleStage::AuthorityLock));
            }
        }

        let deadline = Instant::now() + self.startup_timeout;
        while self.started_components < self.components.len() {
            let result =
                tokio::time::timeout_at(deadline, self.components[self.started_components].start())
                    .await;
            if !matches!(result, Ok(Ok(()))) {
                self.rollback_started_components().await;
                self.release_authority_lock();
                return self.fail(startup_failure(LifecycleStage::ComponentStartup));
            }
            self.started_components += 1;
        }

        let health_deadline = Instant::now() + self.startup_timeout;
        let checks = self.run_checks(Some(health_deadline)).await;
        let status = overall_health(&checks);
        self.snapshot.checks = checks;
        self.snapshot.health = status;
        if !required_checks_pass(&self.snapshot.checks) {
            self.rollback_started_components().await;
            self.release_authority_lock();
            return self.fail(failure(
                "eitmad.error.engine-health-check-failed.v1",
                "eitmad.message.engine-health-check-failed.v1",
                RetryDisposition::SafeAfterDelay(1_000),
                LifecycleStage::ReadinessCheck,
            ));
        }

        self.transition(LifecycleState::Ready, None);
        Ok(())
    }

    pub async fn refresh_health(&mut self) -> HealthStatus {
        let checks = self.run_checks(None).await;
        let status = overall_health(&checks);
        self.snapshot.checks = checks;
        self.snapshot.health = status;
        self.snapshot.ready = self.snapshot.state == LifecycleState::Ready
            && required_checks_pass(&self.snapshot.checks);
        self.snapshot.observed_at = now();
        self.publish();
        status
    }

    /// Stops initialized components in reverse registration order.
    ///
    /// # Errors
    ///
    /// Returns a sanitized structured failure if draining exceeds its deadline
    /// or a component cannot stop cleanly.
    pub async fn shutdown(&mut self, _reason: ShutdownReason) -> Result<(), RuntimeFailure> {
        if self.snapshot.state == LifecycleState::Stopped {
            return Ok(());
        }
        if self.snapshot.state == LifecycleState::Failed {
            return Err(self.terminal_error.clone().unwrap_or_else(|| {
                failure(
                    "eitmad.error.engine-shutdown-failed.v1",
                    "eitmad.message.engine-shutdown-failed.v1",
                    RetryDisposition::Never,
                    LifecycleStage::ComponentShutdown,
                )
            }));
        }

        self.transition(LifecycleState::Stopping, None);
        let deadline = Instant::now() + self.shutdown_timeout;
        while self.started_components > 0 {
            let index = self.started_components - 1;
            let result = tokio::time::timeout_at(deadline, self.components[index].stop()).await;
            if !matches!(result, Ok(Ok(()))) {
                self.release_authority_lock();
                return self.fail(failure(
                    "eitmad.error.engine-shutdown-failed.v1",
                    "eitmad.message.engine-shutdown-failed.v1",
                    RetryDisposition::Never,
                    LifecycleStage::ComponentShutdown,
                ));
            }
            self.started_components -= 1;
        }
        self.release_authority_lock();
        self.transition(LifecycleState::Stopped, None);
        Ok(())
    }

    fn validate_process_identity(&self) -> Result<(), RuntimeFailure> {
        let valid = match self.snapshot.identity.mode {
            EngineMode::SupervisedDesktop => self
                .supervisor_process_id
                .is_some_and(|id| id != 0 && id != process::id()),
            EngineMode::Headless => self.supervisor_process_id.is_none(),
            EngineMode::Diagnostic => false,
        };
        if valid {
            Ok(())
        } else {
            Err(failure(
                "eitmad.error.engine-supervisor-invalid.v1",
                "eitmad.message.engine-supervisor-invalid.v1",
                RetryDisposition::Never,
                LifecycleStage::ProcessIdentity,
            ))
        }
    }

    async fn run_checks(&mut self, deadline: Option<Instant>) -> Vec<HealthCheckResult> {
        let mut results = Vec::with_capacity(self.checks.len());
        for check in &self.checks {
            let status = if let Some(deadline) = deadline {
                tokio::time::timeout_at(deadline, check.check())
                    .await
                    .unwrap_or(HealthStatus::Unhealthy)
            } else {
                check.check().await
            };
            let error = (status != HealthStatus::Healthy).then(|| {
                failure(
                    "eitmad.error.engine-health-check-failed.v1",
                    "eitmad.message.engine-health-check-failed.v1",
                    RetryDisposition::SafeAfterDelay(1_000),
                    LifecycleStage::ReadinessCheck,
                )
                .contract_error
                .as_ref()
                .clone()
            });
            results.push(HealthCheckResult {
                id: check.id(),
                status,
                impact: check.impact(),
                observed_at: now(),
                error,
            });
        }
        results
    }

    async fn rollback_started_components(&mut self) {
        let deadline = Instant::now() + self.shutdown_timeout;
        while self.started_components > 0 {
            let index = self.started_components - 1;
            let _ = tokio::time::timeout_at(deadline, self.components[index].stop()).await;
            self.started_components -= 1;
        }
    }

    fn release_authority_lock(&mut self) {
        if let Some(lock) = self.authority_lock.take() {
            let _ = lock.release();
        }
    }

    fn transition(&mut self, state: LifecycleState, error: Option<ContractError>) {
        assert!(
            valid_transition(self.snapshot.state, state),
            "invalid lifecycle transition: {:?} -> {state:?}",
            self.snapshot.state
        );
        self.snapshot.state = state;
        self.snapshot.live = state.is_live();
        self.snapshot.ready =
            state == LifecycleState::Ready && required_checks_pass(&self.snapshot.checks);
        self.snapshot.observed_at = now();
        self.snapshot.error = error;
        self.publish();
    }

    fn fail<T>(&mut self, failure: RuntimeFailure) -> Result<T, RuntimeFailure> {
        self.transition(
            LifecycleState::Failed,
            Some(failure.contract_error.as_ref().clone()),
        );
        self.terminal_error = Some(failure.clone());
        Err(failure)
    }

    fn publish(&self) {
        let _ = self.events.send(self.snapshot.clone());
    }
}

fn required_checks_pass(checks: &[HealthCheckResult]) -> bool {
    checks.iter().all(|check| {
        check.impact != HealthCheckImpact::RequiredForReadiness
            || check.status == HealthStatus::Healthy
    })
}

fn overall_health(checks: &[HealthCheckResult]) -> HealthStatus {
    if checks.iter().any(|check| {
        check.impact == HealthCheckImpact::RequiredForReadiness
            && check.status != HealthStatus::Healthy
    }) {
        HealthStatus::Unhealthy
    } else if checks
        .iter()
        .any(|check| check.status != HealthStatus::Healthy)
    {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    }
}

const fn valid_transition(from: LifecycleState, to: LifecycleState) -> bool {
    matches!(
        (from, to),
        (
            LifecycleState::Starting,
            LifecycleState::Ready | LifecycleState::Stopping | LifecycleState::Failed
        ) | (
            LifecycleState::Ready,
            LifecycleState::Stopping | LifecycleState::Failed
        ) | (
            LifecycleState::Stopping,
            LifecycleState::Stopped | LifecycleState::Failed
        )
    )
}

fn startup_failure(stage: LifecycleStage) -> RuntimeFailure {
    failure(
        "eitmad.error.engine-startup-failed.v1",
        "eitmad.message.engine-startup-failed.v1",
        RetryDisposition::SafeAfterDelay(1_000),
        stage,
    )
}

fn failure(
    code: &str,
    message_id: &str,
    retry: RetryDisposition,
    stage: LifecycleStage,
) -> RuntimeFailure {
    RuntimeFailure {
        contract_error: Box::new(ContractError {
            code: ErrorCode::parse(code).expect("runtime error code must be valid"),
            message_id: MessageId::parse(message_id).expect("runtime message ID must be valid"),
            parameters: Vec::new(),
            retry,
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            detail: Some(ErrorDetail::Lifecycle { stage }),
        }),
    }
}

fn now() -> UnixMillis {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let millis = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
    UnixMillis(millis)
}

/// Returns the platform-specific engine runtime-data directory.
///
/// # Errors
///
/// Returns an error when the platform environment does not expose a suitable
/// user-local data or state directory.
pub fn default_runtime_directory() -> Result<PathBuf, RuntimeDirectoryError> {
    #[cfg(target_os = "windows")]
    let base = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library").join("Application Support"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local/state"))
        });

    base.map(|path| path.join("Eitmad").join("engine"))
        .ok_or(RuntimeDirectoryError)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeDirectoryError;

impl fmt::Display for RuntimeDirectoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("engine runtime directory is unavailable")
    }
}

impl Error for RuntimeDirectoryError {}

pub struct RuntimeDirectoryHealthCheck {
    runtime_directory: PathBuf,
}

impl RuntimeDirectoryHealthCheck {
    #[must_use]
    pub fn new(runtime_directory: impl Into<PathBuf>) -> Self {
        Self {
            runtime_directory: runtime_directory.into(),
        }
    }
}

impl HealthCheck for RuntimeDirectoryHealthCheck {
    fn id(&self) -> HealthCheckId {
        HealthCheckId::parse("eitmad.health.runtime-directory.v1")
            .expect("runtime directory health ID must be valid")
    }

    fn impact(&self) -> HealthCheckImpact {
        HealthCheckImpact::RequiredForReadiness
    }

    fn check(&self) -> HealthCheckFuture<'_> {
        Box::pin(async move {
            if nearest_existing_ancestor(&self.runtime_directory).is_some_and(Path::is_dir) {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            }
        })
    }
}

fn nearest_existing_ancestor(path: &Path) -> Option<&Path> {
    path.ancestors().find(|candidate| candidate.exists())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tempfile::TempDir;
    use tokio::time::sleep;

    use super::*;

    struct RecordingComponent {
        name: &'static str,
        events: Arc<Mutex<Vec<String>>>,
        fail_start: bool,
        fail_stop: bool,
        delay: Duration,
    }

    impl RuntimeComponent for RecordingComponent {
        fn start(&mut self) -> ComponentFuture<'_> {
            Box::pin(async move {
                sleep(self.delay).await;
                self.events
                    .lock()
                    .expect("recording lock")
                    .push(format!("start:{}", self.name));
                if self.fail_start {
                    Err(ComponentFailure::new())
                } else {
                    Ok(())
                }
            })
        }

        fn stop(&mut self) -> ComponentFuture<'_> {
            Box::pin(async move {
                sleep(self.delay).await;
                self.events
                    .lock()
                    .expect("recording lock")
                    .push(format!("stop:{}", self.name));
                if self.fail_stop {
                    Err(ComponentFailure::new())
                } else {
                    Ok(())
                }
            })
        }
    }

    struct FixedHealthCheck {
        id: &'static str,
        impact: HealthCheckImpact,
        status: HealthStatus,
    }

    impl HealthCheck for FixedHealthCheck {
        fn id(&self) -> HealthCheckId {
            HealthCheckId::parse(self.id).expect("test health ID")
        }

        fn impact(&self) -> HealthCheckImpact {
            self.impact
        }

        fn check(&self) -> HealthCheckFuture<'_> {
            Box::pin(async move { self.status })
        }
    }

    struct DelayedHealthCheck {
        delay: Duration,
    }

    impl HealthCheck for DelayedHealthCheck {
        fn id(&self) -> HealthCheckId {
            HealthCheckId::parse("eitmad.health.delayed.v1").expect("test health ID")
        }

        fn impact(&self) -> HealthCheckImpact {
            HealthCheckImpact::RequiredForReadiness
        }

        fn check(&self) -> HealthCheckFuture<'_> {
            Box::pin(async move {
                sleep(self.delay).await;
                HealthStatus::Healthy
            })
        }
    }

    fn component(name: &'static str, events: &Arc<Mutex<Vec<String>>>) -> RecordingComponent {
        RecordingComponent {
            name,
            events: Arc::clone(events),
            fail_start: false,
            fail_stop: false,
            delay: Duration::ZERO,
        }
    }

    #[test]
    fn lifecycle_transitions_are_explicit() {
        assert!(valid_transition(
            LifecycleState::Starting,
            LifecycleState::Ready
        ));
        assert!(valid_transition(
            LifecycleState::Ready,
            LifecycleState::Stopping
        ));
        assert!(valid_transition(
            LifecycleState::Stopping,
            LifecycleState::Stopped
        ));
        assert!(!valid_transition(
            LifecycleState::Starting,
            LifecycleState::Stopped
        ));
        assert!(!valid_transition(
            LifecycleState::Stopped,
            LifecycleState::Ready
        ));
    }

    #[tokio::test]
    async fn starts_in_order_becomes_ready_and_stops_in_reverse() {
        let directory = TempDir::new().expect("temp directory");
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = RuntimeBuilder::new(EngineMode::Headless, directory.path())
            .component(component("one", &events))
            .component(component("two", &events))
            .health_check(FixedHealthCheck {
                id: "eitmad.health.required.v1",
                impact: HealthCheckImpact::RequiredForReadiness,
                status: HealthStatus::Healthy,
            })
            .build();

        runtime.start().await.expect("startup");
        assert_eq!(runtime.snapshot().state, LifecycleState::Ready);
        assert!(runtime.snapshot().ready);
        runtime
            .shutdown(ShutdownReason::Explicit)
            .await
            .expect("shutdown");
        runtime
            .shutdown(ShutdownReason::Explicit)
            .await
            .expect("idempotent shutdown");

        assert_eq!(
            *events.lock().expect("recording lock"),
            ["start:one", "start:two", "stop:two", "stop:one"]
        );
        assert_eq!(runtime.snapshot().state, LifecycleState::Stopped);
    }

    #[tokio::test]
    async fn shutdown_before_start_is_clean_and_idempotent() {
        let directory = TempDir::new().expect("temp directory");
        let mut runtime = RuntimeBuilder::new(EngineMode::Headless, directory.path()).build();

        runtime
            .shutdown(ShutdownReason::Explicit)
            .await
            .expect("shutdown before start");
        runtime
            .shutdown(ShutdownReason::Explicit)
            .await
            .expect("idempotent shutdown");

        assert_eq!(runtime.snapshot().state, LifecycleState::Stopped);
    }

    #[tokio::test]
    async fn failed_start_rolls_back_initialized_components() {
        let directory = TempDir::new().expect("temp directory");
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut failing = component("two", &events);
        failing.fail_start = true;
        let mut runtime = RuntimeBuilder::new(EngineMode::Headless, directory.path())
            .component(component("one", &events))
            .component(failing)
            .build();

        let error = runtime.start().await.expect_err("startup must fail");

        assert_eq!(
            error.contract_error().code.as_str(),
            "eitmad.error.engine-startup-failed.v1"
        );
        assert_eq!(runtime.snapshot().state, LifecycleState::Failed);
        assert_eq!(
            *events.lock().expect("recording lock"),
            ["start:one", "start:two", "stop:one"]
        );
    }

    #[tokio::test]
    async fn required_health_failure_blocks_readiness() {
        let directory = TempDir::new().expect("temp directory");
        let mut runtime = RuntimeBuilder::new(EngineMode::Headless, directory.path())
            .health_check(FixedHealthCheck {
                id: "eitmad.health.required.v1",
                impact: HealthCheckImpact::RequiredForReadiness,
                status: HealthStatus::Degraded,
            })
            .build();

        let error = runtime
            .start()
            .await
            .expect_err("health must block startup");

        assert_eq!(
            error.contract_error().code.as_str(),
            "eitmad.error.engine-health-check-failed.v1"
        );
        assert!(!runtime.snapshot().ready);
        assert_eq!(runtime.snapshot().health, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn advisory_health_failure_degrades_without_blocking() {
        let directory = TempDir::new().expect("temp directory");
        let mut runtime = RuntimeBuilder::new(EngineMode::Headless, directory.path())
            .health_check(FixedHealthCheck {
                id: "eitmad.health.advisory.v1",
                impact: HealthCheckImpact::Advisory,
                status: HealthStatus::Unhealthy,
            })
            .build();

        runtime.start().await.expect("advisory startup");

        assert!(runtime.snapshot().ready);
        assert_eq!(runtime.snapshot().health, HealthStatus::Degraded);
        runtime
            .shutdown(ShutdownReason::Explicit)
            .await
            .expect("shutdown");
    }

    #[tokio::test]
    async fn startup_and_shutdown_deadlines_fail_structurally() {
        let startup_directory = TempDir::new().expect("temp directory");
        let startup_events = Arc::new(Mutex::new(Vec::new()));
        let mut slow_start = component("slow", &startup_events);
        slow_start.delay = Duration::from_millis(50);
        let mut startup_runtime =
            RuntimeBuilder::new(EngineMode::Headless, startup_directory.path())
                .startup_timeout(Duration::from_millis(1))
                .component(slow_start)
                .build();
        assert_eq!(
            startup_runtime
                .start()
                .await
                .expect_err("startup timeout")
                .contract_error()
                .code
                .as_str(),
            "eitmad.error.engine-startup-failed.v1"
        );

        let shutdown_directory = TempDir::new().expect("temp directory");
        let shutdown_events = Arc::new(Mutex::new(Vec::new()));
        let mut slow_stop = component("slow", &shutdown_events);
        slow_stop.delay = Duration::from_millis(50);
        let mut shutdown_runtime =
            RuntimeBuilder::new(EngineMode::Headless, shutdown_directory.path())
                .shutdown_timeout(Duration::from_millis(1))
                .component(slow_stop)
                .build();
        shutdown_runtime.start().await.expect("startup");
        assert_eq!(
            shutdown_runtime
                .shutdown(ShutdownReason::Explicit)
                .await
                .expect_err("shutdown timeout")
                .contract_error()
                .code
                .as_str(),
            "eitmad.error.engine-shutdown-failed.v1"
        );
    }

    #[tokio::test]
    async fn readiness_checks_receive_a_separate_startup_budget() {
        let directory = TempDir::new().expect("temp directory");
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut slow_start = component("slow", &events);
        slow_start.delay = Duration::from_millis(120);
        let mut runtime = RuntimeBuilder::new(EngineMode::Headless, directory.path())
            .startup_timeout(Duration::from_millis(200))
            .component(slow_start)
            .health_check(DelayedHealthCheck {
                delay: Duration::from_millis(120),
            })
            .build();

        runtime
            .start()
            .await
            .expect("readiness check has an independent deadline");
        assert_eq!(runtime.snapshot().state, LifecycleState::Ready);
        runtime
            .shutdown(ShutdownReason::Explicit)
            .await
            .expect("shutdown");
    }

    #[tokio::test]
    async fn authority_lock_rejects_duplicate_and_releases_cleanly() {
        let directory = TempDir::new().expect("temp directory");
        let mut first = RuntimeBuilder::new(EngineMode::Headless, directory.path()).build();
        let mut second = RuntimeBuilder::new(EngineMode::Headless, directory.path()).build();
        assert_ne!(
            first.snapshot().identity.instance_id,
            second.snapshot().identity.instance_id
        );
        first.start().await.expect("first authority");
        assert_eq!(
            second
                .start()
                .await
                .expect_err("duplicate authority")
                .contract_error()
                .code
                .as_str(),
            "eitmad.error.engine-already-running.v1"
        );
        first
            .shutdown(ShutdownReason::Explicit)
            .await
            .expect("release authority");

        let mut replacement = RuntimeBuilder::new(EngineMode::Headless, directory.path()).build();
        replacement.start().await.expect("replacement authority");
        replacement
            .shutdown(ShutdownReason::Explicit)
            .await
            .expect("replacement shutdown");
    }

    #[tokio::test]
    async fn diagnostics_do_not_acquire_authority() {
        let directory = TempDir::new().expect("temp directory");
        let mut runtime = RuntimeBuilder::new(EngineMode::Headless, directory.path()).build();
        runtime.start().await.expect("authority");

        let report = RuntimeBuilder::new(EngineMode::Headless, directory.path())
            .health_check(FixedHealthCheck {
                id: "eitmad.health.diagnostic.v1",
                impact: HealthCheckImpact::RequiredForReadiness,
                status: HealthStatus::Healthy,
            })
            .diagnose()
            .await;

        assert!(report.ready_to_start);
        assert_eq!(report.status, HealthStatus::Healthy);
        assert_eq!(report.identity.mode, EngineMode::Diagnostic);
        runtime
            .shutdown(ShutdownReason::Explicit)
            .await
            .expect("shutdown");
    }
}
