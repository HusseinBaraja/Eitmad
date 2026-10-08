# Orders ownership

This crate owns order lifecycle validation, derived fulfillment state, permitted actions, and the authenticated server confirmation interface. Contracts remain in `eitmad-contracts`; SQLite and PostgreSQL transactions remain in their storage authorities. WPF only projects confirmed state.

See [the order capability guide](../../docs/developer/subsystems/orders.md) and [the accepted lifecycle](../../docs/developer/subsystems/manager-receptionist-workflows.md).
