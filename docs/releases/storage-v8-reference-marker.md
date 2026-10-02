---
title: "Upgrade local storage to version 8 reference marker state"
description: "Explain the retained version-8 schema history after the reference marker demo was removed."
audience: "support"
page_type: "release"
status: "historical"
owner: "Rust storage maintainers"
last_verified: "2026-08-26"
review_triggers:
  - "storage version 8, reference marker schema, compatibility, or rollback behavior changes"
keywords:
  - "storage version 8"
  - "reference-marker.initial.v1"
  - "reference_markers"
  - "eitmad.schema.reference-marker.v1"
---

# Storage version 8: retired reference marker demo

Migration `reference-marker.initial.v1` created `reference_markers` and `reference_marker_sync_outbox`. The demo had scoped labels, audit, idempotency, and publication support.

The beta cleanup removed its crate, contracts, permissions, dispatcher, shell projection, and tests. No product workflow uses the demo. The migration bytes and existing rows remain unchanged. Retired demo publications remain stored and do not enter active event recovery.

Use the [current local storage guide](../developer/subsystems/local-storage.md) for supported versions and checks. Follow [local storage recovery](../operations/recover-local-storage.md) for backup and restoration; do not edit migration history or clear stored rows.
