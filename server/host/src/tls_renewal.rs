//! Retained-CA renewal policy and crash recovery. Native adapters perform mechanics only.
use serde::{Deserialize, Serialize};

const DAY: u64 = 86_400;
pub const CHECK_INTERVAL_SECONDS: u64 = 3_600;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pair {
    pub certificate: String,
    pub private_key: String,
    pub not_before: u64,
    pub not_after: u64,
    pub ca_not_after: u64,
    pub fingerprint: String,
}

impl Pair {
    fn usable(&self, now: u64) -> bool {
        self.not_before <= now && now < self.not_after && now < self.ca_not_after
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pending {
    pub previous: Pair,
    pub candidate: Pair,
    pub rolling_back: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    #[default]
    NeverRun,
    NotDue,
    Renewed,
    CredentialUnavailable,
    CaTransitionRequired,
    RenewalFailed,
    HostStopped,
    ClockInvalid,
    RollbackFailed,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub version: u8,
    pub outcome: Outcome,
    pub last_attempt: Option<u64>,
    pub last_success: Option<u64>,
    pub leaf_expires: Option<u64>,
    pub ca_expires: Option<u64>,
    pub urgent: bool,
    pub ca_transition_approaching: bool,
    pub schedule_healthy: bool,
    pub pending: Option<Pending>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: 1,
            outcome: Outcome::NeverRun,
            last_attempt: None,
            last_success: None,
            leaf_expires: None,
            ca_expires: None,
            urgent: false,
            ca_transition_approaching: false,
            schedule_healthy: false,
            pending: None,
        }
    }
}

/// The privileged adapter must authorize paths and identities before any retained-state read.
pub trait Platform {
    /// # Errors
    /// Fails if the installed pair, trust, or owned service cannot be inspected safely.
    fn observe(&mut self) -> Result<(Pair, bool), Outcome>;
    /// # Errors
    /// Fails on missing credentials, signing errors, or incomplete durable pair writes.
    fn stage(&mut self) -> Result<Pair, Outcome>;
    /// # Errors
    /// Fails if pair validation or atomic publication fails. Preserve the prior references.
    fn activate(&mut self, pair: &Pair) -> Result<(), Outcome>;
    /// # Errors
    /// Fails if graceful stop or start of the owned host exceeds its deadline.
    fn restart(&mut self) -> Result<(), Outcome>;
    /// # Errors
    /// Fails unless verified HTTPS serves this exact leaf and returns readiness HTTP 204.
    fn ready(&mut self, pair: &Pair) -> Result<(), Outcome>;
    /// Publish durable state atomically and append a sanitized, installation-scoped audit event.
    ///
    /// # Errors
    /// Fails on a durable write, audit, or atomic publication error.
    fn save(&mut self, state: &State, event: &str) -> Result<(), Outcome>;
}

/// Check observed expiry on every invocation, including the first invocation after a missed run.
///
/// # Errors
/// Returns a typed failure after retaining transaction state for recovery.
pub fn tick(platform: &mut impl Platform, state: &mut State, now: u64) -> Result<(), Outcome> {
    if state.version != 1 {
        return Err(Outcome::RenewalFailed);
    }
    if state.last_attempt.is_some_and(|previous| now < previous) {
        return report(platform, state, Outcome::ClockInvalid);
    }
    state.last_attempt = Some(now);
    if let Some(pending) = &state.pending {
        state.urgent = pending.previous.not_after <= now.saturating_add(7 * DAY);
        state.ca_transition_approaching =
            pending.candidate.ca_not_after <= now.saturating_add(180 * DAY);
        return complete(platform, state, now);
    }
    let (previous, running) = match platform.observe() {
        Ok(observation) => observation,
        Err(outcome) => return report(platform, state, outcome),
    };
    state.leaf_expires = Some(previous.not_after);
    state.ca_expires = Some(previous.ca_not_after);
    state.urgent = previous.not_after <= now.saturating_add(7 * DAY);
    state.ca_transition_approaching = previous.ca_not_after <= now.saturating_add(180 * DAY);
    if previous.not_before > now {
        return report(platform, state, Outcome::ClockInvalid);
    }
    if previous.not_after > now.saturating_add(30 * DAY) {
        state.outcome = Outcome::NotDue;
        return platform.save(state, "checked");
    }
    if previous.ca_not_after < now.saturating_add(91 * DAY) {
        return report(platform, state, Outcome::CaTransitionRequired);
    }
    if !running {
        return report(platform, state, Outcome::HostStopped);
    }
    let candidate = match platform.stage() {
        Ok(pair) if pair.usable(now) && pair.not_after > previous.not_after => pair,
        Ok(_) => return report(platform, state, Outcome::RenewalFailed),
        Err(outcome) => return report(platform, state, outcome),
    };
    state.pending = Some(Pending {
        previous,
        candidate,
        rolling_back: false,
    });
    platform.save(state, "prepared")?;
    complete(platform, state, now)
}

fn load(platform: &mut impl Platform, pair: &Pair) -> Result<(), Outcome> {
    platform.activate(pair)?;
    platform.restart()?;
    platform.ready(pair)
}

fn complete(platform: &mut impl Platform, state: &mut State, now: u64) -> Result<(), Outcome> {
    let pending = state.pending.clone().ok_or(Outcome::RenewalFailed)?;
    if !pending.rolling_back {
        if pending.candidate.usable(now) && load(platform, &pending.candidate).is_ok() {
            state.leaf_expires = Some(pending.candidate.not_after);
            state.ca_expires = Some(pending.candidate.ca_not_after);
            state.urgent = false;
            state.last_success = Some(now);
            state.pending = None;
            state.outcome = Outcome::Renewed;
            return platform.save(state, "renewed");
        }
        state
            .pending
            .as_mut()
            .ok_or(Outcome::RenewalFailed)?
            .rolling_back = true;
        platform.save(state, "rollback-started")?;
    }
    if !pending.previous.usable(now) || load(platform, &pending.previous).is_err() {
        return report(platform, state, Outcome::RollbackFailed);
    }
    state.pending = None;
    report(platform, state, Outcome::RenewalFailed)
}

fn report(
    platform: &mut impl Platform,
    state: &mut State,
    outcome: Outcome,
) -> Result<(), Outcome> {
    state.outcome = outcome;
    platform.save(state, "failed")?;
    Err(outcome)
}

#[cfg(test)]
mod tests;
