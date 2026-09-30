# Products authority

`eitmad-product` owns organization-scoped ready-made Product definitions, separate categories, fixed supplier variants, purchase costs, validation, lifecycle checks, historical resolution, authorization, and audit orchestration. It does not own selling-price publication or Furniture manufacturing rules.

Contracts remain in `crates/contracts/src/product.rs`. Transactional storage and migration 17 remain in `crates/storage/src/product.rs`. The runtime dispatcher composes this authority for IPC and headless callers. See [the Products guide](../../docs/developer/subsystems/products.md) for policy, failure handling, and focused checks.
