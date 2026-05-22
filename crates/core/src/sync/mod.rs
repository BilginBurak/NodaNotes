//! Sync Engine for remote storage integration

pub mod client;
pub mod traversal;
pub mod remote_state;
pub mod delta;
pub mod conflict;
pub mod queue;
pub mod engine;

pub use client::{WebDavClient, RemoteEntry};
pub use traversal::list_remote_tree;
pub use remote_state::{RemoteFileMetadata, RemoteState, load_remote_state, save_remote_state};
pub use delta::{SyncAction, SyncPlan, calculate_delta};
pub use conflict::{ConflictEntry, handle_conflict, list_conflicts};
pub use queue::{SyncQueueEntry, SyncQueue};
pub use engine::{SyncEngine, SyncConfig, SyncStatus, SyncReport};
