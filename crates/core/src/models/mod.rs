//! # Domain Models
//!
//! Internal domain types for the Noda core engine.
//! These types are NOT directly serialized across the IPC boundary;
//! `crates/shared` DTOs handle that responsibility.

pub mod note;
pub mod vault;
pub mod sync;
pub mod snapshot;
pub mod trash;

pub use note::{Note, NoteMeta, Frontmatter};
pub use vault::Vault;
pub use sync::{SyncAction, SyncQueueEntry, RemoteEntry};
pub use snapshot::Snapshot;
pub use trash::TrashEntry;
