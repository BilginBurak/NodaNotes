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
use crate::sync::delta::SyncAction;
use crate::sync::remote_state::{load_remote_state, save_remote_state, RemoteState, RemoteFileMetadata};
use crate::sync::conflict::{handle_conflict, ConflictEntry};
use crate::vault::service::VaultService;
use crate::models::note::Note;

/// Configuration for the sync engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncConfig {
    pub webdav_url: String,
    pub webdav_username: String,
    pub webdav_password: Option<String>,
    pub interval_secs: u64,
    #[serde(default)]
    pub device_name: String,
}

impl SyncConfig {
    /// Generates a random unique device name
    pub fn generate_random_device_name() -> String {
        let u = uuid::Uuid::new_v4();
        let hex = &u.to_string()[..4];
        format!("Noda-Device-{}", hex.to_uppercase())
    }

    /// Loads the configuration from `.noda/sync/config.json` within the vault
    pub async fn load<P: AsRef<Path>>(vault_path: P) -> Result<Self, NodaError> {
        let config_path = vault_path.as_ref().join(".noda/sync/config.json");
        if !config_path.exists() {
            return Ok(Self {
                webdav_url: "".to_string(),
                webdav_username: "".to_string(),
                webdav_password: None,
                interval_secs: 300, // 5 minutes default
                device_name: Self::generate_random_device_name(),
            });
        }
        let content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(NodaError::Io)?;
        let mut config: Self = serde_json::from_str(&content)
            .map_err(|e| NodaError::Sync(format!("Failed to parse sync config: {}", e)))?;
        
        if config.device_name.is_empty() {
            config.device_name = Self::generate_random_device_name();
            // Save it back immediately so it's persisted on disk
            let _ = config.save(vault_path).await;
        }
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

/// Progress event representing a single file being processed in the sync pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncProgress {
    pub status: String,
    pub action: String,
    pub file_path: String,
    pub current_index: usize,
    pub total_count: usize,
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
    progress_callback: Arc<RwLock<Option<Arc<dyn Fn(SyncProgress) + Send + Sync + 'static>>>>,
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
            progress_callback: Arc::new(RwLock::new(None)),
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

    /// Sets a progress callback to be notified of individual sync actions progress
    pub fn set_progress_callback<F>(&self, callback: F)
    where
        F: Fn(SyncProgress) + Send + Sync + 'static,
    {
        *self.progress_callback.write() = Some(Arc::new(callback));
    }

    /// Helper to update the internal status and invoke callbacks
    fn update_status(&self, new_status: SyncStatus) {
        *self.status.write() = new_status.clone();
        if let Some(cb) = &*self.status_callback.read() {
            cb(new_status);
        }
    }

    fn emit_progress(&self, progress: SyncProgress) {
        if let Some(cb) = &*self.progress_callback.read() {
            cb(progress);
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
        let result = self.sync_now_internal_inner(vault_path, database).await;
        self.update_status(SyncStatus::Idle);
        result
    }

    async fn sync_now_internal_inner(&self, vault_path: &Path, database: &Database) -> Result<SyncReport, NodaError> {

        let config = self.config.read().clone();
        let client = WebDavClient::new(
            &config.webdav_url,
            &config.webdav_username,
            config.webdav_password.as_deref().unwrap_or(""),
        )?;

        // Step 1: Load remote state from database
        let mut remote_state = {
            let conn = database.conn.lock();
            load_remote_state(&conn).unwrap_or_default()
        };

        // Step 2: Check for local changes (O(1) SQLite Dirty Check Constraint)
        let _local_changed = {
            let conn = database.conn.lock();
            let dirty_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sync_file_states WHERE is_dirty = 1",
                [],
                |row| row.get(0)
            ).unwrap_or(0);
            dirty_count > 0
        };

        let mut report = SyncReport::default();
        let vault_service = VaultService::new(vault_path).map_err(|e| NodaError::Vault(e.to_string()))?;
        let conflict_callback_clone = self.conflict_callback.read().clone();

        // ----------------- STEP 3: WebDAV Fast-Check and Pulling Remote Changes First -----------------
        let sync_dir_path = ".noda/sync";
        let propfind_res = client.propfind(sync_dir_path, 1).await;
        
        let mut remote_entries_list = Vec::new();
        let mut device_remote_actions = Vec::new();

        if let Ok(ref remote_sync_entries) = propfind_res {
            let mut active_remote_devices = std::collections::HashSet::new();
            let mut changed_devices = Vec::new();

            for entry in remote_sync_entries {
                if entry.is_collection {
                    continue;
                }
                let rel_path = crate::sync::delta::get_relative_path(&entry.href, "");
                let filename = std::path::Path::new(&rel_path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");

                if filename.ends_with(".sync") {
                    let device_name = filename.strip_suffix(".sync").unwrap_or(filename).to_string();
                    if device_name == config.device_name {
                        continue;
                    }
                    active_remote_devices.insert(device_name.clone());

                    // Evaluate if signature changed
                    let mut signature_changed = true;
                    if let Some(cached_device) = remote_state.devices.get(&device_name) {
                        let etag_matches = match (&entry.etag, &cached_device.last_known_etag) {
                            (Some(e1), Some(e2)) => e1 == e2,
                            _ => false,
                        };
                        let lm_matches = match (&entry.last_modified, &cached_device.last_known_modified) {
                            (Some(lm1), Some(lm2)) => lm1 == &lm2.to_rfc3339() || {
                                if let Some(dt1) = crate::sync::delta::parse_last_modified(lm1) {
                                    dt1 == *lm2
                                } else {
                                    false
                                }
                            },
                            _ => false,
                        };
                        if etag_matches || lm_matches {
                            signature_changed = false;
                        }
                    } else {
                        // The Bootstrapping Protocol (Force-Catch NULL States)
                        signature_changed = true;
                    }

                    if signature_changed {
                        changed_devices.push((device_name, entry.clone()));
                    }
                }
            }

            // Prune local cached devices not found on WebDAV
            let local_device_keys: Vec<String> = remote_state.devices.keys().cloned().collect();
            let mut pruning_happened = false;
            for cached_device in local_device_keys {
                if cached_device != config.device_name && !active_remote_devices.contains(&cached_device) {
                    tracing::info!("Pruning ghost device {} from local state", cached_device);
                    remote_state.devices.remove(&cached_device);
                    pruning_happened = true;
                }
            }
            if pruning_happened {
                let conn = database.conn.lock();
                let _ = save_remote_state(&conn, &remote_state);
            }

            // Loop through changed devices sequentially (Strict Pull-Before-Push Barrier)
            for (other_device, remote_sync_entry) in changed_devices {
                tracing::info!("Processing remote device: {}", other_device);

                let remote_manifest_path = format!(".noda/sync/manifests/manifest_{}.json", other_device);
                let manifest_bytes = match client.get(&remote_manifest_path).await {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!("Failed to download manifest for {}: {:?}", other_device, e);
                        continue;
                    }
                };

                let new_manifest: crate::sync::manifest::VaultManifest = match serde_json::from_slice(&manifest_bytes) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("Failed to parse manifest for {}: {:?}", other_device, e);
                        continue;
                    }
                };

                // Check historical footprint count
                let has_history = {
                    let conn = database.conn.lock();
                    let count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM peer_file_states WHERE device_name = ?1",
                        [&other_device],
                        |row| row.get(0)
                    ).unwrap_or(0);
                    count > 0
                };

                // Isolate deletions (Union-Based State Erasure Protection)
                let mut deleted_paths = Vec::new();
                if has_history {
                    let conn = database.conn.lock();
                    let mut stmt = conn.prepare("SELECT path FROM peer_file_states WHERE device_name = ?1").map_err(|e| NodaError::Database(e.to_string()))?;
                    let mut rows = stmt.query([&other_device]).map_err(|e| NodaError::Database(e.to_string()))?;
                    while let Some(row) = rows.next().map_err(|e| NodaError::Database(e.to_string()))? {
                        let path: String = row.get(0).map_err(|e| NodaError::Database(e.to_string()))?;
                        let path_lower = path.to_lowercase();
                        if path_lower.ends_with(".ds_store") || path_lower.contains("/.") {
                            continue;
                        }
                        if !new_manifest.files.contains_key(&path) {
                            deleted_paths.push(path);
                        }
                    }
                }

                // Atomic Peer Overwrite (Purge & Bulk Insert) inside a dedicated SQL transaction
                {
                    let mut conn = database.conn.lock();
                    let tx = conn.transaction().map_err(|e| NodaError::Database(e.to_string()))?;
                    tx.execute("DELETE FROM peer_file_states WHERE device_name = ?1", [&other_device]).map_err(|e| NodaError::Database(e.to_string()))?;
                    {
                        let mut stmt = tx.prepare("INSERT INTO peer_file_states (device_name, path, hash) VALUES (?1, ?2, ?3)").map_err(|e| NodaError::Database(e.to_string()))?;
                        for (path, hash) in &new_manifest.files {
                            stmt.execute(rusqlite::params![other_device, path, hash]).map_err(|e| NodaError::Database(e.to_string()))?;
                        }
                    }
                    tx.commit().map_err(|e| NodaError::Database(e.to_string()))?;
                }

                // Precise Native Compound Git-Diff Optimization
                let changed_paths = {
                    let conn = database.conn.lock();
                    let mut paths = Vec::new();
                    let mut stmt = conn.prepare(
                        "SELECT path FROM (
                            SELECT path, hash FROM peer_file_states WHERE device_name = ?1
                            EXCEPT
                            SELECT path, hash FROM sync_file_states
                        )"
                    ).map_err(|e| NodaError::Database(e.to_string()))?;
                    let rows = stmt.query_map([&other_device], |row| row.get::<_, String>(0)).map_err(|e| NodaError::Database(e.to_string()))?;
                    for r in rows {
                        if let Ok(p) = r {
                            let p_lower = p.to_lowercase();
                            if p_lower.ends_with(".ds_store") || p_lower.contains("/.") {
                                continue;
                            }
                            paths.push(p);
                        }
                    }
                    paths
                };

