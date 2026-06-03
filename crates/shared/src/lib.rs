// Defines shared DTOs and models across crates.

use serde::{Deserialize, Serialize};

/// Standardized error structure for cross-boundary (IPC) communication.
#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
}

/// Data transfer objects (DTOs) for IPC boundary.
pub mod dtos {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NoteDto {
        pub id: String,
        pub parent_id: Option<String>,
        pub title: String,
        pub body: String,
        pub color: Option<String>,
        pub pinned: bool,
        pub tags: Vec<String>,
        pub created_at: String,
        pub updated_at: String,
        pub file_path: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NoteMetadataDto {
        pub id: String,
        pub title: String,
        pub file_name: String,
        pub relative_path: String,
        pub absolute_path: String,
        pub created_at: String,
        pub updated_at: String,
        pub tags: Vec<String>,
        pub history_count: usize,
        pub last_upload_time: Option<String>,
        pub file_size_bytes: u64,
        pub word_count: usize,
        pub char_count: usize,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NoteListItemDto {
        pub id: String,
        pub parent_id: Option<String>,
        pub title: String,
        pub color: Option<String>,
        pub pinned: bool,
        pub tags: Vec<String>,
        pub updated_at: String,
        pub file_path: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct VaultInfoDto {
        pub name: String,
        pub path: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SearchResultDto {
        pub note_id: String,
        pub title: String,
        pub snippet: String,
        pub match_type: String,
        pub score: f64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DiffChunk {
        pub tag: String, // "Equal", "Insert", "Delete"
        pub text: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SnapshotDiffDto {
        pub note_id: String,
        pub timestamp: String,
        pub body_chunks: Vec<DiffChunk>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SnapshotDto {
        pub note_id: String,
        pub timestamp: String,
        pub file_path: String,
        pub size_bytes: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TrashEntryDto {
        pub id: String,
        pub title: String,
        pub original_path: String,
        pub deleted_at: String,
        pub trash_path: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ConflictEntryDto {
        pub id: String,
        pub title: String,
        pub file_path: String,
        pub archived_path: String,
        pub detected_at: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AttachmentInfoDto {
        pub name: String,
        pub size_bytes: u64,
        pub mime_type: String,
        pub modified_at: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct SyncReportDto {
        pub uploads: u32,
        pub downloads: u32,
        pub deletes_local: u32,
        pub deletes_remote: u32,
        pub conflicts: u32,
        pub uploaded_files: Vec<String>,
        pub downloaded_files: Vec<String>,
        pub deleted_local_files: Vec<String>,
        pub deleted_remote_files: Vec<String>,
        pub conflict_files: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AppearanceSettingsDto {
        pub theme: String,
        pub accent_color: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EditorSettingsDto {
        pub font_size: u32,
        pub typography: String,
        pub show_word_count: bool,
        pub auto_save_delay_ms: u32,
        pub default_daily_template: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TagWithCountDto {
        pub name: String,
        pub count: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HistorySettingsDto {
        pub retention_days: u32,
        pub max_snapshots_per_note: u32,
        pub empty_trash_after_days: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SyncConfigDto {
        pub webdav_url: String,
        pub webdav_username: String,
        pub webdav_password: Option<String>,
        pub interval_secs: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SettingsDto {
        pub appearance: AppearanceSettingsDto,
        pub editor: EditorSettingsDto,
        pub sync: SyncConfigDto,
        pub history: HistorySettingsDto,
    }
}

