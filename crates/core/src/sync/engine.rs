//! Core sync engine orchestrator.
//! Manages background periodic sync execution, manual sync triggers,
//! event emission, conflict resolution and atomic local-to-remote file processing.

use std::path::Path;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::errors::NodaError;
use crate::database::connection::Database;
use crate::sync::client::{WebDavClient, RemoteEntry};
use crate::sync::traversal::list_remote_tree;
use crate::sync::delta::{calculate_delta, calculate_raw_delta, LocalRawFile, SyncAction};
use crate::sync::remote_state::{load_remote_state, save_remote_state, RemoteFileMetadata};
use crate::sync::queue::SyncQueue;
use crate::sync::conflict::{handle_conflict, ConflictEntry};
use crate::vault::scan::scan_vault;
use crate::vault::service::VaultService;
use crate::models::note::Note;

/// Configuration for the sync engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncConfig {
    pub webdav_url: String,
    pub webdav_username: String,
    pub webdav_password: Option<String>,
    pub interval_secs: u64,
}

impl SyncConfig {
    /// Loads the configuration from `.noda/sync/config.json` within the vault
    pub async fn load<P: AsRef<Path>>(vault_path: P) -> Result<Self, NodaError> {
        let config_path = vault_path.as_ref().join(".noda/sync/config.json");
        if !config_path.exists() {
            return Ok(Self {
                webdav_url: "".to_string(),
                webdav_username: "".to_string(),
                webdav_password: None,
                interval_secs: 300, // 5 minutes default
            });
        }
        let content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(NodaError::Io)?;
        let config = serde_json::from_str(&content)
            .map_err(|e| NodaError::Sync(format!("Failed to parse sync config: {}", e)))?;
        Ok(config)
    }

    /// Saves the configuration to `.noda/sync/config.json` within the vault atomically
    pub async fn save<P: AsRef<Path>>(&self, vault_path: P) -> Result<(), NodaError> {
        let config_path = vault_path.as_ref().join(".noda/sync/config.json");
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(NodaError::Io)?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| NodaError::Sync(format!("Failed to serialize sync config: {}", e)))?;
        
        let temp_path = config_path.with_extension("tmp");
        tokio::fs::write(&temp_path, content).await.map_err(NodaError::Io)?;
        tokio::fs::rename(&temp_path, &config_path)
            .await
            .map_err(NodaError::Io)?;
        Ok(())
    }
}

/// The execution status of the sync engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncStatus {
    Idle,
    Syncing,
    Error(String),
}

