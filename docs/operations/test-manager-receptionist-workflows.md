---
title: "Test the Manager and Receptionist workflows"
description: "Run the real mixed-sales journey, interruption scenarios, and retained-state checks, with explicit readiness blockers."
audience: "developer"
page_type: "task"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-10-16"
review_triggers:
  - "customer lifecycle, default validity, account activation, or workflow integrations change"
keywords:
  - "E2E readiness"
  - "Manager Receptionist"
  - "duplicate submission"
  - "session revocation"
  - "جاهز للتسليم"
---

# Test the Manager and Receptionist workflows

**Full accepted-workflow test readiness: blocked.** Use this runbook to reproduce the implemented sales journey and focused integration evidence. It does not approve a production release. The [accepted specification](../developer/subsystems/manager-receptionist-workflows.md#acceptance-scenarios) remains the acceptance authority; this run does not prove all 28 scenarios.

## Setup

Follow the prerequisites and credential handling in [Run the test environment](run-workflow-test-environment.md). Use synthetic data and one Windows user. Close this environment's client before a rebuild. Use a new state directory and unused ports for a separate run:

```powershell
$state = "$PWD/tests/engine-integration/artifacts/workflow-readiness-20261016-run"
pwsh -File tools/workflow-environment.ps1 Start -StateDirectory $state -PostgresBin "$PWD/target/direct-order-test/pgsql/bin" -DatabasePort 55449 -ServerPort 58449 -VerifySignIn
```

The example PostgreSQL path requires the pinned development binaries already installed by `tools/dev-database.ps1`. The launcher builds current Rust and WPF source. `-SkipBuild` is suitable only when those product binaries match current source; the WPF verification still builds its tests. Read the generated passwords locally as described in the environment guide. Never copy secrets or customer records into evidence.

Rust provisions catalog definitions and publishes them through authenticated operations. The smoke uses real WPF account controls, a supervised Rust process, typed IPC, TLS, native credentials, SQLite, and PostgreSQL with `NOSUPERUSER NOBYPASSRLS`. There are no substituted integration mocks. Protocol negotiation, ReBAC, scope checks, revision checks, and audit remain active.

## Smoke journey

`-VerifySignIn` runs `WorkflowEnvironmentTests.OneDesktopSwitchesAccountsAndRetainsServerConfirmedWork`, including this journey. Commercial actions use the real bridge from the WPF host; this is not a manual click-through of every editor.

| Role and action | Expected result |
| --- | --- |
| Manager configures the seeded catalog and publishes Furniture at 12,500 YER and a ready-made Product at 2,000 YER | Published revisions are server-confirmed. A below-cost warning requires explicit confirmation. Receptionist receives public data without costs. |
| Receptionist creates an Arabic customer and waits for confirmation | Rust retains the customer UUID and revision; queued state becomes server-confirmed. |
| Receptionist creates one Furniture line and one Product line, each quantity 1, with 6% discount | Subtotal 14,500 YER; discount 870 YER; total 13,630 YER. Approval is Pending. |
| Manager approves the exact request | Server confirms the request ID, revisions, and commercial fingerprint. |
| Receptionist issues, records in-person acceptance, and converts | Official quotation and order identities exist. Exactly one Work Order contains only the Furniture line. The same conversion key returns the same Order. |
| Manager assigns the workshop and a future due date, starts and completes manufacturing | Order derives Confirmed → In Production → Ready from its Work Order. |
| Receptionist reads readiness and opens the order and source-quotation documents | Delivery is permitted; manufacturing start is absent. Both confirmed documents have two lines and total 13,630 YER. Native preview renders pages, reauthorizes printing through Rust, and closes through Back. |
| Receptionist records written delivery and repeats the exact request | One Delivery; state Delivered; repeated key returns the same Delivery ID. |
| Both roles switch accounts for three rounds | One engine generation; account controls clear; signed-out reads fail; saved drafts and accepted sources remain unchanged. |

No physical print job, baseline-size visual review, keyboard/accessibility review, or full manual editor journey is claimed by this automation.

The extended database audit also drives the real field bindings and save handlers listed in [the environment guide](run-workflow-test-environment.md). Its private evidence is under `tests/engine-integration/artifacts/database-audit-20261016/`. It compares prior form receipts after restart, checks SQLite integrity and foreign keys directly, and captures entry screens, saved tables, the shared quotation on the other role, and native document previews. Captures record 125% Windows scaling; they do not establish the required 100% visual baseline. Department reports and unimplemented customer lifecycle actions have no testable save path.

## Focused integration scenarios

Run one Cargo command at a time against the stable toolchain cache. Each scenario requires a **fresh** database; do not reuse a populated database or remove `--ignored`. The filter must select one test (the summary must report `1 passed`, not zero).

```powershell
$env:CARGO_TARGET_DIR = "$PWD/target/toolchain-1.85.1"
$env:CARGO_INCREMENTAL = '0'
$toolCargo = rustup which --toolchain 1.85.1 cargo
$env:PATH = "$(Split-Path $toolCargo);$env:PATH"
. ./tools/dev-database.ps1 -Action NewDatabase
$scenario = 'discount_approval_cross_client_live_replay_invalidation_and_rejection'
cargo test --locked -p eitmad-server-connection --test direct_route $scenario -- --ignored --nocapture
```

| Scenario filter | Required proof | Evidence from this run |
| --- | --- | --- |
| `catalog_reaches_separate_receptionist_and_recovers_without_private_fields` | Interrupted transfer resumes across isolated clients; public projection excludes private fields | `catalog_reaches_separate.log`: passed |
| `two_isolated_customer_engines_recover_and_preserve_conflicts` | Offline customer work recovers; concurrent revisions remain explicit conflicts | `two_isolated_customer_engines.log`: passed |
| `quotation_drafts_restart_transfer_replay_and_conflict_through_real_server` | Draft restart, transfer, replay, and concurrent edit | `quotation_drafts_restart_transfer.log`: passed |
| `discount_approval_cross_client_live_replay_invalidation_and_rejection` | Real approval streams, replay, invalidation, rejection, lifecycle races | `discount_approval.log`: passed |
| `orders_cross_client_conversion_transitions_and_restart` | Mixed/ready-made conversion, duplicate intents, work transitions, delivery denial/replay, restart | `orders_cross_client.log`: passed |
| `real_server_authentication_tls_sync_and_reconnect` | TLS/authentication rejection and actual server interruption/reconnect | `real_server_authentication_tls_sync_and_reconnect.log`: passed |
| `revoked_session_cannot_submit_on_an_established_websocket` | Revocation denies an already connected writer | `revoked_session_cannot_submit.log`: passed |
| `manual_two_client_recovery_retains_pending_work_and_accepted_receipts` | Real backup/restore of two SQLite clients and PostgreSQL; pending work confirms; unknown-reply receipt replay creates no duplicate Order effects | `manual_two_client_recovery.log`: passed; recovery report verifies rowsets and zero duplicate effects |

For the recovery scenario, set these additional variables before its Cargo command. Keep the generated artifact directory private. It restores into a new database and preserves the original:

```powershell
$env:EITMAD_RECOVERY_ARTIFACT_DIRECTORY = "$state/recovery-$([Guid]::NewGuid().ToString('N'))"
$env:EITMAD_RECOVERY_PG_BIN = "$PWD/target/direct-order-test/pgsql/bin"
$env:EITMAD_RECOVERY_ADMIN_PASSWORD = (Get-Content target/direct-order-test/credentials.json -Raw | ConvertFrom-Json).Admin
```

The unknown-reply scenario deliberately discards an actual server response before recovery; it does not emulate a server receipt. It does not inject TCP loss at every conversion/delivery commit point. Exact delivery replay is covered separately by the Order integration and smoke.

## Restart and reset

```powershell
pwsh -File tools/workflow-environment.ps1 Stop -StateDirectory $state
pwsh -File tools/workflow-environment.ps1 Start -StateDirectory $state -SkipBuild -VerifySignIn
```

Stop preserves databases, outboxes, audits, and proof files. On another run, `sales-smoke-proof.json` must resolve to the same Delivered Order, official number, and total before new work starts; `accepted-work-proof.json` preserves accepted sources. A failed run must retain state for diagnosis. For a clean reset, stop this environment and start a new named directory with unused ports. Do not delete customer databases, reset business rows, or reseed accepted work. Temporary live integration credentials now have exact-ID cleanup; retained workflow credentials remain available for restart.

## Evidence and remaining blockers

Private local evidence is under `tests/engine-integration/artifacts/workflow-readiness-20261016-run/`; it is intentionally ignored by Git. `wpf-smoke.log` records a complete initial pass (2 m 4 s). `wpf-diagnostics.log` records the latest complete restart pass (3 m 37 s), including retained delivery, both native previews, and three rounds of account switching. The stage and proof files identify the last completed commercial journey. The integration target also passes Rust 1.85.1 formatting and Clippy with `-D warnings`. The full documentation audit passes (103 pages).

`wpf-restart-current.log` completed the retained-order check, mixed journey, and both native document previews, then failed during a later Manager activation: the engine remained `Running` with IPC `Connecting`, and the Manager surface was hidden. This repeated an earlier activation failure. The later pass does not establish its cause or fix it. Temporary diagnostics on the passing run captured only an end-of-stream at shutdown; no causal fix is claimed. Keep intermittent activation as a readiness blocker.

The database audit confirmed and fixed a separate transport defect: cancelling a write under named-pipe backpressure could truncate its frame and block the next request. The adapter regression failed before the fix and passed afterward; see [IPC cancellation semantics](../developer/subsystems/windows-process-supervision.md#restart-and-stale-event-invariants). The earlier activation log did not identify its transport cause, so do not assume this fix resolves every intermittent activation failure. Recheck that blocker with connection-loss diagnostics if it recurs.

Test repairs in this run correct actor/device pairing, use independent credential routes for actual concurrent server requests, establish approval streams before decisions, compose new drafts with current price revisions, and remove exact native credentials created by disposable integrations. Commercial assertions remain active. The smoke retries unavailable or unknown outcomes only with the same intent and key; Rust checks the retained pricing receipt before resubmission. Retained workflow credentials and databases are preserved.

| Blocker | Exact missing behavior and authority |
| --- | --- |
| Desktop account activation | Repeated account switching can lose IPC after successful sign-in and hide the account surface. Retained business records do not prove stable session activation. |
| Customer lifecycle | Manager merge, archive/restore, and extra branch association have no implemented command path. Accepted scenario 4 cannot execute; see [customer implementation boundary](../developer/subsystems/customers.md). |
| Organization-wide customer access | Customer search is bound to one local branch. The all-branch Manager customer journey in scenario 2 is unavailable; server branch authorization alone does not provide that desktop query. |
| Organization default validity | New drafts use 30 days. `SetQuotationValidity` changes an individual draft; no organization-setting mutation supports 30 → 45 for future drafts. Accepted scenario 17 cannot execute. |

Recheck these blockers after implementation, then execute the remaining accepted scenarios before declaring full role test readiness. Production approval additionally requires the separate [release-candidate gates](validate-release-candidate.md); those gates were not run or waived here.

Return to [operations](index.md) or [environment setup](run-workflow-test-environment.md) for the next run.