                // Load local file states for lookups
                let local_files = {
                    let conn = database.conn.lock();
                    load_remote_state(&conn).unwrap_or_default().files
                };

                let mut remote_actions = Vec::new();

                // 1. Process deletions
                for path in deleted_paths {
                    let local_file = local_files.get(&path);
                    let full_path = vault_path.join(&path);
                    if full_path.exists() {
                        let is_local_dirty = local_file.map(|f| f.is_dirty).unwrap_or(false);
                        if is_local_dirty {
                            if !path.starts_with(".noda/") {
                                let path_buf = std::path::Path::new(&path);
                                let note_id_str = path_buf.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                if let Ok(ulid) = ulid::Ulid::from_string(note_id_str) {
                                    let note_id = crate::models::note::NoteId(ulid);
                                    if let Ok(note) = vault_service.read_note(note_id).await {
                                        remote_actions.push(SyncAction::Conflict {
                                            relative_path: path.clone(),
                                            local_note: note,
                                            remote_entry: RemoteEntry {
                                                href: path.clone(),
                                                is_collection: false,
                                                size: Some(0),
                                                last_modified: None,
                                                etag: None,
                                            },
                                        });
                                    }
                                }
                            }
                        } else {
                            remote_actions.push(SyncAction::DeleteLocal { relative_path: path.clone() });
                        }
                    }
                }

