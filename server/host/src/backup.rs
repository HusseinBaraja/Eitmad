//! Server-backup policy. Windows supplies observations and performs OS operations.
use eitmad_contracts::backup::{
    ServerBackupConfiguration as Configuration, ServerBackupOutcome as Outcome,
    ServerBackupStatus as Status,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyError {
    Invalid,
}

/// Validate policy before any native action. Windows separately verifies paths and protection.
///
/// # Errors
/// Returns `Invalid` for unsupported versions, unsafe paths, or unbounded policy.
pub fn validate(config: &Configuration) -> Result<(), PolicyError> {
    let mut paths = vec![
        &config.host_root,
        &config.recovery_directory,
        &config.local_destination,
    ];
    if let Some(external) = &config.external_destination {
        paths.push(external);
    }
    if config.version != 1
        || config.deployment_id.is_nil()
        || !(1..=24).contains(&config.interval_hours)
        || !(2..=31).contains(&config.retention_count)
        || !(1_073_741_824..=1_099_511_627_776).contains(&config.reserve_bytes)
        || config.service_name.is_empty()
        || config.service_name.len() > 32
        || !config.service_name.as_bytes()[0].is_ascii_alphabetic()
        || !config
            .service_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric())
    {
        return Err(PolicyError::Invalid);
    }
    for path in &paths {
        let bytes = path.as_bytes();
        if bytes.len() < 4
            || !bytes[0].is_ascii_alphabetic()
            || &bytes[1..3] != b":\\"
            || path.contains(['/', '\r', '\n', '"', '%', '*', '?', '<', '>', '|', '~'])
            || path.chars().any(char::is_control)
            || path[3..]
                .split('\\')
                .any(|p| p.is_empty() || p.ends_with(['.', ' ']) || p.contains(':') || reserved(p))
        {
            return Err(PolicyError::Invalid);
        }
    }
    for (index, left) in paths.iter().enumerate() {
        for right in paths.iter().skip(index + 1) {
            let left = left.to_ascii_lowercase();
            let right = right.to_ascii_lowercase();
            if left == right
                || left.starts_with(&format!("{right}\\"))
                || right.starts_with(&format!("{left}\\"))
            {
                return Err(PolicyError::Invalid);
            }
        }
    }
    Ok(())
}

fn reserved(component: &str) -> bool {
    let name = component
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(name.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (name.len() == 4
            && (name.starts_with("COM") || name.starts_with("LPT"))
            && matches!(name.as_bytes()[3], b'1'..=b'9'))
}

#[must_use]
pub fn initial(config: &Configuration, now: u64) -> Status {
    Status {
        version: 1,
        deployment_id: config.deployment_id,
        last_attempt_at: None,
        last_local_success_at: None,
        last_external_success_at: None,
        local_outcome: Outcome::NeverRun,
        external_outcome: Outcome::NeverRun,
        next_due_at: now,
        overdue: true,
        external_configured: config.external_destination.is_some(),
        disk_loss_protected: false,
        schedule_outcome: Outcome::NeverRun,
    }
}

/// One catch-up run is due after a missed slot. Never replay every missed interval.
pub fn refresh(config: &Configuration, status: &mut Status, now: u64) {
    let interval = u64::from(config.interval_hours) * 3600;
    status.next_due_at = status
        .last_local_success_at
        .map_or(0, |cut| cut.saturating_add(interval));
    status.overdue =
        now >= status.next_due_at || status.last_local_success_at.is_some_and(|cut| cut > now);
    status.disk_loss_protected = status.external_configured
        && status.external_outcome == Outcome::Succeeded
        && status
            .last_external_success_at
            .is_some_and(|cut| cut <= now && now - cut < interval);
}

/// Only recognized, complete, unheld generations are supplied by the adapter.
#[must_use]
pub fn expired(mut generations: Vec<String>, retain: usize) -> Vec<String> {
    generations.sort_unstable_by(|a, b| b.cmp(a));
    generations.into_iter().skip(retain).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Configuration {
        serde_json::from_value(serde_json::json!({
            "version":1, "deploymentId":"11111111-1111-4111-8111-111111111111",
            "hostRoot":"C:\\Host", "serviceName":"Eitmad", "recoveryDirectory":"C:\\Recovery",
            "localDestination":"C:\\Backups", "externalDestination":"D:\\Backups"
        }))
        .unwrap()
    }
    #[test]
    fn rejects_overlap_and_unbounded_policy() {
        let mut c = config();
        assert!(validate(&c).is_ok());
        c.external_destination = Some("c:\\backups\\nested".into());
        assert!(validate(&c).is_err());
        c = config();
        c.retention_count = 1000;
        assert!(validate(&c).is_err());
        c = config();
        c.local_destination = "C:\\Backups\\..\\Host".into();
        assert!(validate(&c).is_err());
        for unsafe_path in [
            "C:\\Host.",
            "C:\\NUL",
            "C:\\Backups:stream",
            "C:\\PROGRA~1\\Backups",
        ] {
            c = config();
            c.local_destination = unsafe_path.into();
            assert!(validate(&c).is_err());
        }
    }
    #[test]
    fn missed_run_and_external_failure_do_not_hide_local_success() {
        let c = config();
        let mut s = initial(&c, 0);
        s.last_local_success_at = Some(100);
        s.last_external_success_at = Some(100);
        s.external_outcome = Outcome::Succeeded;
        refresh(&c, &mut s, 101);
        assert!(s.disk_loss_protected);
        assert!(!s.overdue);
        refresh(&c, &mut s, 100 + 3 * 86400);
        assert!(s.overdue);
        assert!(!s.disk_loss_protected);
        s.external_outcome = Outcome::DestinationUnavailable;
        s.last_local_success_at = Some(300_000);
        refresh(&c, &mut s, 300_001);
        assert!(!s.overdue);
        assert!(!s.disk_loss_protected);
    }
    #[test]
    fn retention_keeps_newest_and_bounds_deletion() {
        assert_eq!(
            expired(vec!["001".into(), "003".into(), "002".into()], 2),
            vec!["001"]
        );
    }
}
