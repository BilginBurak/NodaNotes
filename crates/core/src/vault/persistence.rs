//! Vault path persistence

use crate::errors::NodaError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Serialize, Deserialize, Default)]
struct AppSettings {
    last_vault_path: Option<PathBuf>,
    daemon_token: Option<String>,
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

fn load_settings_sync() -> Result<AppSettings, NodaError> {
    let file_path = settings_path()?;
    if !file_path.exists() {
        return Ok(AppSettings::default());
    }
    let content = std::fs::read_to_string(&file_path).map_err(NodaError::Io)?;
    let settings: AppSettings = serde_json::from_str(&content)
        .map_err(|e| NodaError::Vault(format!("Failed to parse settings: {}", e)))?;
    Ok(settings)
}

/// Saves the last opened vault path to global app settings
pub async fn save_last_vault_path(path: PathBuf) -> Result<(), NodaError> {
    let file_path = settings_path()?;
    let mut settings = load_settings_sync().unwrap_or_default();
    settings.last_vault_path = Some(path);
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| NodaError::Vault(format!("Failed to serialize settings: {}", e)))?;
    fs::write(file_path, content).await.map_err(NodaError::Io)?;
    Ok(())
}

/// Loads the last opened vault path from global app settings
pub async fn load_last_vault_path() -> Result<Option<PathBuf>, NodaError> {
    let settings = load_settings_sync()?;
    Ok(settings.last_vault_path)
}

/// Gets the existing daemon token from global settings, or generates a new one and saves it synchronously.
pub fn get_or_create_daemon_token_sync() -> Result<String, NodaError> {
    let file_path = settings_path()?;
    let mut settings = load_settings_sync().unwrap_or_default();

    if let Some(token) = &settings.daemon_token {
        Ok(token.clone())
    } else {
        let token = uuid::Uuid::new_v4().simple().to_string();
        settings.daemon_token = Some(token.clone());
        let content = serde_json::to_string_pretty(&settings)
            .map_err(|e| NodaError::Vault(format!("Failed to serialize settings: {}", e)))?;
        std::fs::write(file_path, content).map_err(NodaError::Io)?;
        Ok(token)
    }
}