                // 2. Process additions/modifications (changed_paths)
                for path in changed_paths {
                    let new_hash = new_manifest.files.get(&path).cloned();
                    let local_file = local_files.get(&path);
                    let nh = new_hash.unwrap_or_default();

                    let full_path = vault_path.join(&path);
                    if !full_path.exists() {
                        remote_actions.push(SyncAction::Download {
                            relative_path: path.clone(),
                            remote_entry: RemoteEntry {
                                href: path.clone(),
                                is_collection: false,
                                size: None,
                                last_modified: None,
                                etag: Some(nh),
                            },
                        });
                    } else {
                        let local_hash = local_file.map(|f| f.hash.clone()).unwrap_or_default();
                        if local_hash != nh {
                            let is_local_dirty = local_file.map(|f| f.is_dirty).unwrap_or(false);
                            if is_local_dirty {
                                if !path.starts_with(".noda/") {
                                    let path_buf = std::path::Path::new(&path);
                                    let note_id_str = path_buf.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                    if let Ok(ulid) = ulid::Ulid::from_string(note_id_str) {
                                        let note_id = crate::models::note::NoteId(ulid);
                                        if let Ok(note) = vault_service.read_note(note_id).await {
                                            remote_actions.push(SyncAction::Conflict {
                                                relative_path: path.clone(),
                                                local_note: note,
                                                remote_entry: RemoteEntry {
                                                    href: path.clone(),
                                                    is_collection: false,
                                                    size: None,
                                                    last_modified: None,
                                                    etag: Some(nh),
                                                },
                                            });
                                        }
                                    }
                                }
                            } else {
                                remote_actions.push(SyncAction::Download {
                                    relative_path: path.clone(),
                                    remote_entry: RemoteEntry {
                                        href: path.clone(),
                                        is_collection: false,
                                        size: None,
                                        last_modified: None,
                                        etag: Some(nh),
                                    },
                                });
                            }
                        }
                    }
                }

