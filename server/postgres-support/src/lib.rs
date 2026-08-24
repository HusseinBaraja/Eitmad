//! Narrow PostgreSQL-only `SQLx` facade.
//!
//! This avoids linking `SQLx`'s optional `SQLite` driver into the desktop
//! workspace, where `rusqlite` owns the native `SQLite` library version.

pub use sqlx_core::{
    error::Error, query::query, query_scalar::query_scalar, raw_sql::raw_sql, row::Row,
    transaction::Transaction,
};
pub use sqlx_postgres::{PgPool, PgPoolOptions, PgRow, Postgres};

pub mod postgres {
    pub use sqlx_postgres::PgPoolOptions;
}
