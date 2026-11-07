use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use rusqlite::Connection;

use crate::{StorageError, configure_connection};

// Bound retained caches, without making concurrent readers wait behind a writer.
const MAX_IDLE_CONNECTIONS: usize = 4;

#[derive(Clone, Debug, Default)]
pub(super) struct Connections(Arc<Mutex<Vec<Connection>>>);

impl Connections {
    pub(super) fn acquire(&self, path: &std::path::Path) -> Result<Lease, StorageError> {
        let cached = self.0.lock().map_err(|_| StorageError)?.pop();
        let connection = if let Some(connection) = cached {
            connection
        } else {
            let connection = Connection::open(path).map_err(|_| StorageError)?;
            configure_connection(&connection)?;
            connection
        };
        Ok(Lease {
            connection: Some(connection),
            owner: self.clone(),
        })
    }
}

pub(super) struct Lease {
    connection: Option<Connection>,
    owner: Connections,
}

impl Deref for Lease {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        self.connection.as_ref().expect("leased connection")
    }
}

impl DerefMut for Lease {
    fn deref_mut(&mut self) -> &mut Connection {
        self.connection.as_mut().expect("leased connection")
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            // Never lend an unfinished transaction to another operation.
            if connection.is_autocommit() {
                if let Ok(mut idle) = self.owner.0.lock() {
                    if idle.len() < MAX_IDLE_CONNECTIONS {
                        idle.push(connection);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::AuthorityStore;

    #[test]
    fn unfinished_transaction_is_not_reused_and_concurrent_leases_do_not_block() {
        let directory = tempfile::tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let connection = store.open_connection().unwrap();
        let other = store.open_connection().unwrap();
        connection
            .execute_batch("BEGIN IMMEDIATE; CREATE TABLE uncommitted (id INTEGER)")
            .unwrap();
        drop(connection);
        assert_eq!(
            other
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name='uncommitted'",
                    [],
                    |row| row.get::<_, u32>(0)
                )
                .unwrap(),
            0
        );
        drop(other);
        assert!(store.open_connection().unwrap().is_autocommit());
    }
}