                device_remote_actions.push((other_device, remote_sync_entry, remote_actions));
            }
        }

        // ----------------- STEP 4: Local Mutation Phase (Push local changes) -----------------
        let (total_dirty, quarantined_dirty) = {
            let conn = database.conn.lock();
            let total = conn.query_row(
                "SELECT COUNT(*) FROM sync_file_states WHERE is_dirty = 1",
                [],
                |row| row.get::<_, i64>(0)
            ).unwrap_or(0);
            
            let quarantined = conn.query_row(
                "SELECT COUNT(*) FROM sync_file_states WHERE is_dirty = 1 AND retry_count > 0",
                [],
                |row| row.get::<_, i64>(0)
            ).unwrap_or(0);
            (total, quarantined)
        };

        let bypass_authorized = total_dirty > 0 && total_dirty == quarantined_dirty;
        let mut local_actions = Vec::new();
        
        if total_dirty > 0 && !bypass_authorized {
            let dirty_files = {
                let conn = database.conn.lock();
                let mut stmt = conn.prepare("SELECT path, hash FROM sync_file_states WHERE is_dirty = 1 AND (retry_count = 0 OR retry_count IS NULL)").unwrap();
                let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap();
                let mut list = Vec::new();
                for r in rows {
                    if let Ok(item) = r {
                        list.push(item);
                    }
                }
                list
            };

            for (rel_path, _hash) in dirty_files {
                let full_path = vault_path.join(&rel_path);
                if full_path.exists() {
                    local_actions.push(SyncAction::Upload { relative_path: rel_path });
                } else {
                    local_actions.push(SyncAction::DeleteRemote { relative_path: rel_path });
                }
            }
        }

        // Exclude paths from local actions if they are already present in remote actions
        let mut remote_action_paths = std::collections::HashSet::new();
        for (_, _, actions) in &device_remote_actions {
            for action in actions {
                let path = match action {
                    SyncAction::Upload { relative_path } => relative_path.clone(),
                    SyncAction::Download { relative_path, .. } => relative_path.clone(),
                    SyncAction::DeleteRemote { relative_path } => relative_path.clone(),
                    SyncAction::DeleteLocal { relative_path } => relative_path.clone(),
                    SyncAction::Conflict { relative_path, .. } => relative_path.clone(),
                };
                remote_action_paths.insert(path);
            }
        }

        local_actions.retain(|action| {
            let path = match action {
                SyncAction::Upload { relative_path } => relative_path,
                SyncAction::Download { relative_path, .. } => relative_path,
                SyncAction::DeleteRemote { relative_path } => relative_path,
                SyncAction::DeleteLocal { relative_path } => relative_path,
                SyncAction::Conflict { relative_path, .. } => relative_path,
            };
            !remote_action_paths.contains(path)
        });

        // Compute total counts
        let total_remote_actions_count = device_remote_actions.iter().map(|(_, _, actions)| actions.len()).sum::<usize>();
        let total_count = total_remote_actions_count + local_actions.len();
        let mut current_index = 0;

        // Execute remote actions
        for (other_device, remote_sync_entry, remote_actions) in device_remote_actions {
            if !remote_actions.is_empty() {
                let mut failed_paths = Vec::new();
                if remote_entries_list.is_empty() {
                    if let Ok(entries) = client.propfind("", 9).await {
                        remote_entries_list = entries;
                    }
                }

                for action in remote_actions {
                    current_index += 1;
                    let path = match &action {
                        SyncAction::Upload { relative_path } => relative_path.clone(),
                        SyncAction::Download { relative_path, .. } => relative_path.clone(),
                        SyncAction::DeleteRemote { relative_path } => relative_path.clone(),
                        SyncAction::DeleteLocal { relative_path } => relative_path.clone(),
                        SyncAction::Conflict { relative_path, .. } => relative_path.clone(),
                    };

                    let action_str = match &action {
                        SyncAction::Upload { .. } => "upload",
                        SyncAction::Download { .. } => "download",
                        SyncAction::DeleteRemote { .. } => "remote_delete",
                        SyncAction::DeleteLocal { .. } => "local_delete",
                        SyncAction::Conflict { .. } => "conflict",
                    };

                    self.emit_progress(SyncProgress {
                        status: "Processing".to_string(),
                        action: action_str.to_string(),
                        file_path: path.clone(),
                        current_index,
                        total_count,
                    });

                    println!("DEBUG_SYNC: Starting network action for path: {}", path);
                    match execute_single_action_sequential(
                        &action,
                        &client,
                        vault_path,
                        database,
                        &remote_entries_list,
                        &mut remote_state,
                        &mut report,
                        &vault_service,
                        &conflict_callback_clone,
                    ).await {
                        Ok(()) => {
                            println!("DEBUG_SYNC: Network success for path: {}. Proceeding to DB write.", path);
                        }
                        Err(e) => {
                            println!("DEBUG_SYNC: Network failed for path: {}. Error isolated. Proceeding to Quarantine write.", path);
                            failed_paths.push((path, action, e));
                        }
                    }
                }

                // Retry circuit for failed downloads
                if !failed_paths.is_empty() {
                    for (path, action, mut last_err) in failed_paths {
                        let mut success = false;
                        for attempt in 1..=3 {
                            tokio::time::sleep(tokio::time::Duration::from_secs(attempt)).await;
                            println!("DEBUG_SYNC: Retrying path: {} (attempt {})", path, attempt);
                            match execute_single_action_sequential(
                                &action,
                                &client,
                                vault_path,
                                database,
                                &remote_entries_list,
                                &mut remote_state,
                                &mut report,
                                &vault_service,
                                &conflict_callback_clone,
                            ).await {
                                Ok(()) => {
                                    success = true;
                                    break;
                                }
                                Err(e) => {
                                    last_err = e;
                                }
                            }
                        }

                        if !success {
                            // Quarantine this entry
                            let conn = database.conn.lock();
                            let db_res = conn.execute(
                                "UPDATE sync_file_states SET retry_count = retry_count + 1, sync_error = ?2, is_dirty = 1 WHERE path = ?1",
                                rusqlite::params![path, last_err.to_string()],
                            );
                            if let Err(ref e) = db_res {
                                if e.to_string().contains("locked") || e.to_string().contains("busy") {
                                    println!("CRITICAL: SQLite update failed during sync micro-commit: {:?}", e);
                                }
                            }
                        }
                    }
                }
            }

            // Update peer signature in local state
            let lm = remote_sync_entry.last_modified.as_ref()
                .and_then(|s| crate::sync::delta::parse_last_modified(s));
            remote_state.devices.insert(other_device.clone(), crate::sync::remote_state::DeviceMetadata {
                last_known_etag: remote_sync_entry.etag.clone(),
                last_known_modified: lm,
            });
            
            // Save state to DB
            let conn = database.conn.lock();
            let _ = save_remote_state(&conn, &remote_state);
        }

        // Execute local actions
        if !local_actions.is_empty() {
            let mut failed_paths = Vec::new();
            if remote_entries_list.is_empty() {
                if let Ok(entries) = client.propfind("", 9).await {
                    remote_entries_list = entries;
                }
            }

            for action in local_actions {
                current_index += 1;
                let path = match &action {
                    SyncAction::Upload { relative_path } => relative_path.clone(),
                    SyncAction::Download { relative_path, .. } => relative_path.clone(),
                    SyncAction::DeleteRemote { relative_path } => relative_path.clone(),
                    SyncAction::DeleteLocal { relative_path } => relative_path.clone(),
                    SyncAction::Conflict { relative_path, .. } => relative_path.clone(),
                };

                let action_str = match &action {
                    SyncAction::Upload { .. } => "upload",
                    SyncAction::Download { .. } => "download",
                    SyncAction::DeleteRemote { .. } => "remote_delete",
                    SyncAction::DeleteLocal { .. } => "local_delete",
                    SyncAction::Conflict { .. } => "conflict",
                };

                self.emit_progress(SyncProgress {
                    status: "Processing".to_string(),
                    action: action_str.to_string(),
                    file_path: path.clone(),
                    current_index,
                    total_count,
                });

                println!("DEBUG_SYNC: Starting network action for path: {}", path);
                match execute_single_action_sequential(
                    &action,
                    &client,
                    vault_path,
                    database,
                    &remote_entries_list,
                    &mut remote_state,
                    &mut report,
                    &vault_service,
                    &conflict_callback_clone,
                ).await {
                    Ok(()) => {
                        println!("DEBUG_SYNC: Network success for path: {}. Proceeding to DB write.", path);
                    }
                    Err(e) => {
                        println!("DEBUG_SYNC: Network failed for path: {}. Error isolated. Proceeding to Quarantine write.", path);
                        failed_paths.push((path, action, e));
                    }
                }
            }

            // Retry circuit for failed uploads
            if !failed_paths.is_empty() {
                for (path, action, mut last_err) in failed_paths {
                    let mut success = false;
                    for attempt in 1..=3 {
                        tokio::time::sleep(tokio::time::Duration::from_secs(attempt)).await;
                        println!("DEBUG_SYNC: Retrying path: {} (attempt {})", path, attempt);
                        match execute_single_action_sequential(
                            &action,
                            &client,
                            vault_path,
                            database,
                            &remote_entries_list,
                            &mut remote_state,
                            &mut report,
                            &vault_service,
                            &conflict_callback_clone,
                        ).await {
                            Ok(()) => {
                                success = true;
                                break;
                            }
                            Err(e) => {
                                last_err = e;
                            }
                        }
                    }

                    if !success {
                        // Quarantine this entry
                        let conn = database.conn.lock();
                        let db_res = conn.execute(
                            "UPDATE sync_file_states SET retry_count = retry_count + 1, sync_error = ?2, is_dirty = 1 WHERE path = ?1",
                            rusqlite::params![path, last_err.to_string()],
                        );
                        if let Err(ref e) = db_res {
                            if e.to_string().contains("locked") || e.to_string().contains("busy") {
                                println!("CRITICAL: SQLite update failed during sync micro-commit: {:?}", e);
                            }
                        }
                    }
                }
            }
        }

        // ----------------- STEP 5: Finalize & Upload Manifest Tree -----------------
        let remaining_dirty_count = {
            let conn = database.conn.lock();
            conn.query_row(
                "SELECT COUNT(*) FROM sync_file_states WHERE is_dirty = 1 AND (retry_count = 0 OR retry_count IS NULL)",
                [],
                |row| row.get::<_, i64>(0)
            ).unwrap_or(0)
        };

        let remote_affected = report.uploads > 0 || report.deletes_remote > 0 || report.conflicts > 0;
        if remaining_dirty_count == 0 && remote_affected {
            let local_manifest = {
                let conn = database.conn.lock();
                let mut stmt = conn
                    .prepare("SELECT path, hash FROM sync_file_states WHERE hash != '' AND NOT (is_dirty = 1 AND retry_count > 0)")
                    .map_err(|e| NodaError::Database(format!("Prepare generate_manifest failed: {}", e)))?;

                let rows = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|e| NodaError::Database(format!("Query generate_manifest failed: {}", e)))?;

                let mut files = std::collections::HashMap::new();
                for row in rows {
                    let (path, hash) = row.map_err(|e| {
                        NodaError::Database(format!("Row parsing failed in generate_manifest: {}", e))
                    })?;
                    let full_path = vault_path.join(&path);
                    if full_path.exists() {
                        files.insert(path, hash);
                    }
                }

                crate::sync::manifest::VaultManifest {
                    generated_at: chrono::Utc::now(),
                    files,
                }
            };

            let manifest_json = serde_json::to_string_pretty(&local_manifest)
                .map_err(|e| NodaError::Sync(format!("Failed to serialize manifest: {}", e)))?;

            let manifest_rel_path = format!(".noda/sync/manifests/manifest_{}.json", config.device_name);
            ensure_remote_parent_dirs_exist(&client, &manifest_rel_path, &remote_entries_list, "").await?;
            client.put(&manifest_rel_path, manifest_json.as_bytes().to_vec()).await?;

            // Save locally
            let local_manifest_path = vault_path.join(".noda/sync/manifests").join(format!("manifest_{}.json", config.device_name));
            if let Some(parent) = local_manifest_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            let _ = tokio::fs::write(&local_manifest_path, manifest_json.as_bytes()).await;

            // Upload 0-byte .sync signature
            let our_sync_file_name = format!("{}.sync", config.device_name);
            let our_sync_rel_path = format!(".noda/sync/{}", our_sync_file_name);
            ensure_remote_parent_dirs_exist(&client, &our_sync_rel_path, &remote_entries_list, "").await?;
            client.put(&our_sync_rel_path, Vec::new()).await?;

            // Capture the server's ETag for our sync file
            if let Ok(mut entries) = client.propfind(&our_sync_rel_path, 0).await {
                if let Some(entry) = entries.pop() {
                    let lm = entry.last_modified.as_ref()
                        .and_then(|s| crate::sync::delta::parse_last_modified(s));
                    remote_state.devices.insert(config.device_name.clone(), crate::sync::remote_state::DeviceMetadata {
                        last_known_etag: entry.etag,
                        last_known_modified: lm,
                    });
                }
            }
        }

        remote_state.last_sync_time = Some(chrono::Utc::now());
        {
            let conn = database.conn.lock();
            save_remote_state(&conn, &remote_state)?;
        }

        self.update_status(SyncStatus::Idle);
        if let Some(cb) = &*self.sync_finished_callback.read() {
            cb(report.clone());
        }
        Ok(report)
    }
}

