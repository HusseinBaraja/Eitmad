# Mandatory CI status checks

**Owner:** Release engineering and security maintainers.

Branch protection for `main` must require every job from `.github/workflows/mandatory-validation.yml`:

- `repository-policy`
- `rust-quality (1)`
- `rust-quality (2)`
- `rust-quality (3)`
- `contracts-and-ipc`
- `windows-desktop (1)`
- `windows-desktop (2)`
- `windows-desktop (3)`
- `windows-supervision`
- `macos-bindings`
- `server-smoke`
- `package-windows (windows-desktop)`
- `package-windows (windows-server)`
- `package-server`
- `validation-complete`

The final `validation-complete` job fails when any dependency is skipped or fails. Repository administrators must not use path filters, optional checks, or bypass rules for release pull requests.

Packaging starts alongside the other validation jobs. Both Windows matrix entries must pass; one failure does not cancel the other. Uploaded unsigned bundles are validation evidence only and cannot be promoted before every gate passes.

`rust-quality` runs all workspace unit and integration tests in three required Nextest count partitions, without retries. Each partition runs on its own Windows host with four test processes. The third partition also checks formatting, compiles and lints every target, runs the native credential lifecycle check, and runs documentation tests with Cargo. Every partition must pass before `validation-complete` succeeds. The full workspace suite owns Rust IPC and engine lifecycle coverage. `windows-supervision` owns the engine diagnostic smoke check and fake/real-engine supervision, alongside the independent `windows-desktop` shell suite. The focused contract and supervision workflows remain available through manual dispatch; they do not repeat these automatic gates.

Windows shell tests run in three disjoint class groups, each with its own WPF dispatcher. The first two groups contain measured slow classes; the third includes every other class, including new tests. Every group must pass. Local publication validation still runs the complete unfiltered shell suite. Rust credential-routing unit fixtures use isolated encrypted storage through the test-only `test-support` feature; a separate required step exercises the real Windows credential-store lifecycle without retries.

The `contracts-and-ipc` job installs the pinned contract generator with lifecycle scripts disabled, then runs `npm audit --audit-level=high`. A high or critical advisory blocks contract generation and release validation. Update the pinned generator, verify generated binding drift, and rerun the full contract job. Do not suppress an advisory without an accepted security decision and a bounded replacement date.

The repository-policy job always compares the candidate with a base revision. Pull requests use the pull request base SHA, pushes use the prior SHA, and manual runs require a base SHA input. The comparison rejects changes to or deletion of an existing server migration. A schema change must use a new, numbered migration path and update `deploy/migrations.sha256`.
