//! Vault path persistence

use crate::errors::NodaError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Serialize, Deserialize, Default)]
struct AppSettings {
    last_vault_path: Option<PathBuf>,
}

fn settings_path() -> Result<PathBuf, NodaError> {
    let mut path = dirs::config_dir()
        .ok_or_else(|| NodaError::Vault("Could not find config directory".to_string()))?;
    path.push("noda");
    if !path.exists() {
        std::fs::create_dir_all(&path).map_err(NodaError::Io)?;
    }
    path.push("settings.json");
    Ok(path)
}

/// Saves the last opened vault path to global app settings
pub async fn save_last_vault_path(path: PathBuf) -> Result<(), NodaError> {
    let file_path = settings_path()?;
    let settings = AppSettings {
        last_vault_path: Some(path),
    };
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| NodaError::Vault(format!("Failed to serialize settings: {}", e)))?;
    fs::write(file_path, content).await.map_err(NodaError::Io)?;
    Ok(())
}

/// Loads the last opened vault path from global app settings
pub async fn load_last_vault_path() -> Result<Option<PathBuf>, NodaError> {
    let file_path = settings_path()?;
    if !file_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&file_path).await.map_err(NodaError::Io)?;
    let settings: AppSettings = serde_json::from_str(&content)
        .map_err(|e| NodaError::Vault(format!("Failed to parse settings: {}", e)))?;
    Ok(settings.last_vault_path)
}