fn commit_micro_state(
    database: &Database,
    path: &str,
    meta: &RemoteFileMetadata,
) -> Result<(), NodaError> {
    let conn = database.conn.lock();
    let is_dirty_int = if meta.is_dirty { 1 } else { 0 };
    let lm_str = meta.last_modified.map(|dt| dt.to_rfc3339());
    let lu_str = meta.local_updated_at.map(|dt| dt.to_rfc3339());
    
    let mut actual_size = meta.size;
    if let Some(vault_path) = crate::database::queries::get_vault_path_from_conn(&conn) {
        let full_path = vault_path.join(path);
        if let Ok(fs_meta) = std::fs::metadata(&full_path) {
            actual_size = fs_meta.len();
        }
    }

    let res = conn.execute(
        "INSERT INTO sync_file_states (path, etag, last_modified, size, local_updated_at, hash, is_dirty, retry_count, sync_error) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, NULL) \
         ON CONFLICT(path) DO UPDATE SET \
            etag=excluded.etag, \
            last_modified=excluded.last_modified, \
            size=excluded.size, \
            local_updated_at=excluded.local_updated_at, \
            hash=excluded.hash, \
            is_dirty=excluded.is_dirty, \
            retry_count=0, \
            sync_error=NULL",
        rusqlite::params![path, meta.etag, lm_str, actual_size as i64, lu_str, meta.hash, is_dirty_int],
    );

    if let Err(ref e) = res {
        if e.to_string().contains("locked") || e.to_string().contains("busy") {
            println!("CRITICAL: SQLite update failed during sync micro-commit: {:?}", e);
        }
    }
    
    res.map(|_| ()).map_err(|e| NodaError::Database(e.to_string()))
}

fn remove_micro_state(
    database: &Database,
    path: &str,
) -> Result<(), NodaError> {
    let conn = database.conn.lock();
    let res = conn.execute("DELETE FROM sync_file_states WHERE path = ?1", [path]);
    
    if let Err(ref e) = res {
        if e.to_string().contains("locked") || e.to_string().contains("busy") {
            println!("CRITICAL: SQLite update failed during sync micro-commit: {:?}", e);
        }
    }
    
    res.map(|_| ()).map_err(|e| NodaError::Database(e.to_string()))
}

