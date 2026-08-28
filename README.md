# الاعتماد | Eitmad

Arabic-first operations system for الاعتماد furniture manufacturing.

This repository is a cross-platform native desktop workspace built around a separate, authoritative Rust engine. Native shells stay thin; Rust owns contracts, product behavior, storage, synchronization, authorization, update policy, observability, audit, and external boundaries.

Start with:

- [Repository policy](AGENTS.md)
- [Target architecture](docs/architecture/target-architecture.md)
- [Repository layout and ownership](docs/developer/repository-layout.md)
- [Documentation index](docs/index.md)

## Run the admin dashboard on Windows

Install the .NET 8 SDK and the stable Rust toolchain. From the repository root, build the Rust engine:

```powershell
cargo build -p eitmad-engine-cli
```

Then start the native Windows dashboard with that engine:

```powershell
dotnet run --project shells/windows/Eitmad.WindowsShell.csproj -- --engine target/debug/eitmad-engine-cli.exe
```

The **لوحة التحكم** window opens in **وضع المعاينة**. Closing the window keeps the app in the system tray. Use **إنهاء الاعتماد** in the tray menu to stop the shell and engine. For design and ownership details, see the [Windows operations shell guide](docs/developer/subsystems/windows-native-shell.md).

Foundation verification:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -q -p eitmad-engine-cli -- diagnose
npm ci --ignore-scripts --prefix crates/contracts/codegen
npm run contracts:check --prefix crates/contracts/codegen
dotnet run --project tests/contract-compatibility/csharp/Eitmad.ContractConformance.csproj -- tests/contract-compatibility/fixtures/protocol-v1.json
dotnet run --project platform-adapters/windows/tests/Eitmad.Platform.Windows.Tests.csproj -- --engine target/debug/eitmad-engine-cli.exe
python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs
```

Production features must be organized by bounded product capability. Do not add generic `utils`, `common`, `shared`, `handlers`, or `services` buckets.
