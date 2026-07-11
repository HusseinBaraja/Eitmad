# Update policy ownership

**Owner:** Rust release and update maintainers.

This crate owns signed-manifest verification, update eligibility, channel and rollout policy, compatibility ranges, migration preflight, revocation, and the authoritative update state machine.

It does not install artifacts. Native platform adapters own installation mechanics and report verified results through contracts.

Changes require signature, key-rotation, rollout, compatibility, interruption, recovery, and audit tests.

