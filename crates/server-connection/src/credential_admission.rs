use std::sync::{Condvar, Mutex};
use std::time::Instant;

use eitmad_observability_audit::operation_diagnostics::{
    DiagnosticPhase, DiagnosticResult, DiagnosticSpan,
};
use eitmad_sync::{FailurePhase, RetryAdvice, TransportFailure, TransportFailureKind};

/// Serializes one stored rotating pair without polling or an unbounded wait.
#[derive(Default)]
pub(super) struct CredentialGate {
    state: Mutex<State>,
    released: Condvar,
}

#[derive(Default)]
struct State {
    held: bool,
    poisoned: bool,
}

pub(super) struct CredentialGuard<'a>(&'a CredentialGate);

impl CredentialGate {
    pub(super) fn acquire(
        &self,
        deadline: Instant,
    ) -> Result<CredentialGuard<'_>, TransportFailure> {
        let mut span = DiagnosticSpan::start(DiagnosticPhase::CredentialAdmission);
        let poison = || {
            TransportFailure::new(
                TransportFailureKind::DriverUnavailable,
                FailurePhase::Authentication,
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
                    return Ok(CredentialGuard(self));
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

impl Drop for CredentialGuard<'_> {
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
    fn contention_expires_without_ownership_and_panics_fail_closed() {
        let gate = CredentialGate::default();
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
