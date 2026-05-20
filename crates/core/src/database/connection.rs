//! SQLite connection management.
//!
//! Wraps a `rusqlite::Connection` in a `Mutex` for safe concurrent access.
//! Enables WAL mode for improved concurrent read performance.

use crate::errors::NodaError;
use crate::database::schema;
use crate::database::migrations;
use parking_lot::{Mutex, MutexGuard};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use tracing::instrument;

/// Thread-safe SQLite database handle.
///
/// The inner `Connection` is protected by a `Mutex`. All callers acquire the
/// lock via `db.lock()` and release it by dropping the guard.
#[derive(Clone)]
pub struct Database {
    inner: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open the database at `path`, enabling WAL mode and running migrations.
    ///
    /// Creates the file if it does not exist.
    #[instrument(skip(path), fields(path = %path.as_ref().display()))]
    pub fn open(path: impl AsRef<Path>) -> Result<Self, NodaError> {
        let path = path.as_ref();
        let conn = Connection::open(path)?;

        // Enable WAL mode for concurrent read access.
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        // Enforce foreign key constraints.
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        // Reasonable cache size (8 MiB).
        conn.execute_batch("PRAGMA cache_size=-8000;")?;

        // Create or update schema.
        schema::create_tables(&conn)?;

        // Run any pending migrations.
        migrations::run(&conn)?;

        tracing::info!(path = %path.display(), "database opened");

        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
        })
    }

    /// Acquire the inner connection lock.
    ///
    /// The returned guard automatically releases the lock on drop.
    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        self.inner.lock()
    }
}
