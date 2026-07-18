//! Headless and diagnostic entry point for the Rust engine.

use std::{
    io::{self, Write as _},
    path::PathBuf,
    process::ExitCode,
    sync::Arc,
};

use clap::{Parser, Subcommand, ValueEnum};
use eitmad_contracts::runtime::{EngineMode, HealthStatus};
use eitmad_engine_runtime::{
    AuthorityStoreComponent, AuthorityStoreHandle, AuthorityStoreHealthCheck, ProductDispatcher,
    RuntimeBuilder, RuntimeDirectoryHealthCheck, RuntimeFailure, ShutdownReason,
    default_runtime_directory,
    local_ipc::{EventBroker, LocalIpcConfiguration, LocalIpcServer},
};
use serde::Serialize;
use tokio::{
    io::AsyncReadExt as _,
    sync::{broadcast, mpsc, watch},
};

const EXIT_SUCCESS: u8 = 0;
const EXIT_RUNTIME_FAILURE: u8 = 1;
const EXIT_DIAGNOSTIC_UNHEALTHY: u8 = 3;
const DEVELOPMENT_IPC_TOKEN_ENV: &str = "EITMAD_DEVELOPMENT_IPC_TOKEN";

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
        /// Enables temporary bearer-token authentication for development only.
        #[arg(long)]
        allow_insecure_development_auth: bool,
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
            allow_insecure_development_auth,
        } => {
            run(
                mode,
                supervisor_pid,
                runtime_directory,
                ipc_pipe_name,
                allow_insecure_development_auth,
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
    allow_insecure_development_auth: bool,
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
        .health_check(AuthorityStoreHealthCheck::new(&directory));
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
    let dispatcher = Arc::new(ProductDispatcher::new(
        store,
        event_broker.clone(),
        allow_insecure_development_auth,
    ));
    let ipc_task = ipc_pipe_name.map(|pipe_name| {
        let development_token = allow_insecure_development_auth
            .then(|| std::env::var(DEVELOPMENT_IPC_TOKEN_ENV).ok())
            .flatten();
        tokio::spawn(
            LocalIpcServer::new(
                LocalIpcConfiguration::development(pipe_name, development_token),
                dispatcher,
                ipc_shutdown_sender.clone(),
            )
            .with_event_broker(event_broker)
            .run(ipc_cancel_receiver),
        )
    });

    let reason = wait_for_shutdown(mode, &mut ipc_shutdown_receiver).await;
    let outcome = runtime.shutdown(reason).await;
    let _ = ipc_cancel_sender.send(true);
    let ipc_stopped_cleanly = if let Some(task) = ipc_task {
        match task.await {
            Ok(Ok(())) => true,
            Ok(Err(error)) => {
                eprintln!("local IPC server failed: {error}");
                false
            }
            Err(error) => {
                eprintln!("local IPC server task failed: {error}");
                false
            }
        }
    } else {
        true
    };
    if let Err(failure) = &outcome {
        emit_failure(failure);
    }
    let _ = emitter.await;
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
    match serde_json::to_string(failure.contract_error()) {
        Ok(encoded) => eprintln!("{encoded}"),
        Err(_) => eprintln!("{{\"code\":\"eitmad.error.engine-startup-failed.v1\"}}"),
    }
}