async fn execute_single_action_sequential(
    action: &SyncAction,
    client: &WebDavClient,
    vault_path: &Path,
    database: &Database,
    remote_entries: &[RemoteEntry],
    remote_state: &mut RemoteState,
    report: &mut SyncReport,
    vault_service: &VaultService,
    conflict_callback: &Option<Arc<dyn Fn(ConflictEntry) + Send + Sync + 'static>>,
) -> Result<(), NodaError> {
    match action {
        SyncAction::Upload { relative_path } if relative_path.starts_with(".noda/") => {
            let full_path = vault_path.join(relative_path);
            let bytes = tokio::fs::read(&full_path).await.map_err(NodaError::Io)?;
            ensure_remote_parent_dirs_exist(client, relative_path, remote_entries, "").await?;
            client.put(relative_path, bytes.clone()).await?;

            let raw_hash = xxhash_rust::xxh3::xxh3_64(&bytes);
            let hash_hex = format!("{:016x}", raw_hash);

            let meta = tokio::fs::metadata(&full_path).await.ok();
            let modified = meta.as_ref()
                .and_then(|m| m.modified().ok())
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                .unwrap_or_else(chrono::Utc::now);

            let updated_entry = match client.propfind(relative_path, 0).await {
                Ok(mut entries) => entries.pop(),
                Err(_) => None,
            };

            let file_meta = if let Some(remote_entry) = updated_entry {
                let lm = remote_entry.last_modified.as_ref()
                    .and_then(|s| crate::sync::delta::parse_last_modified(s));
                RemoteFileMetadata {
                    etag: remote_entry.etag,
                    last_modified: lm,
                    size: remote_entry.size.unwrap_or(bytes.len() as u64),
                    local_updated_at: Some(modified),
                    hash: hash_hex,
                    is_dirty: false,
                }
            } else {
                RemoteFileMetadata {
                    etag: None,
                    last_modified: Some(modified),
                    size: bytes.len() as u64,
                    local_updated_at: Some(modified),
                    hash: hash_hex,
                    is_dirty: false,
                }
            };

            remote_state.files.insert(relative_path.clone(), file_meta.clone());
            commit_micro_state(database, relative_path, &file_meta)?;

            // Post-Sync Post-Flush Verification
            if full_path.exists() {
                if let Ok(meta) = std::fs::metadata(&full_path) {
                    let modified = meta.modified()
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                        .unwrap_or_else(|_| chrono::Utc::now());
                    if let Some(fm) = remote_state.files.get_mut(relative_path) {
                        fm.local_updated_at = Some(modified);
                        fm.size = meta.len();
                        let _ = commit_micro_state(database, relative_path, fm);
                    }
                }
            }

            report.uploads += 1;
            report.uploaded_files.push(relative_path.clone());
        }
        SyncAction::Upload { relative_path } => {
            let path_buf = std::path::Path::new(relative_path);
            let note_id_str = path_buf.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let note_id = if let Ok(ulid) = ulid::Ulid::from_string(note_id_str) {
                crate::models::note::NoteId(ulid)
            } else {
                return Ok(());
            };

            let note = vault_service.read_note(note_id).await?;
            let markdown = note.to_markdown().map_err(|e| NodaError::Vault(e.to_string()))?;

            ensure_remote_parent_dirs_exist(client, relative_path, remote_entries, "").await?;
            client.put(relative_path, markdown.as_bytes().to_vec()).await?;

            let full_path = vault_path.join(relative_path);
            let meta = tokio::fs::metadata(&full_path).await.ok();
            let modified = meta.as_ref()
                .and_then(|m| m.modified().ok())
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                .unwrap_or(note.updated_at);

            let note_hash = xxhash_rust::xxh3::xxh3_64(markdown.as_bytes());
            let note_hash_hex = format!("{:016x}", note_hash);

            {
                let db_conn = database.conn.lock();
                crate::database::queries::upsert_note(&db_conn, &note, relative_path, false)?;
            }

            let updated_entry = match client.propfind(relative_path, 0).await {
                Ok(mut entries) => entries.pop(),
                Err(_) => None,
            };

            let file_meta = if let Some(remote_entry) = updated_entry {
                let lm = remote_entry.last_modified.as_ref()
                    .and_then(|s| crate::sync::delta::parse_last_modified(s));
                RemoteFileMetadata {
                    etag: remote_entry.etag,
                    last_modified: lm,
                    size: remote_entry.size.unwrap_or(markdown.len() as u64),
                    local_updated_at: Some(modified),
                    hash: note_hash_hex,
                    is_dirty: false,
                }
            } else {
                RemoteFileMetadata {
                    etag: None,
                    last_modified: Some(modified),
                    size: markdown.len() as u64,
                    local_updated_at: Some(modified),
                    hash: note_hash_hex,
                    is_dirty: false,
                }
            };

            remote_state.files.insert(relative_path.clone(), file_meta.clone());
            commit_micro_state(database, relative_path, &file_meta)?;

            // Post-Sync Post-Flush Verification
            if full_path.exists() {
                if let Ok(meta) = std::fs::metadata(&full_path) {
                    let modified = meta.modified()
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                        .unwrap_or_else(|_| chrono::Utc::now());
                    if let Some(fm) = remote_state.files.get_mut(relative_path) {
                        fm.local_updated_at = Some(modified);
                        fm.size = meta.len();
                        let _ = commit_micro_state(database, relative_path, fm);
                    }
                }
            }

            report.uploads += 1;
            report.uploaded_files.push(relative_path.clone());
        }
        SyncAction::Download { relative_path, remote_entry } if relative_path.starts_with(".noda/") => {
            match client.get(relative_path).await {
                Ok(bytes) => {
                    let full_path = vault_path.join(relative_path);
                    if let Some(parent) = full_path.parent() {
                        tokio::fs::create_dir_all(parent).await.map_err(NodaError::Io)?;
                    }

                    tokio::fs::write(&full_path, &bytes).await.map_err(NodaError::Io)?;

                    let raw_hash = xxhash_rust::xxh3::xxh3_64(&bytes);
                    let hash_hex = format!("{:016x}", raw_hash);

                    let meta = tokio::fs::metadata(&full_path).await.ok();
                    let modified = meta.as_ref()
                        .and_then(|m| m.modified().ok())
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                        .unwrap_or_else(chrono::Utc::now);

                    let lm = remote_entry.last_modified.as_ref()
                        .and_then(|s| crate::sync::delta::parse_last_modified(s));

                    let file_meta = RemoteFileMetadata {
                        etag: remote_entry.etag.clone(),
                        last_modified: lm,
                        size: remote_entry.size.unwrap_or(bytes.len() as u64),
                        local_updated_at: Some(modified),
                        hash: hash_hex,
                        is_dirty: false,
                    };

                    remote_state.files.insert(relative_path.clone(), file_meta.clone());
                    commit_micro_state(database, relative_path, &file_meta)?;

                    // Post-Sync Post-Flush Verification
                    if full_path.exists() {
                        if let Ok(meta) = std::fs::metadata(&full_path) {
                            let modified = meta.modified()
                                .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                                .unwrap_or_else(|_| chrono::Utc::now());
                            if let Some(fm) = remote_state.files.get_mut(relative_path) {
                                fm.local_updated_at = Some(modified);
                                fm.size = meta.len();
                                let _ = commit_micro_state(database, relative_path, fm);
                            }
                        }
                    }

                    report.downloads += 1;
                    report.downloaded_files.push(relative_path.clone());
                }
                Err(NodaError::NotFound(msg)) => {
                    tracing::warn!("Download skipped - Raw file not found on remote: {}. Cleaning up remote state.", msg);
                    remote_state.files.remove(relative_path);
                    remove_micro_state(database, relative_path)?;
                }
                Err(e) => return Err(e),
            }
        }
        SyncAction::Download { relative_path, remote_entry } => {
            match client.get(relative_path).await {
                Ok(bytes) => {
                    let content = String::from_utf8_lossy(&bytes).into_owned();
                    let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
                    let parsed = matter.parse(&content);

                    if let Some(data) = parsed.data {
                        let frontmatter = data.deserialize::<crate::models::note::Frontmatter>()
                            .map_err(|e| NodaError::Sync(format!("Failed to deserialize frontmatter: {}", e)))?;
                        let outline = Some(Note::parse_outline(&parsed.content));
                        let note = Note {
                            id: frontmatter.id,
                            parent_id: frontmatter.parent_id,
                            title: frontmatter.title,
                            inline_tags: Note::parse_inline_tags(&parsed.content),
                            body: parsed.content,
                            color: frontmatter.color,
                            pinned: frontmatter.pinned,
                            tags: frontmatter.tags,
                            status: frontmatter.status,
                            created_at: frontmatter.created_at,
                            updated_at: frontmatter.updated_at,
                            file_path: relative_path.clone(),
                            is_encrypted: frontmatter.is_encrypted,
                            dek_encrypted: frontmatter.dek_encrypted.clone(),
                            dek_nonce: frontmatter.dek_nonce.clone(),
                            outline,
                        };

                        vault_service.write_note(&note).await?;

                        let full_path = vault_path.join(relative_path);
                        let meta = tokio::fs::metadata(&full_path).await.ok();
                        let modified = meta.as_ref()
                            .and_then(|m| m.modified().ok())
                            .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                            .unwrap_or(note.updated_at);

                        let note_hash = xxhash_rust::xxh3::xxh3_64(content.as_bytes());
                        let note_hash_hex = format!("{:016x}", note_hash);

                        {
                            let db_conn = database.conn.lock();
                            crate::database::queries::upsert_note(&db_conn, &note, relative_path, false)?;
                        }

                        let lm = remote_entry.last_modified.as_ref()
                            .and_then(|s| crate::sync::delta::parse_last_modified(s));

                        let file_meta = RemoteFileMetadata {
                            etag: remote_entry.etag.clone(),
                            last_modified: lm,
                            size: remote_entry.size.unwrap_or(bytes.len() as u64),
                            local_updated_at: Some(modified),
                            hash: note_hash_hex,
                            is_dirty: false,
                        };

                        remote_state.files.insert(relative_path.clone(), file_meta.clone());
                        commit_micro_state(database, relative_path, &file_meta)?;

                        // Post-Sync Post-Flush Verification
                        if full_path.exists() {
                            if let Ok(meta) = std::fs::metadata(&full_path) {
                                let modified = meta.modified()
                                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                                    .unwrap_or_else(|_| chrono::Utc::now());
                                if let Some(fm) = remote_state.files.get_mut(relative_path) {
                                    fm.local_updated_at = Some(modified);
                                    fm.size = meta.len();
                                    let _ = commit_micro_state(database, relative_path, fm);
                                }
                            }
                        }

                        report.downloads += 1;
                        report.downloaded_files.push(relative_path.clone());
                    } else {
                        return Err(NodaError::Sync(format!("No frontmatter found in downloaded note: {}", relative_path)));
                    }
                }
                Err(NodaError::NotFound(msg)) => {
                    tracing::warn!("Download skipped - Note not found on remote: {}. Cleaning up remote state.", msg);
                    remote_state.files.remove(relative_path);
                    remove_micro_state(database, relative_path)?;
                }
                Err(e) => return Err(e),
            }
        }
        SyncAction::DeleteRemote { relative_path } => {
            client.delete(relative_path).await?;
            remote_state.files.remove(relative_path);
            remove_micro_state(database, relative_path)?;
            report.deletes_remote += 1;
            report.deleted_remote_files.push(relative_path.clone());
        }
        SyncAction::DeleteLocal { relative_path } if relative_path.starts_with(".noda/") => {
            let full_path = vault_path.join(relative_path);
            if full_path.exists() {
                tokio::fs::remove_file(&full_path).await.map_err(NodaError::Io)?;
            }

            remote_state.files.remove(relative_path);
            remove_micro_state(database, relative_path)?;
            report.deletes_local += 1;
            report.deleted_local_files.push(relative_path.clone());
        }
        SyncAction::DeleteLocal { relative_path } => {
            let full_path = vault_path.join(relative_path);
            if full_path.exists() {
                tokio::fs::remove_file(&full_path).await.map_err(NodaError::Io)?;
            }

            {
                let db_conn = database.conn.lock();
                crate::database::queries::delete_note_by_path(&db_conn, relative_path, false)?;
            }

            remote_state.files.remove(relative_path);
            remove_micro_state(database, relative_path)?;
            report.deletes_local += 1;
            report.deleted_local_files.push(relative_path.clone());
        }
        SyncAction::Conflict { relative_path, local_note, remote_entry: _ } => {
            match client.get(relative_path).await {
                Ok(remote_bytes) => {
                    let conflict_entry = handle_conflict(vault_path, local_note, &remote_bytes).await?;
                    let markdown = local_note.to_markdown().map_err(|e| NodaError::Vault(e.to_string()))?;

                    client.put(relative_path, markdown.as_bytes().to_vec()).await?;

                    let full_path = vault_path.join(relative_path);
                    let meta = tokio::fs::metadata(&full_path).await.ok();
                    let modified = meta.as_ref()
                        .and_then(|m| m.modified().ok())
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                        .unwrap_or(local_note.updated_at);

                    let note_hash = xxhash_rust::xxh3::xxh3_64(markdown.as_bytes());
                    let note_hash_hex = format!("{:016x}", note_hash);

                    let updated_entry = match client.propfind(relative_path, 0).await {
                        Ok(mut entries) => entries.pop(),
                        Err(_) => None,
                    };

                    let file_meta = if let Some(entry) = updated_entry {
                        let lm = entry.last_modified.as_ref()
                            .and_then(|s| crate::sync::delta::parse_last_modified(s));
                        RemoteFileMetadata {
                            etag: entry.etag,
                            last_modified: lm,
                            size: entry.size.unwrap_or(markdown.len() as u64),
                            local_updated_at: Some(modified),
                            hash: note_hash_hex,
                            is_dirty: false,
                        }
                    } else {
                        RemoteFileMetadata {
                            etag: None,
                            last_modified: Some(modified),
                            size: markdown.len() as u64,
                            local_updated_at: Some(modified),
                            hash: note_hash_hex,
                            is_dirty: false,
                        }
                    };

                    remote_state.files.insert(relative_path.clone(), file_meta.clone());
                    commit_micro_state(database, relative_path, &file_meta)?;

                    // Post-Sync Post-Flush Verification
                    if full_path.exists() {
                        if let Ok(meta) = std::fs::metadata(&full_path) {
                            let modified = meta.modified()
                                .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                                .unwrap_or_else(|_| chrono::Utc::now());
                            if let Some(fm) = remote_state.files.get_mut(relative_path) {
                                fm.local_updated_at = Some(modified);
                                fm.size = meta.len();
                                let _ = commit_micro_state(database, relative_path, fm);
                            }
                        }
                    }

                    if let Some(cb) = conflict_callback {
                        cb(conflict_entry.clone());
                    }

                    tracing::info!("Conflict archived and resolved: {:?}", conflict_entry);
                    report.conflicts += 1;
                    report.conflict_files.push(relative_path.clone());
                }
                Err(NodaError::NotFound(msg)) => {
                    tracing::warn!("Conflict download failed with 404 - Remote file no longer exists: {}. Proceeding to upload local note.", msg);
                    let markdown = local_note.to_markdown().map_err(|e| NodaError::Vault(e.to_string()))?;

                    client.put(relative_path, markdown.as_bytes().to_vec()).await?;

                    let full_path = vault_path.join(relative_path);
                    let meta = tokio::fs::metadata(&full_path).await.ok();
                    let modified = meta.as_ref()
                        .and_then(|m| m.modified().ok())
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                        .unwrap_or(local_note.updated_at);

                    let note_hash = xxhash_rust::xxh3::xxh3_64(markdown.as_bytes());
                    let note_hash_hex = format!("{:016x}", note_hash);

                    let updated_entry = match client.propfind(relative_path, 0).await {
                        Ok(mut entries) => entries.pop(),
                        Err(_) => None,
                    };

                    let file_meta = if let Some(entry) = updated_entry {
                        let lm = entry.last_modified.as_ref()
                            .and_then(|s| crate::sync::delta::parse_last_modified(s));
                        RemoteFileMetadata {
                            etag: entry.etag,
                            last_modified: lm,
                            size: entry.size.unwrap_or(markdown.len() as u64),
                            local_updated_at: Some(modified),
                            hash: note_hash_hex,
                            is_dirty: false,
                        }
                    } else {
                        RemoteFileMetadata {
                            etag: None,
                            last_modified: Some(modified),
                            size: markdown.len() as u64,
                            local_updated_at: Some(modified),
                            hash: note_hash_hex,
                            is_dirty: false,
                        }
                    };

                    remote_state.files.insert(relative_path.clone(), file_meta.clone());
                    commit_micro_state(database, relative_path, &file_meta)?;
                }
                Err(e) => return Err(e),
            }
        }
    }
    Ok(())
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
            device_name: "test-device".to_string(),
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
                            let body = if req_str.contains(".noda/sync") {
                                // Request for device signatures under .noda/sync
                                concat!(
                                    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
                                    "<d:multistatus xmlns:d=\"DAV:\">\n",
                                    "  <d:response>\n",
                                    "    <d:href>/.noda/sync/</d:href>\n",
                                    "    <d:propstat>\n",
                                    "      <d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop>\n",
                                    "      <d:status>HTTP/1.1 200 OK</d:status>\n",
                                    "    </d:propstat>\n",
                                    "  </d:response>\n",
                                    "  <d:response>\n",
                                    "    <d:href>/.noda/sync/device2.sync</d:href>\n",
                                    "    <d:propstat>\n",
                                    "      <d:prop>\n",
                                    "        <d:resourcetype/>\n",
                                    "        <d:getcontentlength>0</d:getcontentlength>\n",
                                    "        <d:getlastmodified>Wed, 20 May 2026 03:00:00 GMT</d:getlastmodified>\n",
                                    "        <d:getetag>\"etag-device2-sync\"</d:getetag>\n",
                                    "      </d:prop>\n",
                                    "      <d:status>HTTP/1.1 200 OK</d:status>\n",
                                    "    </d:propstat>\n",
                                    "  </d:response>\n",
                                    "</d:multistatus>"
                                )
                            } else {
                                // Fallback
                                concat!(
                                    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
                                    "<d:multistatus xmlns:d=\"DAV:\">\n",
                                    "  <d:response>\n",
                                    "    <d:href>/</d:href>\n",
                                    "    <d:propstat>\n",
                                    "      <d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop>\n",
                                    "      <d:status>HTTP/1.1 200 OK</d:status>\n",
                                    "    </d:propstat>\n",
                                    "  </d:response>\n",
                                    "</d:multistatus>"
                                )
                            };
                            let response = format!(
                                "HTTP/1.1 207 Multi-Status\r\nContent-Type: text/xml; charset=\"utf-8\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        } else if req_str.starts_with("GET") {
                            let (body, content_type) = if req_str.contains("manifest_device2.json") {
                                (
                                    concat!(
                                        "{\n",
                                        "  \"generated_at\": \"2026-05-20T03:00:00Z\",\n",
                                        "  \"files\": {\n",
                                        "    \"01H7V18105M5MWRB28Z1220000.md\": \"some_xxh3_hash\"\n",
                                        "  }\n",
                                        "}"
                                    ),
                                    "application/json"
                                )
                            } else {
                                (
                                    concat!(
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
                                    ),
                                    "text/markdown"
                                )
                            };
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                content_type,
                                body.len(),
                                body
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        } else if req_str.starts_with("PUT") {
                            let response = "HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                            let _ = socket.write_all(response.as_bytes()).await;
                        } else if req_str.starts_with("MKCOL") {
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
            device_name: "test-device".to_string(),
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
