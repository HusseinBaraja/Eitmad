# Mandatory CI status checks

**Owner:** Release engineering and security maintainers.

Branch protection for `main` must require every job from `.github/workflows/mandatory-validation.yml`:

- `repository-policy`
- `rust-quality`
- `contracts-and-ipc`
- `windows-desktop`
- `windows-supervision`
- `macos-bindings`
- `server-smoke`
- `package-windows (windows-desktop)`
- `package-windows (windows-server)`
- `package-server`
- `validation-complete`

The final `validation-complete` job fails when any dependency is skipped or fails. Repository administrators must not use path filters, optional checks, or bypass rules for release pull requests.

Packaging starts alongside the other validation jobs. Both Windows matrix entries must pass; one failure does not cancel the other. Uploaded unsigned bundles are validation evidence only and cannot be promoted before every gate passes.

`rust-quality` runs all workspace unit and integration tests with pinned Nextest, without retries, and runs documentation tests separately with Cargo. The full workspace suite owns Rust IPC and engine lifecycle coverage. `windows-supervision` owns the engine diagnostic smoke check and fake/real-engine supervision, alongside the independent `windows-desktop` shell suite. The focused contract and supervision workflows remain available through manual dispatch; they do not repeat these automatic gates.

The `contracts-and-ipc` job installs the pinned contract generator with lifecycle scripts disabled, then runs `npm audit --audit-level=high`. A high or critical advisory blocks contract generation and release validation. Update the pinned generator, verify generated binding drift, and rerun the full contract job. Do not suppress an advisory without an accepted security decision and a bounded replacement date.

The repository-policy job always compares the candidate with a base revision. Pull requests use the pull request base SHA, pushes use the prior SHA, and manual runs require a base SHA input. The comparison rejects changes to or deletion of an existing server migration. A schema change must use a new, numbered migration path and update `deploy/migrations.sha256`.
