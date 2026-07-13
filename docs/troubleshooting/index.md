---
title: "Troubleshoot Eitmad"
description: "Find safe diagnostic paths for stable errors, generated contract drift, and recoverable system failures."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-13"
review_triggers:
  - "a stable diagnosable error or recovery path is added"
keywords:
  - "troubleshooting"
  - "contract drift"
---

# Troubleshoot Eitmad

Use symptom and identifier pages to reach the authoritative owner and a non-destructive recovery path.

## Contract failures

- [Resolve generated contract drift](contract-binding-drift.md): repair missing or stale schemas, registries, C#/Swift bindings, fixtures, and generated references.

## Engine process failures

- [Resolve local IPC connection, request, and subscription failures](local-ipc-failures.md): diagnose unavailable engines, rejected sessions, resync, backpressure, version mismatch, deadlines, and payload bounds.
- [Resolve engine startup and authority failures](engine-startup-failures.md): diagnose failed startup, invalid supervision, readiness checks, shutdown failures, and duplicate engine authorities.
- [Resolve Windows engine supervision failures](windows-engine-supervision-failures.md): diagnose restart exhaustion, forced shutdown, stale observations, and Job Object setup failures.

No product workflow or user-facing Arabic error is implemented yet. Runtime failures expose stable localization message IDs for future shells.

Return to the [documentation index](../index.md) or review [protocol v1](../api/index.md).
