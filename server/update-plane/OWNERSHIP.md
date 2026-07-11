# Update plane ownership

**Owner:** Release infrastructure maintainers.

This plane owns signed manifest and artifact publication, channel metadata, staged rollout inputs, pause/revocation controls, and distribution availability.

It does not decide client eligibility or migration safety; the Rust engine update policy does. Changes require signing, key-rotation, integrity, authorization, rollback-control, CDN, and availability tests.

