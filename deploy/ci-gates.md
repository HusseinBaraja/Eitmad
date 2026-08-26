# Mandatory CI status checks

**Owner:** Release engineering and security maintainers.

Branch protection for `main` must require every job from `.github/workflows/mandatory-validation.yml`:

- `repository-policy`
- `rust-quality`
- `contracts-and-ipc`
- `windows-desktop`
- `macos-bindings`
- `server-smoke`
- `package-windows`
- `package-server`
- `validation-complete`

The final `validation-complete` job fails when any dependency is skipped or fails. Repository administrators must not use path filters, optional checks, or bypass rules for release pull requests.
