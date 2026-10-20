---
title: "ADR-0030: Provide offline guided Windows setup"
description: "Define one offline Server or Client installer, first-owner setup, independent devices, and authenticated LAN discovery."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "product developer"
last_verified: "2026-10-20"
review_triggers:
  - "installer roles, offline prerequisites, discovery trust, account creation, or recovery changes"
keywords:
  - "offline installer"
  - "server discovery"
  - "first owner"
  - "device pairing"
---

# ADR-0030: Provide offline guided Windows setup

> - Status: Accepted
> - Date: 2026-10-20
> - Decision owners: Product developer
> - Supersedes: None
> - Superseded by: None

## Context

Workshop PCs can lack internet access. Operators need a native Arabic setup experience without database commands, IP entry, SDK installation, or copied device identities. The product developer selected a single-workshop LAN with a required server connection, a server PC that can also run the desktop, and independent staff accounts.

## Decision

Distribute one offline ZIP with a native `EitmadSetup.exe` and reviewed runtime payloads. Setup offers Server and Client. Server installs a dedicated PostgreSQL cluster and Rust service, plus the same desktop app used on clients. Services start independently of desktop windows and Windows sign-in. Server installation requests administrator rights. Client installation is per Windows user and does not install server components or require administrator rights. Ship compiled Rust and self-contained .NET; end users do not install development toolchains.

Rust owns setup contracts, validation, configuration, credentials, database preparation, account initialization, authorization, and audit. The Windows adapter owns elevation, package registration, file permissions, process invocation, services, firewall APIs, and shortcuts. The native shell renders verified progress and failures. An interrupted setup preserves state and reports the failed stage. It does not reset a database or silently replace an existing installation.

The owner enters a workshop name, username, and password once. Rust creates the first active Owner with Manager duties and a first branch atomically. Only Owner authority manages staff accounts through the existing Users workflow. Manager and Receptionist remain operational roles. Windows administrator rights do not grant application Owner authority. Personal and shared PCs use individual application sign-ins; switching users closes the previous application session.

Clients discover available LAN servers automatically. Discovery data is untrusted and cannot authorize a connection, install a CA, or supply an identity. A short-lived, single-use code shown by an authorized Owner establishes server trust through an authenticated pairing exchange. Bind that exchange to the server identity, trust anchor, installation scope, and the new client's public device key. Rate-limit attempts and refuse expired, replayed, or changed exchanges. Do not validate TLS against a CA obtained only from an unauthenticated discovery response. Never disable certificate validation.

Each client creates and retains its own device key. Store only the authenticated endpoint, public trust, and scope settings needed for reconnect. Discover address changes by retained server identity, then verify TLS and current authorization. Discovery does not replace password sign-in or the [live-authority rule](0029-require-live-server-authority-for-protected-access.md).

PostgreSQL remains loopback-only. Native firewall changes must be bounded to the private workshop network and intended clients; discovery must not create permanent broad public access. Keep the server awake as an explicit host requirement. Setup must verify readiness over HTTPS with certificate name, chain, and expiry checks before it reports completion.

Provide automatic local backups and an optional external destination. Clearly state that a backup on the same disk does not protect against disk loss. Preserve credentials, CA renewal material, database state, and device-specific recovery needs. Prove restore before calling the backup feature complete.

## Consequences and alternatives

The target PCs need no internet to install or use a complete supported bundle. Release preparation still needs trusted dependency acquisition, runtime security updates, publisher signing, and clean-machine evidence. Keep the current MSIX identity and native Windows behavior where they meet per-user installation requirements. Do not introduce a portable copy path that bypasses existing publisher or upgrade checks.

Manual connection entry remains a diagnostic path while discovery is built. It is not the target onboarding experience. The current implementation status and operator commands belong in [Windows installation](../operations/package-windows-desktop.md) and [server operation](../operations/run-server-authority.md), not in this decision. An accepted design does not claim that every selected feature is implemented.

## Verification

Require fresh isolated setup, refusal of existing state, rollback of failed identity or audit writes, separate client installation without server payload installation, authenticated pairing, two independent devices, validated HTTPS readiness, authorized cross-client work, and reconnect after application and server restart. Native rendering uses the baseline sizes in `AGENTS.md`. Production acceptance also requires a clean offline Windows host, boot without sign-in, backup restore, and trusted signed artifacts.

Next, use the [current Windows setup procedures](../operations/package-windows-desktop.md) and the [decision index](index.md).
