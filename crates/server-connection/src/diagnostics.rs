use eitmad_observability_audit::operation_diagnostics::{
    DiagnosticPhase, DiagnosticResult, DiagnosticSpan,
};
use eitmad_sync::{FailurePhase, TransportFailure, TransportFailureKind};
use std::time::Instant;

pub(super) fn failure_result(failure: &TransportFailure, deadline: Instant) -> DiagnosticResult {
    if Instant::now() >= deadline {
        return DiagnosticResult::Deadline;
    }
    match failure.kind {
        TransportFailureKind::RetryNotReady => DiagnosticResult::GateBusy,
        TransportFailureKind::DriverUnavailable => DiagnosticResult::Poisoned,
        TransportFailureKind::Cancelled => DiagnosticResult::Cancelled,
        TransportFailureKind::AuthenticationFailed | TransportFailureKind::EncryptionRequired => {
            DiagnosticResult::Denied
        }
        _ => DiagnosticResult::Network,
    }
}

pub(super) fn measure<T>(
    phase: DiagnosticPhase,
    deadline: Instant,
    action: impl FnOnce() -> Result<T, TransportFailure>,
) -> Result<T, TransportFailure> {
    let mut span = DiagnosticSpan::start(phase);
    let result = action();
    span.finish(result.as_ref().map_or_else(
        |failure| match failure_result(failure, deadline) {
            DiagnosticResult::Network => DiagnosticResult::Failed,
            result => result,
        },
        |_| DiagnosticResult::Succeeded,
    ));
    result
}

pub(super) fn io_result(error: &std::io::Error, deadline: Instant) -> DiagnosticResult {
    if Instant::now() >= deadline {
        DiagnosticResult::Deadline
    } else if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        DiagnosticResult::IoTimeout
    } else {
        DiagnosticResult::Network
    }
}

pub(super) fn measure_io<T>(
    phase: DiagnosticPhase,
    failure_phase: FailurePhase,
    deadline: Instant,
    action: impl FnOnce() -> std::io::Result<T>,
) -> Result<T, TransportFailure> {
    let mut span = DiagnosticSpan::start(phase);
    let result = action();
    span.finish(result.as_ref().map_or_else(
        |error| io_result(error, deadline),
        |_| DiagnosticResult::Succeeded,
    ));
    result.map_err(|_| super::unavailable(failure_phase))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn deadline_and_network_preserve_distinct_typed_causes() {
        let future = Instant::now() + Duration::from_secs(5);
        let past = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        let network = super::super::unavailable(FailurePhase::Connect);
        assert_eq!(failure_result(&network, past), DiagnosticResult::Deadline);
        assert_eq!(failure_result(&network, future), DiagnosticResult::Network);
        let mapped = crate::authenticated_http::HttpError::Unavailable(network.clone());
        let crate::authenticated_http::HttpError::Unavailable(retained) = mapped else {
            panic!("cause lost")
        };
        assert_eq!(retained, network);
    }
    #[test]
    fn withheld_loopback_reply_reports_io_timeout_before_the_request_deadline() {
        use std::{
            io::Read as _,
            net::{TcpListener, TcpStream},
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (_peer, _) = listener.accept().unwrap();
        client
            .set_read_timeout(Some(Duration::from_millis(40)))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let error = client.read(&mut [0; 1]).unwrap_err();
        assert_eq!(io_result(&error, deadline), DiagnosticResult::IoTimeout);
        assert!(Instant::now() < deadline);
        let other_windows_timeout = std::io::Error::from(std::io::ErrorKind::WouldBlock);
        assert_eq!(
            io_result(&other_windows_timeout, deadline),
            DiagnosticResult::IoTimeout
        );
        assert_eq!(
            io_result(&error, Instant::now()),
            DiagnosticResult::Deadline
        );
        let refused = std::io::Error::from(std::io::ErrorKind::ConnectionRefused);
        assert_eq!(io_result(&refused, deadline), DiagnosticResult::Network);
    }
}
