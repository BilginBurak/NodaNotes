pub mod init;
pub mod persistence;
pub mod scan;
pub mod service;

pub use init::*;
pub use persistence::*;
pub use scan::*;
pub use service::*;

use crate::errors::NodaError;
use crate::models::note::Note;
use crate::models::vault::Vault;
use std::path::Path;

/// Initializes a new vault at the given path
pub async fn create_vault<P: AsRef<Path>>(path: P) -> Result<Vault, NodaError> {
    let path = path.as_ref();
    if !path.exists() {
        tokio::fs::create_dir_all(path).await.map_err(NodaError::Io)?;
    }

    init::create_noda_dir(path).await?;
    init::create_manifest(path).await?;
    init::create_sync_files(path).await?;

    // Persist path so app remembers next time
    persistence::save_last_vault_path(path.to_path_buf()).await?;

    Ok(Vault::new(path.to_path_buf()))
}

/// Opens an existing vault, validates it, and returns the handle alongside its notes
pub async fn open_vault<P: AsRef<Path>>(path: P) -> Result<(Vault, Vec<Note>), NodaError> {
    let path = path.as_ref();
    
    // Validate or repair structure
    validate_vault(path).await?;
    
    // Scan for existing notes
    let notes = scan::scan_vault(path).await?;
    
    // Persist path so app remembers next time
    persistence::save_last_vault_path(path.to_path_buf()).await?;
    
    Ok((Vault::new(path.to_path_buf()), notes))
}

/// Validates and repairs the vault structure
pub async fn validate_vault<P: AsRef<Path>>(path: P) -> Result<(), NodaError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(NodaError::Vault("Vault path does not exist".to_string()));
    }

    // Repair mode: re-create missing directories/files
    init::create_noda_dir(path).await?;
    init::create_manifest(path).await?;
    init::create_sync_files(path).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_and_validate_vault() {
        let dir = tempdir().unwrap();
        
        let vault = create_vault(dir.path()).await.expect("Failed to create vault");
        assert!(vault.path.join(".noda").exists());
        assert!(vault.path.join(".noda").join("manifest.json").exists());
        assert!(vault.path.join(".noda").join("sync").join("queue.json").exists());

        // Should not fail on validation (which also repairs)
        validate_vault(dir.path()).await.expect("Failed to validate vault");
    }
}
