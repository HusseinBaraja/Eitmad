---
title: "Discover untrusted Eitmad servers on a Private LAN"
description: "Use bounded Rust discovery results, preserve installation identity, and keep discovery separate from pairing and access."
audience: "developer"
page_type: "reference"
status: "active"
owner: "Rust server discovery maintainer"
last_verified: "2026-10-20"
review_triggers:
  - "discovery protocol, identity location, network policy, IPC, or firewall rules change"
keywords:
  - "server discovery"
  - "EITMAD-DISCOVERY/1"
  - "Private LocalSubnet"
  - "identity conflict"
---

# Discover untrusted Eitmad servers on a Private LAN

`eitmad-server-discovery` finds connection candidates without server IP configuration. Rust owns the protocol, installation identity, candidate expiry, interface policy, and cancellation. The server host owns responder lifetime; local IPC authorizes scans through its authenticated bootstrap channel. The [Rust pairing backend](server-authority.md#pair-a-discovered-server-and-approve-its-device) authenticates a selected candidate. Native onboarding UI remains separate work under [ADR-0030](../../decisions/0030-offline-guided-windows-setup.md).

## Identity and trust boundary

Each server creates a random UUID in `server-identity` inside its configured update-manifest directory. The installed service already has write permission there. The signed-manifest loader ignores this file because it has no `.json` extension. Exclusive creation and `sync_all` persist the identity. Restarts and binary upgrades retain it. An existing invalid, empty, or interrupted file fails startup without replacement. Preserve it for recovery. Uninstall retains it with all other server files. Do not copy the file to a second active installation.

Each responder also creates a process nonce. Multiple addresses with the same identity and nonce represent one server on multiple interfaces. Different nonces claiming the same identity mark every candidate for that identity as conflicted. This detects accidental copied identities; it does not authenticate a server. A malicious peer can claim any identity or cause a conflict.

Discovery never installs a CA, reads secrets, enrolls a device, changes a saved endpoint, or grants access. Candidates contain only the claimed UUID, observed IPv4 source address, HTTPS port, expiry, and conflict flag. The address comes from the received datagram, not an advertised hostname or URL. There is no trust anchor, account, tenant, or customer payload. Pairing must independently authenticate identity, trust, TLS name, and scope before saving connection settings. Current [TLS and sign-in checks](../../operations/setup-lan-server.md) remain required.

## Eitmad Discovery v1 protocol

This is an Eitmad-specific UDP request/offer protocol, not mDNS or DNS-SD. It uses IPv4 directed broadcast on UDP **58444**. Rust calculates each broadcast address from the current interface address and prefix; no server address is hardcoded. Routers must not forward discovery. The socket TTL is 1. IPv6 and routed discovery are not supported.

Every datagram is at most 512 bytes. It starts with the exact UTF-8 bytes `EITMAD-DISCOVERY/1` followed by LF, then one JSON object. Unknown fields, invalid JSON, oversized packets, and other protocol versions are ignored. Field names below are the wire names defined in `crates/server-discovery/src/wire.rs`:

| Packet | Fields |
| --- | --- |
| Query | `kind: "query"`, `nonce`: fresh random UUID for this scan |
| Offer | `kind: "offer"`, echoed `nonce`, `server_id`: installation UUID, `boot`: responder UUID, `port`: HTTPS port, `lease_seconds`: 1–15 |

A client sends one query per eligible interface, then one retry after two seconds. The server sends unicast offers only in response to queries from an eligible local subnet. Replies originate from UDP 58444. No periodic broadcast advertisement or goodbye is required: a stopped server expires through its lease. A previous scan's nonce cannot populate a new scan.

## Bounds and interface changes

The normal scan deadline is five seconds, including profile reads. A six-second outer bound also cancels blocked socket operations. One scan runs per engine process and one per IPC connection. A scan reads at most 256 datagrams, keeps at most 32 candidate entries, and sends at most two queries on each of at most 16 interfaces. Exceeding limits returns `limitReached` with no partial list.

The Windows adapter reads active IPv4 addresses and network categories through a hidden, read-only PowerShell process. Each read has a two-second deadline and a 16 KiB output limit. Only active interfaces whose profiles are all Private qualify. Public, Domain, unknown, loopback, and overlapping non-Private routes are excluded. The adapter does not change network categories or firewall policy.

Scans recheck the complete eligible interface set once per second. Address, prefix, adapter index, or profile changes return `networkChanged` and discard candidates. Adapter failure returns `unavailable`. The responder refreshes interface policy between receive windows and again before each reply; it closes its socket when policy changes or fails. It processes at most 32 datagrams per receive window, has a 100 ms send deadline, and retries adapter/socket failures after two seconds in the host. Wildcard reception is required for Windows directed broadcasts; Private + LocalSubnet firewall ingress is part of the deployment boundary.

Candidate expiry uses monotonic time internally. IPC returns a wall-clock `expiresAt` for display. A desktop must discard expired candidates and scan again before selection. Cancellation, disconnect, or engine shutdown drops scan sockets and any interface-query child process. No result cache survives a scan or process restart. A scan with no eligible interfaces or no offers completes with an empty list.

## Typed desktop boundary

Local protocol **1.29** advertises `eitmad.capability.server-discovery.v1`. Rust owns `crates/contracts/src/discovery.rs`; native bindings are generated with the other IPC contracts.

Send `eitmad.ipc.discover-servers.v1` with `requestId` and `correlationId` after the local bootstrap handshake. No account sign-in or tenant grant is needed to inspect untrusted LAN candidates. Without the handshake, Rust returns `authenticationRequired` without network I/O. Product queries still require normal ReBAC authorization. Scans mutate no product state and create no product mutation audit.

`eitmad.ipc.server-discovery.v1` returns those IDs, `status`, and `servers`. Status is `complete`, `cancelled`, `networkChanged`, `unavailable`, `busy`, `authenticationRequired`, or `limitReached`. Only `complete` carries candidates. Never choose an arbitrary server when several are returned. Never automatically select an `identityConflict` candidate.

To cancel, send `eitmad.ipc.cancel-server-discovery.v1` with the active scan's same IDs on the same connection. The original request completes with `cancelled`; cancellation has no separate acknowledgment. An unmatched cancellation does nothing. The future onboarding adapter must route the generated result and cancellation messages; this change adds no screen or automatic connection.

## Installation and focused verification

New Windows service installations bind HTTPS on IPv4 wildcard so DHCP and interface changes do not require a new listener address. The saved endpoint remains the TLS readiness/diagnostic endpoint. Existing retained service configuration is not rewritten. A certificate must still cover the authenticated TLS name; discovery cannot solve a certificate-name mismatch. [Private LAN firewall setup](../../operations/run-windows-host-services.md#connect-the-second-pc-yourself) permits only the installed server executable, the configured TCP HTTPS port, and UDP 58444. Both rules use Private + LocalSubnet and block edge traversal. PostgreSQL stays on loopback. Switching to exact-client diagnostic access removes the discovery rule; uninstall removes both owned rules.

Run `cargo test --locked -p eitmad-server-discovery` for isolated identity persistence, copied identities, expiry, malformed offers, interface changes, cancellation, and a real loopback UDP exchange with port cleanup. Run `cargo test --locked -p eitmad-engine-runtime local_ipc` for local bootstrap and asynchronous IPC behavior. Run `powershell -NoProfile -File deploy/windows/test-host.ps1` for mocked firewall scope, ownership, preview, and uninstall. Run `powershell -NoProfile -File deploy/windows/test-discovery-interfaces.ps1` for synthetic Windows profile classification. These checks do not change the live installation or prove effective LAN firewall policy.

Acceptance on two isolated Windows PCs remains required: discover several synthetic servers on Private, change DHCP addresses and profiles, stop a server, cancel a scan, and verify no discovery/HTTPS access through owned rules on Public or Domain. Preserve identity and data throughout. If no server appears, check Private category, UDP 58444 filtering, matching subnet, server listener, and Wi-Fi client isolation. Do not open Public access or bypass TLS.

Next, use the [developer guide](../index.md) for checks and the [Windows host guide](../../operations/run-windows-host-services.md) for controlled installation.