/// Report containing statistics for a completed sync cycle
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SyncReport {
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

/// The core SyncEngine managing synchronization tasks and periodic sync loops
#[derive(Clone)]
pub struct SyncEngine {
    config: Arc<RwLock<SyncConfig>>,
    status: Arc<RwLock<SyncStatus>>,
    shutdown_tx: Arc<RwLock<Option<oneshot::Sender<()>>>>,
    background_task: Arc<RwLock<Option<JoinHandle<()>>>>,
    status_callback: Arc<RwLock<Option<Arc<dyn Fn(SyncStatus) + Send + Sync + 'static>>>>,
    sync_finished_callback: Arc<RwLock<Option<Arc<dyn Fn(SyncReport) + Send + Sync + 'static>>>>,
    conflict_callback: Arc<RwLock<Option<Arc<dyn Fn(ConflictEntry) + Send + Sync + 'static>>>>,
}

impl SyncEngine {
    /// Creates a new SyncEngine with the given configuration
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            status: Arc::new(RwLock::new(SyncStatus::Idle)),
            shutdown_tx: Arc::new(RwLock::new(None)),
            background_task: Arc::new(RwLock::new(None)),
            status_callback: Arc::new(RwLock::new(None)),
            sync_finished_callback: Arc::new(RwLock::new(None)),
            conflict_callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Updates the configuration of the sync engine dynamically
    pub fn set_config(&self, config: SyncConfig) {
        *self.config.write() = config;
    }

    /// Gets the current configuration of the sync engine
    pub fn get_config(&self) -> SyncConfig {
        self.config.read().clone()
    }

    /// Gets the current status of the sync engine
    pub fn get_status(&self) -> SyncStatus {
        self.status.read().clone()
    }

    /// Checks if the periodic background sync task is currently active/running
    pub fn is_background_sync_running(&self) -> bool {
        self.background_task.read().is_some()
    }

    /// Sets a status callback to be notified when status changes
    pub fn set_status_callback<F>(&self, callback: F)
    where
        F: Fn(SyncStatus) + Send + Sync + 'static,
    {
        *self.status_callback.write() = Some(Arc::new(callback));
    }

    /// Sets a sync finished callback to be notified when a sync cycle completes successfully
    pub fn set_sync_finished_callback<F>(&self, callback: F)
    where
        F: Fn(SyncReport) + Send + Sync + 'static,
    {
        *self.sync_finished_callback.write() = Some(Arc::new(callback));
    }

    /// Sets a conflict callback to be notified when a sync conflict is detected and archived
    pub fn set_conflict_callback<F>(&self, callback: F)
    where
        F: Fn(ConflictEntry) + Send + Sync + 'static,
    {
        *self.conflict_callback.write() = Some(Arc::new(callback));
    }

    /// Helper to update the internal status and invoke callbacks
    fn update_status(&self, new_status: SyncStatus) {
        *self.status.write() = new_status.clone();
        if let Some(cb) = &*self.status_callback.read() {
            cb(new_status);
        }
    }

    /// Spawns a background task for periodic sync execution
    pub fn start_sync<P: AsRef<Path>>(&self, vault_path: P, database: Database) -> Result<(), NodaError> {
        let mut bg_task = self.background_task.write();
        if bg_task.is_some() {
            // Already running
            return Ok(());
        }

        let (tx, mut rx) = oneshot::channel::<()>();
        *self.shutdown_tx.write() = Some(tx);

        let vault_path = vault_path.as_ref().to_path_buf();
        let engine_clone = self.clone();
        
        let interval_secs = self.config.read().interval_secs;
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = engine_clone.sync_now_internal(&vault_path, &database).await {
                            tracing::error!("Background sync cycle failed: {:?}", e);
                            engine_clone.update_status(SyncStatus::Error(e.to_string()));
                        }
                    }
                    _ = &mut rx => {
                        break;
                    }
                }
            }
        });

        *bg_task = Some(handle);
        Ok(())
    }

    /// Gracefully stops the periodic background sync task
    pub async fn stop_sync(&self) -> Result<(), NodaError> {
        if let Some(tx) = self.shutdown_tx.write().take() {
            let _ = tx.send(());
        }
        let handle = self.background_task.write().take();
        if let Some(h) = handle {
            let _ = h.await.map_err(|e| NodaError::Sync(format!("Failed to stop background task: {}", e)))?;
        }
        self.update_status(SyncStatus::Idle);
        Ok(())
    }

    /// Manually triggers a complete synchronization cycle
    pub async fn sync_now<P: AsRef<Path>>(&self, vault_path: P, database: Database) -> Result<SyncReport, NodaError> {
        {
            let current = self.status.read().clone();
            if current == SyncStatus::Syncing {
                return Err(NodaError::Sync("Sync already in progress".to_string()));
            }
        }
        self.sync_now_internal(vault_path.as_ref(), &database).await
    }

    /// Core sync implementation
    async fn sync_now_internal(&self, vault_path: &Path, database: &Database) -> Result<SyncReport, NodaError> {
        self.update_status(SyncStatus::Syncing);

        let config = self.config.read().clone();
        let client = WebDavClient::new(
            &config.webdav_url,
            &config.webdav_username,
            config.webdav_password.as_deref().unwrap_or(""),
        )?;

        // Ensure remote vault directory exists (InfiniCLOUD/WebDAV typical root setup)
        // A direct empty propfind checks connectivity
        let remote_entries = match list_remote_tree(&client, "").await {
            Ok(entries) => entries,
            Err(e) => {
                self.update_status(SyncStatus::Error(format!("Remote traversal failed: {}", e)));
                return Err(e);
            }
        };

        let local_notes = match scan_vault(vault_path).await {
            Ok(notes) => notes,
            Err(e) => {
                self.update_status(SyncStatus::Error(format!("Local vault scan failed: {}", e)));
                return Err(e);
            }
        };

        let mut remote_state = load_remote_state(vault_path).await.unwrap_or_default();
        let mut sync_queue = SyncQueue::load(vault_path).await?;
        let vault_service = VaultService::new(vault_path).map_err(|e| NodaError::Vault(e.to_string()))?;

        // 1. Calculate the sync plan for notes
        let note_plan = calculate_delta(&local_notes, &remote_entries, &remote_state, "");

        // 2. Scan and calculate the sync plan for raw files (.noda/attachments and .noda/history)
        let local_raw = scan_local_raw_files(vault_path).await.unwrap_or_default();
        let raw_plan = calculate_raw_delta(&local_raw, &remote_entries, &remote_state, "");

        // 3. Queue all actions to the persistent store
        for action in note_plan.actions.into_iter().chain(raw_plan.actions.into_iter()) {
            sync_queue.enqueue(action).await?;
        }

        let mut report = SyncReport::default();

        // 3. Process each queued action
        while let Some(entry) = sync_queue.dequeue().await {
            let action = entry.action.clone();
            match &action {
                SyncAction::Upload { relative_path } if relative_path.starts_with(".noda/") => {
                    let full_path = vault_path.join(relative_path);
                    match tokio::fs::read(&full_path).await {
                        Ok(bytes) => {
                            if let Err(e) = ensure_remote_parent_dirs_exist(&client, relative_path, &remote_entries, "").await {
                                sync_queue.enqueue(action.clone()).await?;
                                self.update_status(SyncStatus::Error(format!("Parent directory creation failed for {}: {}", relative_path, e)));
                                return Err(e);
                            }

                            if let Err(e) = client.put(relative_path, bytes.clone()).await {
                                sync_queue.enqueue(action.clone()).await?;
                                self.update_status(SyncStatus::Error(format!("Upload failed for raw file {}: {}", relative_path, e)));
                                return Err(e);
                            }

                            let meta = tokio::fs::metadata(&full_path).await.ok();
                            let modified = meta.as_ref()
                                .and_then(|m| m.modified().ok())
                                .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                                .unwrap_or_else(chrono::Utc::now);

                            // Fetch updated remote metadata for state tracking
                            let updated_entry = match client.propfind(relative_path, 0).await {
                                Ok(mut entries) => entries.pop(),
                                Err(_) => None,
                            };

                            if let Some(remote_entry) = updated_entry {
                                let lm = remote_entry.last_modified.as_ref()
                                    .and_then(|s| crate::sync::delta::parse_last_modified(s));
                                remote_state.files.insert(relative_path.clone(), RemoteFileMetadata {
                                    etag: remote_entry.etag,
                                    last_modified: lm,
                                    size: remote_entry.size.unwrap_or(bytes.len() as u64),
                                    local_updated_at: Some(modified),
                                });
                            } else {
                                remote_state.files.insert(relative_path.clone(), RemoteFileMetadata {
                                    etag: None,
                                    last_modified: Some(modified),
                                    size: bytes.len() as u64,
                                    local_updated_at: Some(modified),
                                });
                            }

                            report.uploads += 1;
                            report.uploaded_files.push(relative_path.clone());
                        }
                        Err(e) => {
                            tracing::warn!("Raw file {} was deleted locally before upload: {:?}", relative_path, e);
                        }
                    }
                }
                SyncAction::Upload { relative_path } => {
                    let note_id_str = relative_path.trim_end_matches(".md");
                    let note_id = if let Ok(ulid) = ulid::Ulid::from_string(note_id_str) {
                        crate::models::note::NoteId(ulid)
                    } else {
                        continue;
                    };

                    match vault_service.read_note(note_id).await {
                        Ok(note) => {
                            let markdown = note.to_markdown()
                                .map_err(|e| NodaError::Vault(e.to_string()))?;

                            if let Err(e) = ensure_remote_parent_dirs_exist(&client, relative_path, &remote_entries, "").await {
                                sync_queue.enqueue(action.clone()).await?;
                                self.update_status(SyncStatus::Error(format!("Parent directory creation failed for {}: {}", relative_path, e)));
                                return Err(e);
                            }

                            if let Err(e) = client.put(relative_path, markdown.as_bytes().to_vec()).await {
                                // Put it back to queue for resilience
                                sync_queue.enqueue(action.clone()).await?;
                                self.update_status(SyncStatus::Error(format!("Upload failed for {}: {}", relative_path, e)));
                                return Err(e);
                            }

                            // Fetch updated remote metadata for state tracking
                            let updated_entry = match client.propfind(relative_path, 0).await {
                                Ok(mut entries) => entries.pop(),
                                Err(_) => None,
                            };

                            if let Some(remote_entry) = updated_entry {
                                let lm = remote_entry.last_modified.as_ref()
                                    .and_then(|s| crate::sync::delta::parse_last_modified(s));
                                remote_state.files.insert(relative_path.clone(), RemoteFileMetadata {
                                    etag: remote_entry.etag,
                                    last_modified: lm,
                                    size: remote_entry.size.unwrap_or(markdown.len() as u64),
                                    local_updated_at: Some(note.updated_at),
                                });
                            } else {
                                remote_state.files.insert(relative_path.clone(), RemoteFileMetadata {
                                    etag: None,
                                    last_modified: Some(note.updated_at),
                                    size: markdown.len() as u64,
                                    local_updated_at: Some(note.updated_at),
                                });
                            }

                            report.uploads += 1;
                            report.uploaded_files.push(relative_path.clone());
                        }
                        Err(e) => {
                            tracing::warn!("Note {} was deleted locally before upload: {:?}", relative_path, e);
                        }
                    }
                }
                SyncAction::Download { relative_path, remote_entry } if relative_path.starts_with(".noda/") => {
                    match client.get(relative_path).await {
                        Ok(bytes) => {
                            let full_path = vault_path.join(relative_path);
                            // Ensure parent directory exists
                            if let Some(parent) = full_path.parent() {
                                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                                    sync_queue.enqueue(action.clone()).await?;
                                    self.update_status(SyncStatus::Error(format!("Failed to create parent directory for downloaded file: {}", e)));
                                    return Err(NodaError::Io(e));
                                }
                            }

                            if let Err(e) = tokio::fs::write(&full_path, &bytes).await {
                                sync_queue.enqueue(action.clone()).await?;
                                self.update_status(SyncStatus::Error(format!("Write failed for downloaded file {}: {}", relative_path, e)));
                                return Err(NodaError::Io(e));
                            }

                            let meta = tokio::fs::metadata(&full_path).await.ok();
                            let modified = meta.as_ref()
                                .and_then(|m| m.modified().ok())
                                .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                                .unwrap_or_else(chrono::Utc::now);

                            // Update Remote State
                            let lm = remote_entry.last_modified.as_ref()
                                .and_then(|s| crate::sync::delta::parse_last_modified(s));
                            
                            remote_state.files.insert(relative_path.clone(), RemoteFileMetadata {
                                etag: remote_entry.etag.clone(),
                                last_modified: lm,
                                size: remote_entry.size.unwrap_or(bytes.len() as u64),
                                local_updated_at: Some(modified),
                            });

                            report.downloads += 1;
                            report.downloaded_files.push(relative_path.clone());
                        }
                        Err(NodaError::NotFound(msg)) => {
                            tracing::warn!("Download skipped - Raw file not found on remote: {}. Cleaning up remote state.", msg);
                            remote_state.files.remove(relative_path);
                        }
                        Err(e) => {
                            sync_queue.enqueue(action.clone()).await?;
                            self.update_status(SyncStatus::Error(format!("Download failed for {}: {}", relative_path, e)));
                            return Err(e);
                        }
                    }
                }
                SyncAction::Download { relative_path, remote_entry } => {
                    match client.get(relative_path).await {
                        Ok(bytes) => {
                            let content = String::from_utf8_lossy(&bytes).into_owned();
                            let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
                            let parsed = matter.parse(&content);
                            
                            if let Some(data) = parsed.data {
                                match data.deserialize::<crate::models::note::Frontmatter>() {
                                    Ok(frontmatter) => {
                                        let note = Note {
                                            id: frontmatter.id,
                                            parent_id: frontmatter.parent_id,
                                            title: frontmatter.title,
                                            body: parsed.content,
                                            color: frontmatter.color,
                                            pinned: frontmatter.pinned,
                                            tags: frontmatter.tags,
                                            status: frontmatter.status,
                                            created_at: frontmatter.created_at,
                                            updated_at: frontmatter.updated_at,
                                            file_path: relative_path.clone(),
                                        };

                                        // Persist locally
                                        if let Err(e) = vault_service.write_note(&note).await {
                                            sync_queue.enqueue(action.clone()).await?;
                                            self.update_status(SyncStatus::Error(format!("Write failed for downloaded note: {}", e)));
                                            return Err(e);
                                        }

                                        // Update SQLite in a separate block to ensure the MutexGuard is dropped before any await point
                                        let db_result = {
                                            let db_conn = database.conn.lock();
                                            crate::database::queries::upsert_note(&db_conn, &note, relative_path, "dummy_hash")
                                        };
                                        if let Err(e) = db_result {
                                            sync_queue.enqueue(action.clone()).await?;
                                            self.update_status(SyncStatus::Error(format!("DB update failed for downloaded note: {}", e)));
                                            return Err(e);
                                        }

                                        // Update Remote State
                                        let lm = remote_entry.last_modified.as_ref()
                                            .and_then(|s| crate::sync::delta::parse_last_modified(s));
                                        
                                        remote_state.files.insert(relative_path.clone(), RemoteFileMetadata {
                                            etag: remote_entry.etag.clone(),
                                            last_modified: lm,
                                            size: remote_entry.size.unwrap_or(bytes.len() as u64),
                                            local_updated_at: Some(note.updated_at),
                                        });

                                        report.downloads += 1;
                                        report.downloaded_files.push(relative_path.clone());
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to parse downloaded note {} frontmatter: {:?}", relative_path, e);
                                    }
                                }
                            } else {
                                tracing::error!("No frontmatter found in downloaded note {}", relative_path);
                            }
                        }
                        Err(NodaError::NotFound(msg)) => {
                            tracing::warn!("Download skipped - Note not found on remote: {}. Cleaning up remote state.", msg);
                            remote_state.files.remove(relative_path);
                        }
                        Err(e) => {
                            sync_queue.enqueue(action.clone()).await?;
                            self.update_status(SyncStatus::Error(format!("Download failed for {}: {}", relative_path, e)));
                            return Err(e);
                        }
                    }
                }
                SyncAction::DeleteRemote { relative_path } => {
                    if let Err(e) = client.delete(relative_path).await {
                        sync_queue.enqueue(action.clone()).await?;
                        self.update_status(SyncStatus::Error(format!("Delete remote failed for {}: {}", relative_path, e)));
                        return Err(e);
                    }
                    remote_state.files.remove(relative_path);
                    report.deletes_remote += 1;
                    report.deleted_remote_files.push(relative_path.clone());
                }
                SyncAction::DeleteLocal { relative_path } if relative_path.starts_with(".noda/") => {
                    let full_path = vault_path.join(relative_path);
                    if full_path.exists() {
                        if let Err(e) = tokio::fs::remove_file(&full_path).await {
                            sync_queue.enqueue(action.clone()).await?;
                            self.update_status(SyncStatus::Error(format!("Delete local failed for {}: {}", relative_path, e)));
                            return Err(NodaError::Io(e));
                        }
                    }

                    remote_state.files.remove(relative_path);
                    report.deletes_local += 1;
                    report.deleted_local_files.push(relative_path.clone());
                }
                SyncAction::DeleteLocal { relative_path } => {
                    let full_path = vault_path.join(relative_path);
                    if full_path.exists() {
                        if let Err(e) = tokio::fs::remove_file(&full_path).await {
                            sync_queue.enqueue(action.clone()).await?;
                            self.update_status(SyncStatus::Error(format!("Delete local failed for {}: {}", relative_path, e)));
                            return Err(NodaError::Io(e));
                        }
                    }

                    let db_result = {
                        let db_conn = database.conn.lock();
                        crate::database::queries::delete_note_by_path(&db_conn, relative_path)
                    };
                    if let Err(e) = db_result {
                        sync_queue.enqueue(action.clone()).await?;
                        self.update_status(SyncStatus::Error(format!("Delete DB note failed for {}: {}", relative_path, e)));
                        return Err(e);
                    }

                    remote_state.files.remove(relative_path);
                    report.deletes_local += 1;
                    report.deleted_local_files.push(relative_path.clone());
                }
                SyncAction::Conflict { relative_path, local_note, remote_entry: _ } => {
                    // Conflict Resolution Flow:
                    // 1. Download conflicting remote copy
                    match client.get(relative_path).await {
                        Ok(remote_bytes) => {
                            // 2. Archive remote copy locally
                            match handle_conflict(vault_path, local_note, &remote_bytes).await {
                                Ok(conflict_entry) => {
                                    // 3. Resolve by uploading local version as primary source
                                    let markdown = local_note.to_markdown()
                                        .map_err(|e| NodaError::Vault(e.to_string()))?;

                                    if let Err(e) = client.put(relative_path, markdown.as_bytes().to_vec()).await {
                                        sync_queue.enqueue(action.clone()).await?;
                                        self.update_status(SyncStatus::Error(format!("Conflict resolution upload failed for {}: {}", relative_path, e)));
                                        return Err(e);
                                    }

                                    // 4. Update Remote State
                                    let updated_entry = match client.propfind(relative_path, 0).await {
                                        Ok(mut entries) => entries.pop(),
                                        Err(_) => None,
                                    };

                                    if let Some(entry) = updated_entry {
                                        let lm = entry.last_modified.as_ref()
                                            .and_then(|s| crate::sync::delta::parse_last_modified(s));
                                        remote_state.files.insert(relative_path.clone(), RemoteFileMetadata {
                                            etag: entry.etag,
                                            last_modified: lm,
                                            size: entry.size.unwrap_or(markdown.len() as u64),
                                            local_updated_at: Some(local_note.updated_at),
                                        });
                                    } else {
                                        remote_state.files.insert(relative_path.clone(), RemoteFileMetadata {
                                            etag: None,
                                            last_modified: Some(local_note.updated_at),
                                            size: markdown.len() as u64,
                                            local_updated_at: Some(local_note.updated_at),
                                        });
                                    }

                                    // Notify conflict callback if registered
                                    if let Some(cb) = &*self.conflict_callback.read() {
                                        cb(conflict_entry.clone());
                                    }

                                    tracing::info!("Conflict archived and resolved: {:?}", conflict_entry);
                                    report.conflicts += 1;
                                    report.conflict_files.push(relative_path.clone());
                                }
                                Err(e) => {
                                    sync_queue.enqueue(action.clone()).await?;
                                    self.update_status(SyncStatus::Error(format!("Archive conflict failed for {}: {}", relative_path, e)));
                                    return Err(e);
                                }
                            }
                        }
                        Err(NodaError::NotFound(msg)) => {
                            tracing::warn!("Conflict download failed with 404 - Remote file no longer exists: {}. Proceeding to upload local note.", msg);
                            let markdown = local_note.to_markdown()
                                .map_err(|e| NodaError::Vault(e.to_string()))?;

                            if let Err(e) = client.put(relative_path, markdown.as_bytes().to_vec()).await {
                                sync_queue.enqueue(action.clone()).await?;
                                self.update_status(SyncStatus::Error(format!("Conflict resolution upload failed for {}: {}", relative_path, e)));
                                return Err(e);
                            }

                            // Fetch updated remote metadata for state tracking
                            let updated_entry = match client.propfind(relative_path, 0).await {
                                Ok(mut entries) => entries.pop(),
                                Err(_) => None,
                            };

                            if let Some(entry) = updated_entry {
                                let lm = entry.last_modified.as_ref()
                                    .and_then(|s| crate::sync::delta::parse_last_modified(s));
                                remote_state.files.insert(relative_path.clone(), RemoteFileMetadata {
                                    etag: entry.etag,
                                    last_modified: lm,
                                    size: entry.size.unwrap_or(markdown.len() as u64),
                                    local_updated_at: Some(local_note.updated_at),
                                });
                            } else {
                                remote_state.files.insert(relative_path.clone(), RemoteFileMetadata {
                                    etag: None,
                                    last_modified: Some(local_note.updated_at),
                                    size: markdown.len() as u64,
                                    local_updated_at: Some(local_note.updated_at),
                                });
                            }
                        }
                        Err(e) => {
                            sync_queue.enqueue(action.clone()).await?;
                            self.update_status(SyncStatus::Error(format!("Failed to retrieve remote conflicting file {}: {}", relative_path, e)));
                            return Err(e);
                        }
                    }
                }
            }
        }

        // For any files that were already identical and had no action, ensure they are in remote_state
        let local_map: std::collections::HashMap<String, &Note> = local_notes
            .iter()
            .map(|n| (format!("{}.md", n.id.0.to_string()), n))
            .collect();

        let remote_map: std::collections::HashMap<String, &RemoteEntry> = remote_entries
            .iter()
            .filter(|e| !e.is_collection)
            .map(|e| (crate::sync::delta::get_relative_path(&e.href, ""), e))
            .collect();

        for (path, local_note) in &local_map {
            if !remote_state.files.contains_key(path) {
                if let Some(remote_entry) = remote_map.get(path) {
                    if crate::sync::delta::is_local_remote_identical(local_note, remote_entry) {
                        let lm = remote_entry.last_modified.as_ref()
                            .and_then(|s| crate::sync::delta::parse_last_modified(s));
                        remote_state.files.insert(path.clone(), RemoteFileMetadata {
                            etag: remote_entry.etag.clone(),
                            last_modified: lm,
                            size: remote_entry.size.unwrap_or(0),
                            local_updated_at: Some(local_note.updated_at),
                        });
                    }
                }
            }
        }

        // Save updated remote state to survive crash
        remote_state.last_sync_time = Some(chrono::Utc::now());
        save_remote_state(vault_path, &remote_state).await?;

        self.update_status(SyncStatus::Idle);
        if let Some(cb) = &*self.sync_finished_callback.read() {
            cb(report.clone());
        }
        Ok(report)
    }
}

