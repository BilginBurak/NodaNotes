use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::errors::NodaError;
use crate::sync::engine::SyncConfig;

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
        if !config_path.exists() {
            let old_sync = SyncConfig::load(vault_path.as_ref()).await.unwrap_or_default();
            let default_config = Self {
                sync: old_sync,
                ..Default::default()
            };
            return Ok(default_config);
        }

        let content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(NodaError::Io)?;
            
        let config = serde_json::from_str(&content)
            .map_err(|e| NodaError::Sync(format!("Failed to parse settings: {}", e)))?;
            
        Ok(config)
    }

    pub async fn save<P: AsRef<Path>>(&self, vault_path: P) -> Result<(), NodaError> {
        let config_path = vault_path.as_ref().join(".noda/settings.json");
        
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
