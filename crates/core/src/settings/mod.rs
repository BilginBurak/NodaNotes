use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::errors::NodaError;
use crate::sync::engine::SyncConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub theme: String, // "light", "dark", "auto"
    pub accent_color: String,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            accent_color: "blue".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub font_size: u32,
    pub typography: String, // "sans", "serif", "mono"
    pub show_word_count: bool,
    pub auto_save_delay_ms: u32,
    pub default_daily_template: Option<String>,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_size: 14,
            typography: "sans".to_string(),
            show_word_count: true,
            auto_save_delay_ms: 1500,
            default_daily_template: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySettings {
    pub retention_days: u32,
    pub max_snapshots_per_note: u32,
    pub empty_trash_after_days: u32,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            retention_days: 30,
            max_snapshots_per_note: 50,
            empty_trash_after_days: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub appearance: AppearanceSettings,
    pub editor: EditorSettings,
    pub sync: SyncConfig,
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
