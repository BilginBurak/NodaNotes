//! # Database Module
//!
//! SQLite cache and FTS5 search index.
//!
//! SQLite is NEVER primary storage — the `.md` files are.
//! The database is fully rebuildable from the vault at any time.

pub mod connection;
pub mod migrations;
pub mod queries;
pub mod schema;

use crate::errors::NodaError;
use crate::models::Vault;
use crate::vault::scan::scan_vault;
use connection::Database;
use tracing::instrument;

/// Open (or create) the database for the given vault.
///
/// Runs all pending migrations and verifies FTS5 is available.
#[instrument(skip(vault), fields(vault = %vault.root.display()))]
pub async fn open(vault: &Vault) -> Result<Database, NodaError> {
    let db = Database::open(vault.db_path())?;
    Ok(db)
}

/// Rebuild the entire database from scratch by re-scanning all vault `.md` files.
///
/// Drops all tables, recreates them, and re-populates with fresh data.
/// Called when the database is missing, corrupt, or explicitly requested.
#[instrument(skip(vault, db), fields(vault = %vault.root.display()))]
pub async fn rebuild(vault: &Vault, db: &Database) -> Result<(), NodaError> {
    tracing::warn!("rebuilding database from vault filesystem");

    {
        let conn = db.lock();
        schema::drop_and_recreate(&conn)?;
    }

    let notes = scan_vault(&vault.root).await?;
    let vault_root = &vault.root;

    {
        let conn = db.lock();
        for note in &notes {
            // Compute relative path from vault root.
            let relative_path = note
                .file_path
                .strip_prefix(vault_root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| note.relative_path.clone());

            queries::upsert_note(&conn, note, &relative_path)?;
        }
    }

    tracing::info!(count = notes.len(), "database rebuild complete");
    Ok(())
}

pub use connection::Database;
