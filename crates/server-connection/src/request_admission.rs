use std::sync::{Condvar, Mutex};
use std::time::Instant;

use eitmad_observability_audit::operation_diagnostics::{
    DiagnosticPhase, DiagnosticResult, DiagnosticSpan,
};
use eitmad_sync::{FailurePhase, RetryAdvice, TransportFailure, TransportFailureKind};

/// Serializes a connection operation without polling or an unbounded wait.
pub(super) struct RequestGate {
    phase: DiagnosticPhase,
    failure_phase: FailurePhase,
    state: Mutex<State>,
    released: Condvar,
}

#[derive(Default)]
struct State {
    held: bool,
    poisoned: bool,
}

pub(super) struct RequestGuard<'a>(&'a RequestGate);

impl RequestGate {
    pub(super) fn new(phase: DiagnosticPhase, failure_phase: FailurePhase) -> Self {
        Self {
            phase,
            failure_phase,
            state: Mutex::new(State::default()),
            released: Condvar::new(),
        }
    }
    pub(super) fn acquire(&self, deadline: Instant) -> Result<RequestGuard<'_>, TransportFailure> {
        let mut span = DiagnosticSpan::start(self.phase);
        let poison = || {
            TransportFailure::new(
                TransportFailureKind::DriverUnavailable,
                self.failure_phase,
                RetryAdvice::Never,
            )
        };
        let result = (|| {
            let mut state = self.state.lock().map_err(|_| poison())?;
            loop {
                let remaining = super::remaining_io(deadline)
                    .map_err(|_| super::unavailable(FailurePhase::Cancellation))?;
                if state.poisoned {
                    return Err(poison());
                }
                if !state.held {
                    state.held = true;
                    return Ok(RequestGuard(self));
                }
                (state, _) = self
                    .released
                    .wait_timeout(state, remaining)
                    .map_err(|_| poison())?;
            }
        })();
        span.finish(result.as_ref().map_or_else(
            |cause| super::diagnostics::failure_result(cause, deadline),
            |_| DiagnosticResult::Succeeded,
        ));
        result
    }
}

impl Drop for RequestGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.poisoned |= std::thread::panicking();
            state.held = false;
            self.0.released.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn catalog_waiter_acquires_after_the_active_worker_releases() {
        let gate = RequestGate::new(DiagnosticPhase::DriverAdmission, FailurePhase::Connect);
        let deadline = Instant::now() + Duration::from_secs(2);
        let owner = gate.acquire(deadline).unwrap();
        let (entered, waiting) = std::sync::mpsc::channel();
        let (finished, result) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            let gate = &gate;
            let worker = scope.spawn(move || {
                entered.send(()).unwrap();
                let _owner = gate.acquire(deadline).unwrap();
                finished.send(()).unwrap();
            });
            waiting.recv_timeout(Duration::from_secs(1)).unwrap();
            assert!(matches!(
                result.recv_timeout(Duration::from_millis(100)),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            ));
            drop(owner);
            result.recv_timeout(Duration::from_secs(1)).unwrap();
            worker.join().unwrap();
        });
    }

    #[test]
    fn contention_expires_without_ownership_and_panics_fail_closed() {
        let gate = RequestGate::new(
            DiagnosticPhase::CredentialAdmission,
            FailurePhase::Authentication,
        );
        let end = Instant::now() + Duration::from_secs(2);
        let held = gate.acquire(end).unwrap();
        let started = Instant::now();
        let error = gate
            .acquire(started + Duration::from_millis(50))
            .err()
            .unwrap();
        assert_eq!(error.phase, FailurePhase::Cancellation);
        assert!(started.elapsed() >= Duration::from_millis(50));
        assert!(started.elapsed() < Duration::from_secs(1));
        drop(held);
        assert!(
            gate.acquire(Instant::now()).is_err(),
            "Expired callers cannot take a free gate"
        );
        drop(gate.acquire(end).unwrap());
        let _ = std::panic::catch_unwind(|| {
            let _held = gate.acquire(end).unwrap();
            panic!("synthetic refresh panic");
        });
        assert_eq!(
            gate.acquire(end).err().unwrap().kind,
            TransportFailureKind::DriverUnavailable
        );
    }
}
