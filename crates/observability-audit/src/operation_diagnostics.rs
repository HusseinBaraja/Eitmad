//! Opt-in, process-bounded troubleshooting. Callers cannot supply payload text.
use std::{
    cell::Cell,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    time::Instant,
};

use eitmad_contracts::transport::CorrelationId;
use uuid::Uuid;

thread_local! { static CORRELATION: Cell<Option<CorrelationId>> = const { Cell::new(None) }; }
thread_local! { static SOURCE: Cell<DiagnosticSource> = const { Cell::new(DiagnosticSource::Unspecified) }; }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSource {
    Unspecified,
    Quotations,
    Orders,
    WorkOrders,
    Approvals,
    Catalog,
    Pricing,
    Images,
}
#[must_use]
pub fn diagnostic_source() -> DiagnosticSource {
    SOURCE.get()
}
static EVENTS: AtomicUsize = AtomicUsize::new(0);
const LIMIT: usize = 4096;
static OUTPUT: OnceLock<Option<SyncSender<String>>> = OnceLock::new();
fn output_channel(mut output: impl std::io::Write + Send + 'static) -> Option<SyncSender<String>> {
    let (send, receive) = sync_channel::<String>(256);
    std::thread::Builder::new()
        .name("diagnostic-output".into())
        .spawn(move || {
            for line in receive {
                let _ = writeln!(output, "{line}");
            }
        })
        .ok()
        .map(|_| send)
}
fn enqueue(line: String) {
    if let Some(sender) = OUTPUT.get_or_init(|| output_channel(std::io::stderr())) {
        let _ = sender.try_send(line);
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DiagnosticPhase {
    Worker,
    DriverAdmission,
    CredentialAdmission,
    CredentialRefresh,
    Dns,
    Connect,
    Tls,
    Send,
    Receive,
    Confirmation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticResult {
    Started,
    Succeeded,
    GateBusy,
    Poisoned,
    Deadline,
    Cancelled,
    Network,
    IoTimeout,
    Denied,
    Invalid,
    Server,
    Failed,
}

#[must_use]
pub fn diagnostic_correlation() -> Option<CorrelationId> {
    CORRELATION.get()
}

/// Carries an IPC correlation across a blocking-worker boundary and restores prior state.
pub fn with_diagnostic_context<T>(
    id: CorrelationId,
    source: DiagnosticSource,
    action: impl FnOnce() -> T,
) -> T {
    struct Restore(Option<CorrelationId>, DiagnosticSource);
    impl Drop for Restore {
        fn drop(&mut self) {
            CORRELATION.set(self.0);
            SOURCE.set(self.1);
        }
    }
    let _restore = Restore(CORRELATION.replace(Some(id)), SOURCE.replace(source));
    action()
}

/// One synchronous phase; unfinished scopes report failure, never synthetic success.
pub struct DiagnosticSpan {
    correlation: CorrelationId,
    phase: DiagnosticPhase,
    source: DiagnosticSource,
    started: Instant,
    result: DiagnosticResult,
}
impl DiagnosticSpan {
    #[must_use]
    pub fn start(phase: DiagnosticPhase) -> Self {
        let span = Self {
            correlation: CORRELATION
                .get()
                .unwrap_or_else(|| CorrelationId::new(Uuid::new_v4())),
            phase,
            source: SOURCE.get(),
            started: Instant::now(),
            result: DiagnosticResult::Failed,
        };
        span.emit(DiagnosticResult::Started);
        span
    }
    pub fn finish(&mut self, result: DiagnosticResult) {
        self.result = result;
    }
    fn emit(&self, result: DiagnosticResult) {
        if std::env::var_os("EITMAD_DIAGNOSTICS").as_deref() != Some(std::ffi::OsStr::new("1")) {
            return;
        }
        if EVENTS
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                (n < LIMIT).then_some(n + 1)
            })
            .is_err()
        {
            return;
        }
        // Fixed keys/enums and an opaque ID only. Never format an underlying error.
        let line = serde_json::json!({"diagnostic":"eitmad.operation.v1", "correlation":self.correlation, "source":format!("{:?}", self.source), "phase":format!("{:?}", self.phase), "result":format!("{result:?}"), "elapsedMs":u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)});
        enqueue(line.to_string());
    }
}
impl Drop for DiagnosticSpan {
    fn drop(&mut self) {
        self.emit(self.result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_and_panicking_workers_restore_correlation() {
        let first = CorrelationId::new(Uuid::from_u128(1));
        let second = CorrelationId::new(Uuid::from_u128(2));
        with_diagnostic_context(first, DiagnosticSource::Orders, || {
            assert_eq!(
                DiagnosticSpan::start(DiagnosticPhase::Worker).correlation,
                first
            );
            let _ = std::panic::catch_unwind(|| {
                with_diagnostic_context(second, DiagnosticSource::Approvals, || {
                    assert_eq!(
                        DiagnosticSpan::start(DiagnosticPhase::Receive).correlation,
                        second
                    );
                    panic!("synthetic failure");
                })
            });
            assert_eq!(CORRELATION.get(), Some(first));
            assert_eq!(diagnostic_source(), DiagnosticSource::Orders);
        });
        assert_eq!(CORRELATION.get(), None);
        assert_eq!(diagnostic_source(), DiagnosticSource::Unspecified);
    }
    #[test]
    fn blocked_output_drops_without_blocking_the_caller() {
        struct Blocked {
            entered: std::sync::mpsc::Sender<()>,
            release: Option<std::sync::mpsc::Receiver<()>>,
        }
        impl std::io::Write for Blocked {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if let Some(release) = self.release.take() {
                    let _ = self.entered.send(());
                    let _ = release.recv();
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let (entered, ready) = std::sync::mpsc::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let output = output_channel(Blocked {
            entered,
            release: Some(wait),
        })
        .unwrap();
        output.try_send("synthetic".into()).unwrap();
        ready
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let start = Instant::now();
        let dropped = (0..1024)
            .filter(|_| output.try_send("synthetic".into()).is_err())
            .count();
        let _ = release.send(());
        assert!(dropped > 0);
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
    }
}
