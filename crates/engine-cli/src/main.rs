//! Headless and diagnostic entry point for the Rust engine.

use std::{
    io::{self, Write as _},
    path::PathBuf,
    process::ExitCode,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{Parser, Subcommand, ValueEnum};
use eitmad_contracts::{
    observability::{
        ComponentId, DataClassification, ObservationEventId, ObservationFieldName,
        ObservationSeverity, ObservationValueKind,
    },
    runtime::{EngineMode, HealthStatus},
    transport::{CorrelationId, UnixMillis},
};
use eitmad_engine_runtime::{
    AuthorityStoreComponent, AuthorityStoreHandle, AuthorityStoreHealthCheck, ProductDispatcher,
    RuntimeBuilder, RuntimeDirectoryHealthCheck, RuntimeFailure, ShutdownReason,
    default_runtime_directory,
    local_ipc::{EventBroker, LocalIpcConfiguration, LocalIpcServer},
};
use eitmad_observability_audit::{
    ObservationContract, ObservationFieldContract, ObservationValue, RedactionContext,
};
use eitmad_storage::AuthorityStore;
use serde::Serialize;
use tokio::{
    io::AsyncReadExt as _,
    sync::{broadcast, mpsc, watch},
};

const EXIT_SUCCESS: u8 = 0;
const EXIT_RUNTIME_FAILURE: u8 = 1;
const EXIT_DIAGNOSTIC_UNHEALTHY: u8 = 3;
const IPC_BOOTSTRAP_TOKEN_BYTES: usize = 64;

enum LocalIpcStartupError {
    Authentication,
    Identity,
}

#[derive(Debug, Parser)]
#[command(name = "eitmad-engine-cli", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the authoritative engine in the foreground.
    Run {
        #[arg(long, value_enum)]
        mode: RunMode,
        /// Supervisor PID for correlation only; it grants no trust.
        #[arg(long)]
        supervisor_pid: Option<u32>,
        /// Override the platform runtime-data directory.
        #[arg(long, value_name = "PATH")]
        runtime_directory: Option<PathBuf>,
        /// Windows named-pipe endpoint created by the engine.
        #[arg(long)]
        ipc_pipe_name: Option<String>,
        /// Reads the process-lifetime local IPC bootstrap token from standard input.
        #[arg(long)]
        ipc_bootstrap_stdin: bool,
    },
    /// Run non-mutating preflight and health checks once.
    Diagnose {
        /// Override the platform runtime-data directory.
        #[arg(long, value_name = "PATH")]
        runtime_directory: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum RunMode {
    Headless,
    Supervised,
}

#[tokio::main]
async fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Run {
            mode,
            supervisor_pid,
            runtime_directory,
            ipc_pipe_name,
            ipc_bootstrap_stdin,
        } => {
            run(
                mode,
                supervisor_pid,
                runtime_directory,
                ipc_pipe_name,
                ipc_bootstrap_stdin,
            )
            .await
        }
        Command::Diagnose { runtime_directory } => diagnose(runtime_directory).await,
    }
}

