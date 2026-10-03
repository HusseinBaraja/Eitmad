---
title: "Import and transfer durable catalog images"
description: "Maintain bounded Product and Furniture assets across local restarts and authorized server clients."
audience: "developer"
page_type: "reference"
status: "active"
owner: "catalog image capability"
last_verified: "2026-10-03"
review_triggers:
  - "image contracts, decoder limits, permissions, retention, or server transfer changes"
keywords:
  - "catalog images"
  - "اختيار صورة"
  - "إزالة الصورة"
  - "eitmad.catalog-image.import.v1"
  - "EITMAD_MEDIA_SERVER"
---

# Import and transfer durable catalog images

Product and Furniture revisions can retain one optional immutable image reference. WPF selects a file with the native Windows picker. Rust reads and validates the selected file, strips metadata by re-encoding, stores scoped bytes, and serves authorized chunks. The reference contains an asset ID, capability kind, and SHA-256 digest; it contains no filesystem path.

## Authority and limits

`crates/catalog-image` owns import, normalization, authorization, integrity, and durable upload retries. [Products](products.md) and [Furniture](furniture.md) own attachment revisions. `crates/contracts/src/catalog_image.rs` own typed import and read operations, server transfer DTOs, and bounds. Protocol `1.14` adds optional `eitmad.capability.catalog-image.v1` and `eitmad.schema.catalog-image.v1`. Missing capability withholds image operations.

Rust accepts regular PNG or JPEG files of at most 8 MiB. It rejects malformed or truncated images, sides above 8192 pixels, decoded area above 16 Mi pixels, and decoder allocations above 64 MiB. Imported output is a PNG with each side at most 2048 pixels and at most 8 MiB. Native IPC and server reads return at most 64 KiB of image content per response. Upload requests carry at most 8 MiB of decoded content within a 12 MiB HTTP body limit. The receiving server and client validate the content digest, asset ID, format, and decoded limits independently.

The source path is transient import input. Retry evidence stores its request hash and the resulting reference. It stores neither the path nor image content. Debug output and errors omit paths, decoder messages, image bytes, and base64. Logs must never serialize these request bodies or raw IPC frames.

## Scope, authorization, audit, and history

Local import requires the owning Product or Furniture write and read permissions. Every read requires its owning read permission. Attachment validation rejects wrong capability kinds, missing assets, and references from another scope. The existing local authority uses a tenant-rooted organization scope; the configured server route maps that scope to the installation's registered organization. Credential user and tenant must match the local actor. Server reads and uploads enforce the authenticated tenant and organization relationship before accessing bytes. PostgreSQL also forces tenant row-level security.

Local storage migration `catalog.images.v1` is version `19`; server sync migration `0006_catalog_images.sql` stores organization assets. Image insertion, import retry evidence, pending upload work, and a redacted audit record commit atomically. Server insertion and audit commit atomically. Local cache insertion and upload acknowledgement also require audit. SQLite and PostgreSQL prevent image updates and deletion.

Replacement imports another asset and saves a new definition revision. Removal saves a null image reference. Existing revision references and their bytes remain readable with the required permission. No garbage collection runs: canceled imports can remain as unreferenced retained assets. Existing definitions without images continue to deserialize, including historical JSON. Image import never changes a definition by itself.

## Native workflow and partial failure

The Manager uses **اختيار صورة** in the Product or Furniture editor, then saves the definition. **إزالة الصورة** stages removal. The native shell performs no selected-file reads, database access, authorization, or network transfer. It renders Rust chunks through an in-memory WPF decoder on a worker thread. Late image loads cannot restore a changed editor or an ended session.

Catalog text and prices load before thumbnails. A failed image read leaves those fields available. A server outage does not block local import or definition save. Rust retains upload work across restart and retries in the background. An authenticated exact acknowledgement removes the pending item. Image reads use a local cache first; a cache miss uses the authorized server route and validates bytes before caching them. Cached assets remain readable offline under current local authorization.

Image transfer does not publish or synchronize Product or Furniture definitions. Their existing local definition workflows and private Manager projection remain unchanged. A second authorized client must receive the immutable reference through an authorized catalog definition or consuming workflow; image transfer alone cannot invent that catalog relationship. Receptionist catalog publication remains outside this capability.

## Configure the engine transfer route

Before enabling transfer, register the installation organization and authenticate the engine user/device with the existing Rust server-connection flow. Persist that authenticated session through `eitmad_server_connection::store_session` in native secret storage. The stored credential's principal and tenant must match local authority. This change adds no server account provisioning UI.

| Engine setting | Value |
| --- | --- |
| `EITMAD_MEDIA_SERVER` | Trusted HTTPS server origin |
| `EITMAD_MEDIA_TRUST_PEM` | Local PEM trust anchor path, used only by Rust |
| `EITMAD_MEDIA_CREDENTIAL_ID` | JSON-serialized Rust `SecretId` for the stored authenticated session; never a bearer token |

Without `EITMAD_MEDIA_SERVER`, local images work and uploads remain queued. A configured route requires all three values and native secret storage. Invalid configuration fails engine startup with a redacted component error. Valid configuration with an unavailable server retains work for retry. Credentials refresh through the existing authenticated device flow. The worker attempts one upload at a time, drains acknowledged work, and waits 30 seconds when no upload succeeds. It stops with the engine.

## Verify and extend

Run `cargo test -p eitmad-catalog-image -p eitmad-product -p eitmad-furniture` for synthetic invalid inputs, restart, exact retries, optional attachments, replacement history, scope denial, tampered transfer rejection, and durable outage recovery. Run `CatalogImageRenderedTests` and the affected Product/Furniture shell tests for native rendering and image removal. The available display runs at 125% scaling: captures use 1553.6 × 881.6, 1338.4 × 752.8, and 720 × 560 DIPs. Exact 1920 × 1080 rendering at 100% scaling remains unverified. The existing Windows adapter real-engine scenario imports synthetic assets, removes source files, saves references, and reads them after engine restart.

`crates/server-connection/tests/direct_route.rs` contains the ignored `catalog_image_transfers_between_authorized_clients_and_survives_restart` test. Use a fresh disposable PostgreSQL database and the trusted development certificate variables documented in [the direct desktop connection test](../../operations/run-server-authority.md#run-the-direct-desktop-connection-test). The test transfers a multi-chunk synthetic image over HTTPS between two authenticated devices, restarts the server, checks the second client's durable offline cache, and rejects foreign-organization and ungranted-principal reads. Production data and personal photographs are not test inputs.

Keep future media codecs, retention rules, or published catalog relationships in Rust. Return to the [developer index](../index.md) for focused checks and the [local storage guide](local-storage.md) for recovery.
