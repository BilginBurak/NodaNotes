//! # NodaError
//!
//! The central error type for the `core` crate.
//!
//! All fallible operations in `crates/core` return `Result<T, NodaError>`.
//! `NodaError` is converted to `shared::AppError` by `crates/tauri-shell`
//! before crossing the IPC boundary — internal details are never exposed to
//! the frontend.

use thiserror::Error;

/// All errors that can occur within the Noda core engine.
#[derive(Debug, Error)]
pub enum NodaError {
    // ─── I/O ──────────────────────────────────────────────────────────────
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    // ─── Vault ────────────────────────────────────────────────────────────
    #[error("Vault not found at path: {path}")]
    VaultNotFound { path: String },

    #[error("Invalid vault structure: {reason}")]
    VaultInvalid { reason: String },

    #[error("A note with the title '{title}' already exists in this vault")]
    DuplicateFilename { title: String },

    #[error("Note not found: {id}")]
    NoteNotFound { id: String },

    // ─── Frontmatter ──────────────────────────────────────────────────────
    #[error("Failed to parse YAML frontmatter in '{file}': {reason}")]
    FrontmatterParse { file: String, reason: String },

    #[error("Missing required frontmatter field '{field}' in '{file}'")]
    FrontmatterMissingField { field: String, file: String },

    // ─── Database ─────────────────────────────────────────────────────────
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Database schema migration failed: {reason}")]
    Migration { reason: String },

    // ─── Search ───────────────────────────────────────────────────────────
    #[error("Search index error: {reason}")]
    SearchIndex { reason: String },

    // ─── Watcher ──────────────────────────────────────────────────────────
    #[error("File watcher error: {reason}")]
    Watcher { reason: String },

    // ─── Sync ─────────────────────────────────────────────────────────────
    #[error("WebDAV request failed: {reason}")]
    WebDav { reason: String },

    #[error("HTTP error {status}: {reason}")]
    Http { status: u16, reason: String },

    #[error("Sync queue error: {reason}")]
    SyncQueue { reason: String },

    #[error("Sync conflict detected for note '{title}'")]
    SyncConflict { title: String },

    #[error("Network error: {reason}")]
    Network { reason: String },

    // ─── Protocol / Security ──────────────────────────────────────────────
    #[error("Path traversal attempt rejected: '{path}'")]
    PathTraversal { path: String },

    #[error("Attachment not found: '{name}'")]
    AttachmentNotFound { name: String },

    // ─── History ──────────────────────────────────────────────────────────
    #[error("Snapshot not found: '{path}'")]
    SnapshotNotFound { path: String },

    // ─── Trash ────────────────────────────────────────────────────────────
    #[error("Trash entry not found: '{id}'")]
    TrashEntryNotFound { id: String },

    // ─── Generic ──────────────────────────────────────────────────────────
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
}
