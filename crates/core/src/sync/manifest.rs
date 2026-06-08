use std::collections::HashMap;
use std::path::Path;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rusqlite::Connection;
use crate::errors::NodaError;

/// Flat representation of the vault's active files and their content hashes at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultManifest {
    pub generated_at: DateTime<Utc>,
    pub files: HashMap<String, String>, // path relative to vault root -> XXH3 hash (hex)
}

/// Generates a `VaultManifest` from the current active files tracked in the database
/// that physically exist in the vault folder.
pub fn generate_manifest(
    conn: &Connection,
    vault_path: &Path,
) -> Result<VaultManifest, NodaError> {
    let mut stmt = conn
        .prepare("SELECT path, hash FROM sync_file_states WHERE hash != ''")
        .map_err(|e| NodaError::Database(format!("Prepare generate_manifest failed: {}", e)))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| NodaError::Database(format!("Query generate_manifest failed: {}", e)))?;

    let mut files = HashMap::new();
    for row in rows {
        let (path, hash) = row.map_err(|e| {
            NodaError::Database(format!("Row parsing failed in generate_manifest: {}", e))
        })?;
        
        // Ensure the file physically exists on disk.
        // Files that are soft-deleted or removed won't be included in the active manifest tree.
        let full_path = vault_path.join(&path);
        if full_path.exists() {
            files.insert(path, hash);
        }
    }

    Ok(VaultManifest {
        generated_at: Utc::now(),
        files,
    })
}