async fn run(
    mode: RunMode,
    supervisor_pid: Option<u32>,
    runtime_directory: Option<PathBuf>,
    ipc_pipe_name: Option<String>,
    ipc_bootstrap_stdin: bool,
) -> ExitCode {
    let Some(directory) = resolve_or_emit_runtime_directory(runtime_directory) else {
        return ExitCode::from(EXIT_RUNTIME_FAILURE);
    };
    let engine_mode = match mode {
        RunMode::Headless => EngineMode::Headless,
        RunMode::Supervised => EngineMode::SupervisedDesktop,
    };
    let store_handle = AuthorityStoreHandle::default();
    let mut builder = RuntimeBuilder::new(engine_mode, &directory)
        .component(AuthorityStoreComponent::new(
            &directory,
            store_handle.clone(),
        ))
        .health_check(RuntimeDirectoryHealthCheck::new(&directory))
        .health_check(AuthorityStoreHealthCheck::started(store_handle.clone()));
    if let Some(process_id) = supervisor_pid {
        builder = builder.supervisor_process_id(process_id);
    }
    let mut runtime = builder.build();
    if emit_json(runtime.snapshot()).is_err() {
        return ExitCode::from(EXIT_RUNTIME_FAILURE);
    }
    let events = runtime.subscribe();
    let emitter = tokio::spawn(emit_lifecycle_events(events));

    if let Err(failure) = runtime.start().await {
        emit_failure(&failure);
        let _ = emitter.await;
        return ExitCode::from(EXIT_RUNTIME_FAILURE);
    }
    let Ok(store) = store_handle.store() else {
        let failure = RuntimeFailure::component_unavailable();
        emit_failure(&failure);
        let _ = runtime.shutdown(ShutdownReason::Explicit).await;
        let _ = emitter.await;
        return ExitCode::from(EXIT_RUNTIME_FAILURE);
    };

    let (ipc_shutdown_sender, mut ipc_shutdown_receiver) = mpsc::channel(1);
    let _ipc_shutdown_guard = ipc_shutdown_sender.clone();
    let (ipc_cancel_sender, ipc_cancel_receiver) = watch::channel(false);
    let event_broker = EventBroker::new();
    let dispatcher = Arc::new(ProductDispatcher::new(store.clone(), event_broker.clone()));
    if dispatcher.drain_pending_publications().is_err() {
        emit_operational_failure(
            "eitmad.observation.event-publication-recovery-failed.v1",
            "eitmad.error.event-publication-recovery-failed.v1",
        );
        let _ = runtime.shutdown(ShutdownReason::Explicit).await;
        let _ = emitter.await;
        return ExitCode::from(EXIT_RUNTIME_FAILURE);
    }
    let ipc_configuration =
        match prepare_local_ipc(&store, ipc_pipe_name, ipc_bootstrap_stdin).await {
            Ok(configuration) => configuration,
            Err(error) => {
                match error {
                    LocalIpcStartupError::Authentication => emit_operational_failure(
                        "eitmad.observation.local-ipc-authentication-failed.v1",
                        "eitmad.error.local-ipc-authentication-failed.v1",
                    ),
                    LocalIpcStartupError::Identity => emit_operational_failure(
                        "eitmad.observation.local-identity-unavailable.v1",
                        "eitmad.error.local-identity-unavailable.v1",
                    ),
                }
                let _ = runtime.shutdown(ShutdownReason::Explicit).await;
                let _ = emitter.await;
                return ExitCode::from(EXIT_RUNTIME_FAILURE);
            }
        };
    let ipc_task = ipc_configuration.map(|configuration| {
        tokio::spawn(
            LocalIpcServer::new(configuration, dispatcher, ipc_shutdown_sender.clone())
                .with_event_broker(event_broker)
                .run(ipc_cancel_receiver),
        )
    });

    let reason = wait_for_shutdown(mode, &mut ipc_shutdown_receiver).await;
    let _ = ipc_cancel_sender.send(true);
    let ipc_stopped_cleanly = await_local_ipc(ipc_task).await;
    let outcome = runtime.shutdown(reason).await;
    let _ = emitter.await;
    exit_after_shutdown(&outcome, ipc_stopped_cleanly)
}

async fn await_local_ipc(task: Option<tokio::task::JoinHandle<io::Result<()>>>) -> bool {
    let Some(task) = task else {
        return true;
    };
    match task.await {
        Ok(Ok(())) => true,
        Ok(Err(_)) => {
            emit_operational_failure(
                "eitmad.observation.local-ipc-server-failed.v1",
                "eitmad.error.local-ipc-server-failed.v1",
            );
            false
        }
        Err(_) => {
            emit_operational_failure(
                "eitmad.observation.local-ipc-task-failed.v1",
                "eitmad.error.local-ipc-task-failed.v1",
            );
            false
        }
    }
}

async fn prepare_local_ipc(
    store: &AuthorityStore,
    pipe_name: Option<String>,
    read_token_from_stdin: bool,
) -> Result<Option<LocalIpcConfiguration>, LocalIpcStartupError> {
    let Some(pipe_name) = pipe_name else {
        return Ok(None);
    };
    if !read_token_from_stdin {
        return Err(LocalIpcStartupError::Authentication);
    }
    let bootstrap_token = read_ipc_bootstrap_token()
        .await
        .map_err(|_| LocalIpcStartupError::Authentication)?;
    let authorization = store
        .local_authorization_context(current_time())
        .map_err(|_| LocalIpcStartupError::Identity)?;
    Ok(Some(LocalIpcConfiguration::authenticated(
        pipe_name,
        bootstrap_token,
        authorization,
    )))
}

async fn read_ipc_bootstrap_token() -> io::Result<String> {
    let mut bytes = [0_u8; IPC_BOOTSTRAP_TOKEN_BYTES + 1];
    tokio::io::stdin().read_exact(&mut bytes).await?;
    if bytes[IPC_BOOTSTRAP_TOKEN_BYTES] != b'\n'
        || !bytes[..IPC_BOOTSTRAP_TOKEN_BYTES]
            .iter()
            .all(u8::is_ascii_hexdigit)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid local IPC bootstrap token",
        ));
    }
    String::from_utf8(bytes[..IPC_BOOTSTRAP_TOKEN_BYTES].to_vec()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid local IPC bootstrap token",
        )
    })
}

fn exit_after_shutdown(
    outcome: &Result<(), RuntimeFailure>,
    ipc_stopped_cleanly: bool,
) -> ExitCode {
    if let Err(failure) = outcome {
        emit_failure(failure);
    }
    ExitCode::from(if outcome.is_ok() && ipc_stopped_cleanly {
        EXIT_SUCCESS
    } else {
        EXIT_RUNTIME_FAILURE
    })
}

