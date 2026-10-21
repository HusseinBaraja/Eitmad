use super::*;

const NOW: u64 = 2_000_000_000;

fn pair(days: u64, name: &str) -> Pair {
    Pair {
        certificate: format!("private/{name}/certificate.pem"),
        private_key: format!("private/{name}/key.pem"),
        not_before: NOW - DAY,
        not_after: NOW + days * DAY,
        ca_not_after: NOW + 365 * DAY,
        fingerprint: name.to_owned(),
    }
}

struct Fake {
    current: Pair,
    running: bool,
    stage_error: Option<Outcome>,
    readiness_failures: usize,
    active: String,
    events: Vec<String>,
    durable: State,
}

impl Default for Fake {
    fn default() -> Self {
        Self {
            current: pair(29, "old"),
            running: true,
            stage_error: None,
            readiness_failures: 0,
            active: "old".into(),
            events: Vec::new(),
            durable: State::default(),
        }
    }
}

impl Platform for Fake {
    fn observe(&mut self) -> Result<(Pair, bool), Outcome> {
        Ok((self.current.clone(), self.running))
    }
    fn stage(&mut self) -> Result<Pair, Outcome> {
        self.events.push("stage".into());
        self.stage_error.map_or_else(|| Ok(pair(90, "new")), Err)
    }
    fn activate(&mut self, pair: &Pair) -> Result<(), Outcome> {
        assert!(
            self.durable.pending.is_some(),
            "activation requires durable recovery state"
        );
        self.events.push(format!("activate:{}", pair.fingerprint));
        self.active.clone_from(&pair.fingerprint);
        Ok(())
    }
    fn restart(&mut self) -> Result<(), Outcome> {
        self.events.push("restart".into());
        Ok(())
    }
    fn ready(&mut self, pair: &Pair) -> Result<(), Outcome> {
        self.events.push(format!("ready:{}", pair.fingerprint));
        if self.readiness_failures > 0 {
            self.readiness_failures -= 1;
            Err(Outcome::RenewalFailed)
        } else {
            Ok(())
        }
    }
    fn save(&mut self, state: &State, event: &str) -> Result<(), Outcome> {
        self.events.push(format!("save:{event}"));
        self.durable = serde_json::from_value(serde_json::to_value(state).unwrap()).unwrap();
        Ok(())
    }
}

#[test]
fn renewal_is_due_from_expiry_including_missed_and_expired_runs() {
    for days in [0, 1, 30] {
        let mut platform = Fake {
            current: pair(days, "old"),
            ..Fake::default()
        };
        let mut state = State::default();
        tick(&mut platform, &mut state, NOW).unwrap();
        assert_eq!(state.outcome, Outcome::Renewed);
        assert_eq!(state.last_success, Some(NOW));
        assert_eq!(platform.active, "new");
        assert!(
            platform
                .events
                .iter()
                .position(|e| e == "ready:new")
                .unwrap()
                < platform
                    .events
                    .iter()
                    .position(|e| e == "save:renewed")
                    .unwrap()
        );
    }
    let mut platform = Fake {
        current: pair(31, "old"),
        ..Fake::default()
    };
    tick(&mut platform, &mut State::default(), NOW).unwrap();
    assert!(!platform.events.iter().any(|e| e == "stage"));
}

#[test]
fn failures_before_activation_preserve_pair_and_report_urgent_expiry() {
    for failure in [Outcome::CredentialUnavailable, Outcome::RenewalFailed] {
        let mut platform = Fake {
            current: pair(2, "old"),
            stage_error: Some(failure),
            ..Fake::default()
        };
        let mut state = State::default();
        assert_eq!(tick(&mut platform, &mut state, NOW), Err(failure));
        assert!(state.urgent);
        assert!(state.pending.is_none());
        assert_eq!(platform.active, "old");
    }
}

#[test]
fn readiness_failure_rolls_back_and_next_run_retries() {
    let mut platform = Fake {
        readiness_failures: 1,
        ..Fake::default()
    };
    let mut state = State::default();
    assert_eq!(
        tick(&mut platform, &mut state, NOW),
        Err(Outcome::RenewalFailed)
    );
    assert_eq!(platform.active, "old");
    assert!(state.pending.is_none());
    assert!(state.last_success.is_none());
    tick(&mut platform, &mut state, NOW + 3600).unwrap();
    assert_eq!(state.outcome, Outcome::Renewed);
}

#[test]
fn interrupted_activation_and_rollback_resume_without_issuing_again() {
    for rolling_back in [false, true] {
        let pending = Pending {
            previous: pair(20, "old"),
            candidate: pair(90, "new"),
            rolling_back,
        };
        let mut state = State {
            pending: Some(pending),
            ..State::default()
        };
        let mut platform = Fake::default();
        platform.save(&state, "prepared").unwrap();
        let result = tick(&mut platform, &mut state, NOW);
        assert_eq!(
            result,
            if rolling_back {
                Err(Outcome::RenewalFailed)
            } else {
                Ok(())
            }
        );
        assert_eq!(platform.active, if rolling_back { "old" } else { "new" });
        assert!(!platform.events.iter().any(|e| e == "stage"));
        assert!(state.pending.is_none());
    }
}

#[test]
fn unsuccessful_rollback_retains_recovery_journal_and_never_activates_expired_pair() {
    for expired in [false, true] {
        let mut platform = Fake {
            current: pair(if expired { 0 } else { 20 }, "old"),
            readiness_failures: 2,
            ..Fake::default()
        };
        let mut state = State::default();
        assert_eq!(
            tick(&mut platform, &mut state, NOW),
            Err(Outcome::RollbackFailed)
        );
        assert!(state.pending.as_ref().unwrap().rolling_back);
        if expired {
            assert!(!platform.events.iter().any(|e| e == "activate:old"));
        }
    }
}

#[test]
fn ca_expiry_warns_early_and_refuses_new_leaf_without_replacing_trust() {
    let mut platform = Fake::default();
    platform.current.ca_not_after = NOW + 150 * DAY;
    platform.current.not_after = NOW + 40 * DAY;
    let mut state = State::default();
    tick(&mut platform, &mut state, NOW).unwrap();
    assert!(state.ca_transition_approaching);
    platform.current.ca_not_after = NOW + 90 * DAY;
    platform.current.not_after = NOW + 20 * DAY;
    assert_eq!(
        tick(&mut platform, &mut state, NOW),
        Err(Outcome::CaTransitionRequired)
    );
    assert!(!platform.events.iter().any(|e| e == "stage"));
    assert_eq!(platform.active, "old");
}

#[test]
fn stopped_host_and_backward_clock_do_not_change_services() {
    let mut platform = Fake {
        running: false,
        ..Fake::default()
    };
    let mut state = State::default();
    assert_eq!(
        tick(&mut platform, &mut state, NOW),
        Err(Outcome::HostStopped)
    );
    assert_eq!(
        tick(&mut platform, &mut state, NOW - 1),
        Err(Outcome::ClockInvalid)
    );
    assert!(!platform.events.iter().any(|e| e == "restart"));
}
