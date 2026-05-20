//! Domain model for Vault handle

use std::path::PathBuf;

/// Vault handle holding configuration and root path
#[derive(Debug, Clone)]
pub struct Vault {
    pub path: PathBuf,
    pub name: String,
}

impl Vault {
    pub fn new(path: PathBuf) -> Self {
        let name = path.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unknown Vault".to_string());
            
        Self { path, name }
    }
}
