# Orders ownership

This crate owns Order and Work Order lifecycle validation, exact Furniture-line coverage, derived fulfillment state, permitted actions, and the authenticated server confirmation interface. Manufacturing snapshots remain in the server transaction authority. Contracts remain in `eitmad-contracts`; SQLite and PostgreSQL transactions remain in their storage authorities. WPF only projects confirmed state.

See [the Order guide](../../docs/developer/subsystems/orders.md), [the Work Orders guide](../../docs/developer/subsystems/work-orders.md), and [the accepted lifecycle](../../docs/developer/subsystems/manager-receptionist-workflows.md).