async fn ensure_remote_parent_dirs_exist(
    client: &WebDavClient,
    relative_path: &str,
    remote_entries: &[RemoteEntry],
    root_path: &str,
) -> Result<(), NodaError> {
    let mut parts: Vec<&str> = relative_path.split('/').collect();
    if parts.is_empty() {
        return Ok(());
    }
    
    parts.pop();
    
    let mut current_path = String::new();
    
    let existing_collections: std::collections::HashSet<String> = remote_entries
        .iter()
        .filter(|e| e.is_collection)
        .map(|e| crate::sync::delta::get_relative_path(&e.href, root_path))
        .collect();

    for part in parts {
        if part.is_empty() {
            continue;
        }
        if current_path.is_empty() {
            current_path = part.to_string();
        } else {
            current_path = format!("{}/{}", current_path, part);
        }
        
        if !existing_collections.contains(&current_path) {
            tracing::info!("Creating remote collection: {}", current_path);
            match client.mkcol(&current_path).await {
                Ok(_) => {}
                Err(e) => {
                    let err_str = e.to_string();
                    if !err_str.contains("405") && !err_str.contains("409") {
                        return Err(e);
                    }
                }
            }
        }
    }
    
    Ok(())
}

async fn scan_local_raw_files<P: AsRef<Path>>(vault_path: P) -> Result<Vec<LocalRawFile>, NodaError> {
    let mut files = Vec::new();
    let vault_ref = vault_path.as_ref();
    let attachments_dir = vault_ref.join(".noda").join("attachments");
    let history_dir = vault_ref.join(".noda").join("history");

    for dir in &[attachments_dir, history_dir] {
        if dir.exists() {
            for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.starts_with('.') {
                            continue;
                        }
                    }
                    if let Ok(rel_path) = entry.path().strip_prefix(vault_ref) {
                        let path_str = rel_path.to_string_lossy().to_string();
                        if let Ok(meta) = entry.metadata() {
                            let size = meta.len();
                            let modified = meta.modified()
                                .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                                .unwrap_or_else(|_| chrono::Utc::now());
                            files.push(LocalRawFile {
                                relative_path: path_str,
                                size,
                                modified,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::tempdir;
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_sync_engine_status_callback() {
        let config = SyncConfig {
            webdav_url: "http://127.0.0.1:12345".to_string(),
            webdav_username: "user".to_string(),
            webdav_password: Some("pass".to_string()),
            interval_secs: 10,
        };

        let engine = SyncEngine::new(config);
        assert_eq!(engine.get_status(), SyncStatus::Idle);

        let status_changed = Arc::new(RwLock::new(false));
        let status_changed_clone = status_changed.clone();

        engine.set_status_callback(move |status| {
            if status == SyncStatus::Syncing {
                *status_changed_clone.write() = true;
            }
        });

        engine.update_status(SyncStatus::Syncing);
        assert_eq!(engine.get_status(), SyncStatus::Syncing);
        assert!(*status_changed.read());
    }

    #[tokio::test]
    async fn test_sync_engine_orchestration_flow() {
        // Start a local mock WebDAV HTTP server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_url = format!("http://{}", addr);

        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let mut temp = [0; 1024];
                    loop {
                        match socket.read(&mut temp).await {
                            Ok(0) => break,
                            Ok(n) => {
                                buf.extend_from_slice(&temp[..n]);
                                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    if buf.is_empty() { return; }
                    let req_str = String::from_utf8_lossy(&buf);
                        
                        if req_str.starts_with("PROPFIND") {
                            // Remote has a new note to download: "01H7V18105M5MWRB28Z1220000.md"
                            let body = concat!(
                                "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
                                "<d:multistatus xmlns:d=\"DAV:\">\n",
                                "  <d:response>\n",
                                "    <d:href>/</d:href>\n",
                                "    <d:propstat>\n",
                                "      <d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop>\n",
                                "      <d:status>HTTP/1.1 200 OK</d:status>\n",
                                "    </d:propstat>\n",
                                "  </d:response>\n",
                                "  <d:response>\n",
                                "    <d:href>/01H7V18105M5MWRB28Z1220000.md</d:href>\n",
                                "    <d:propstat>\n",
                                "      <d:prop>\n",
                                "        <d:resourcetype/>\n",
                                "        <d:getcontentlength>150</d:getcontentlength>\n",
                                "        <d:getlastmodified>Wed, 20 May 2026 03:00:00 GMT</d:getlastmodified>\n",
                                "        <d:getetag>\"etag-note\"</d:getetag>\n",
                                "      </d:prop>\n",
                                "      <d:status>HTTP/1.1 200 OK</d:status>\n",
                                "    </d:propstat>\n",
                                "  </d:response>\n",
                                "</d:multistatus>"
                            );
                            let response = format!(
                                "HTTP/1.1 207 Multi-Status\r\nContent-Type: text/xml; charset=\"utf-8\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        } else if req_str.starts_with("GET") {
                            // Return the note content for the new download
                            let body = concat!(
                                "---\n",
                                "id: 01H7V18105M5MWRB28Z1220000\n",
                                "title: \"Remote Note\"\n",
                                "pinned: false\n",
                                "tags: []\n",
                                "status: \"active\"\n",
                                "created_at: \"2026-05-20T03:00:00Z\"\n",
                                "updated_at: \"2026-05-20T03:00:00Z\"\n",
                                "---\n",
                                "Body of remote note"
                            );
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/markdown\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        } else if req_str.starts_with("PUT") {
                            let response = "HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    });
                }
            });

        // Prepare temporary vault and DB
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("index.db");
        let database = Database::open(&db_path).unwrap();

        // Create engine
        let config = SyncConfig {
            webdav_url: server_url,
            webdav_username: "user".to_string(),
            webdav_password: Some("pass".to_string()),
            interval_secs: 10,
        };
        let engine = SyncEngine::new(config);

        // Run manual sync
        let report = engine.sync_now(dir.path(), database).await.unwrap();

        // Assert download happened
        assert_eq!(report.downloads, 1);
        assert_eq!(report.uploads, 0);

        // Verify file is saved on disk
        let saved_file = dir.path().join("01H7V18105M5MWRB28Z1220000.md");
        assert!(saved_file.exists());
    }


}