async fn diagnose(runtime_directory: Option<PathBuf>) -> ExitCode {
    let Some(directory) = resolve_or_emit_runtime_directory(runtime_directory) else {
        return ExitCode::from(EXIT_RUNTIME_FAILURE);
    };
    let report = RuntimeBuilder::new(EngineMode::Diagnostic, &directory)
        .health_check(RuntimeDirectoryHealthCheck::new(&directory))
        .health_check(AuthorityStoreHealthCheck::new(&directory))
        .diagnose()
        .await;
    if emit_json(&report).is_err() {
        return ExitCode::from(EXIT_RUNTIME_FAILURE);
    }
    ExitCode::from(
        if report.status == HealthStatus::Healthy && report.ready_to_start {
            EXIT_SUCCESS
        } else {
            EXIT_DIAGNOSTIC_UNHEALTHY
        },
    )
}

fn resolve_runtime_directory(directory: Option<PathBuf>) -> Result<PathBuf, RuntimeFailure> {
    directory
        .or_else(|| default_runtime_directory().ok())
        .ok_or_else(RuntimeFailure::runtime_directory_unavailable)
}

fn resolve_or_emit_runtime_directory(directory: Option<PathBuf>) -> Option<PathBuf> {
    match resolve_runtime_directory(directory) {
        Ok(directory) => Some(directory),
        Err(failure) => {
            emit_failure(&failure);
            None
        }
    }
}

async fn wait_for_shutdown(mode: RunMode, ipc_shutdown: &mut mpsc::Receiver<()>) -> ShutdownReason {
    match mode {
        RunMode::Headless => {
            tokio::select! {
                result = tokio::signal::ctrl_c() => {
                    let _ = result;
                    ShutdownReason::Interrupt
                }
                value = ipc_shutdown.recv() => {
                    let _ = value;
                    ShutdownReason::Explicit
                }
            }
        }
        RunMode::Supervised => {
            tokio::select! {
                result = tokio::signal::ctrl_c() => {
                    let _ = result;
                    ShutdownReason::Interrupt
                }
                () = wait_for_supervisor_pipe_close() => ShutdownReason::SupervisorLost,
                value = ipc_shutdown.recv() => {
                    let _ = value;
                    ShutdownReason::Explicit
                },
            }
        }
    }
}

async fn wait_for_supervisor_pipe_close() {
    let mut stdin = tokio::io::stdin();
    let mut buffer = [0_u8; 64];
    loop {
        match stdin.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
    }
}

async fn emit_lifecycle_events(
    mut events: broadcast::Receiver<eitmad_contracts::runtime::LifecycleSnapshot>,
) {
    loop {
        match events.recv().await {
            Ok(snapshot) => {
                let terminal = snapshot.state.is_terminal();
                if emit_json(&snapshot).is_err() || terminal {
                    return;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {}
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}

fn emit_json(value: &impl Serialize) -> Result<(), serde_json::Error> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer(&mut lock, value)?;
    lock.write_all(b"\n").map_err(serde_json::Error::io)?;
    lock.flush().map_err(serde_json::Error::io)
}

fn emit_failure(failure: &RuntimeFailure) {
    let safe = failure.contract_error().redacted_for_external_boundary();
    match serde_json::to_string(&safe) {
        Ok(encoded) => eprintln!("{encoded}"),
        Err(_) => eprintln!("{{\"code\":\"eitmad.error.engine-startup-failed.v1\"}}"),
    }
}

fn emit_operational_failure(event_id: &str, error_code: &str) {
    let correlation_id = CorrelationId::new(uuid::Uuid::new_v4());
    let field_name = ObservationFieldName::parse("error-code")
        .expect("static observation field name must be valid");
    let contract = ObservationContract::new(
        ObservationEventId::parse(event_id).expect("static observation event ID must be valid"),
        [ObservationFieldContract {
            name: field_name.clone(),
            classification: DataClassification::Metadata,
            value_kind: ObservationValueKind::Identifier,
        }],
    )
    .expect("static observation contract must be valid");
    let log = contract
        .redact(
            current_time(),
            ComponentId::parse("engine-cli").expect("static component ID must be valid"),
            ObservationSeverity::Error,
            correlation_id,
            [(
                field_name,
                ObservationValue::Identifier(error_code.to_owned()),
            )],
            RedactionContext::metadata_only(),
        )
        .expect("static metadata must match its observation contract");
    match serde_json::to_string(&log) {
        Ok(encoded) => eprintln!("{encoded}"),
        Err(_) => eprintln!(
            "{{\"eventId\":\"eitmad.observation.serialization-failed.v1\",\"correlationId\":\"{}\"}}",
            correlation_id.value()
        ),
    }
}

fn current_time() -> UnixMillis {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    UnixMillis(i64::try_from(millis).unwrap_or(i64::MAX))
}
