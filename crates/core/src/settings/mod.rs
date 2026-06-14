use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::errors::NodaError;
use crate::sync::engine::SyncConfig;
use crate::sync::client::WebDavClient;

fn default_theme() -> String { "dark".to_string() }
fn default_accent_color() -> String { "blue".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    #[serde(default = "default_theme")]
    pub theme: String, // "light", "dark", "auto"
    #[serde(default = "default_accent_color")]
    pub accent_color: String,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            accent_color: default_accent_color(),
        }
    }
}

fn default_font_size() -> u32 { 14 }
fn default_typography() -> String { "sans".to_string() }
fn default_show_word_count() -> bool { true }
fn default_auto_save_delay_ms() -> u32 { 1500 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    #[serde(default = "default_typography")]
    pub typography: String, // "sans", "serif", "mono"
    #[serde(default = "default_show_word_count")]
    pub show_word_count: bool,
    #[serde(default = "default_auto_save_delay_ms")]
    pub auto_save_delay_ms: u32,
    #[serde(default)]
    pub default_daily_template: Option<String>,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            typography: default_typography(),
            show_word_count: default_show_word_count(),
            auto_save_delay_ms: default_auto_save_delay_ms(),
            default_daily_template: None,
        }
    }
}

fn default_retention_days() -> u32 { 30 }
fn default_max_snapshots_per_note() -> u32 { 50 }
fn default_empty_trash_after_days() -> u32 { 30 }
fn default_snapshot_interval_mins() -> u32 { 5 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySettings {
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    #[serde(default = "default_max_snapshots_per_note")]
    pub max_snapshots_per_note: u32,
    #[serde(default = "default_empty_trash_after_days")]
    pub empty_trash_after_days: u32,
    #[serde(default = "default_snapshot_interval_mins")]
    pub snapshot_interval_mins: u32,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            retention_days: default_retention_days(),
            max_snapshots_per_note: default_max_snapshots_per_note(),
            empty_trash_after_days: default_empty_trash_after_days(),
            snapshot_interval_mins: default_snapshot_interval_mins(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub appearance: AppearanceSettings,
    #[serde(default)]
    pub editor: EditorSettings,
    #[serde(default)]
    pub sync: SyncConfig,
    #[serde(default)]
    pub history: HistorySettings,
}

impl AppConfig {
    pub async fn load<P: AsRef<Path>>(vault_path: P) -> Result<Self, NodaError> {
        let config_path = vault_path.as_ref().join(".noda/settings.json");
        
        // Ensure sync config is synced from older files if it exists but settings.json doesn't
        let mut config = if !config_path.exists() {
            let old_sync = SyncConfig::load(vault_path.as_ref()).await.unwrap_or_default();
            Self {
                sync: old_sync,
                ..Default::default()
            }
        } else {
            let content = tokio::fs::read_to_string(&config_path)
                .await
                .map_err(NodaError::Io)?;
                
            serde_json::from_str(&content)
                .map_err(|e| NodaError::Sync(format!("Failed to parse settings: {}", e)))?
        };

        if config.sync.device_name.is_empty() {
            config.sync.device_name = SyncConfig::generate_random_device_name();
            // Save immediately so it's persisted on disk
            let _ = config.save(vault_path).await;
        }
            
        Ok(config)
    }

    pub async fn save<P: AsRef<Path>>(&self, vault_path: P) -> Result<(), NodaError> {
        let config_path = vault_path.as_ref().join(".noda/settings.json");

        // Load the existing settings directly from disk without calling AppConfig::load to avoid recursion
        let old_config: Option<AppConfig> = if config_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
                serde_json::from_str(&content).ok()
            } else {
                None
            }
        } else {
            None
        };

        if let Some(old_config) = old_config {
            if !self.sync.webdav_url.is_empty() 
                && self.sync.device_name != old_config.sync.device_name 
                && !self.sync.device_name.is_empty() 
            {
                // Create client using the proposed sync settings
                let client = WebDavClient::new(
                    &self.sync.webdav_url,
                    &self.sync.webdav_username,
                    self.sync.webdav_password.as_deref().unwrap_or(""),
                )?;
                
                // Scan the WebDAV .noda/sync/ folder
                let conflict = match client.propfind(".noda/sync", 1).await {
                    Ok(entries) => {
                        let expected_file = format!("{}.sync", self.sync.device_name);
                        let mut found = false;
                        for entry in entries {
                            if !entry.is_collection {
                                if let Some(filename) = std::path::Path::new(&entry.href).file_name() {
                                    if filename.to_string_lossy() == expected_file {
                                        found = true;
                                        break;
                                    }
                                }
                            }
                        }
                        found
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        if err_str.contains("404") || err_str.contains("Not Found") {
                            false
                        } else {
                            return Err(e);
                        }
                    }
                };

                if conflict {
                    return Err(NodaError::Sync("This device name is already taken!".to_string()));
                }

                // If old device name is not empty, delete the old .sync file from remote WebDAV
                if !old_config.sync.device_name.is_empty() {
                    let old_sync_path = format!(".noda/sync/{}.sync", old_config.sync.device_name);
                    let _ = client.delete(&old_sync_path).await;
                }

                // Create the new .sync file on remote WebDAV
                let _ = client.mkcol(".noda").await;
                let _ = client.mkcol(".noda/sync").await;
                let new_sync_path = format!(".noda/sync/{}.sync", self.sync.device_name);
                let _ = client.put(&new_sync_path, Vec::new()).await;
            }
        }
        
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(NodaError::Io)?;
        }
        
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| NodaError::Sync(format!("Failed to serialize settings: {}", e)))?;
            
        // Write to tmp and rename atomically
        let tmp_path = config_path.with_extension("tmp");
        tokio::fs::write(&tmp_path, content).await.map_err(NodaError::Io)?;
        tokio::fs::rename(&tmp_path, &config_path).await.map_err(NodaError::Io)?;
        
        // For backwards compatibility, also save the sync config to the old path
        let _ = self.sync.save(vault_path).await;
        
        Ok(())
    }
}
