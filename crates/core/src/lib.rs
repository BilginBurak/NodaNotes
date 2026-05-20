//! # noda-core
//!
//! Pure Rust engine for the Noda markdown vault application.
//!
//! This crate is **UI-agnostic** and must compile without Tauri, WebView,
//! or any platform-specific UI dependency. It is the sole implementation of
//! all business logic:
//!
//! - **vault** — vault lifecycle, note CRUD, atomic file I/O
//! - **models** — internal domain types
//! - **errors** — core error hierarchy (`NodaError`)
//! - **database** — SQLite cache and FTS5 search index
//! - **search** — full-text search query execution
//! - **watcher** — filesystem monitoring with event batching
//! - **history** — per-note snapshot system
//! - **trash** — soft-delete and restore
//! - **sync** — WebDAV delta synchronisation engine
//! - **attachments** — attachment storage and path resolution
//! - **protocol** — custom protocol path validation and file serving

pub mod attachments;
pub mod database;
pub mod errors;
pub mod history;
pub mod models;
pub mod protocol;
pub mod search;
pub mod sync;
pub mod trash;
pub mod vault;
pub mod watcher;

/// Re-export the primary error type for convenient use by dependent crates.
pub use errors::NodaError;
