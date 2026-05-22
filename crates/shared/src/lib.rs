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
}
