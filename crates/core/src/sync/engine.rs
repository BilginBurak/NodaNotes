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
use crate::sync::remote_state::{load_remote_state, save_remote_state, RemoteState, RemoteFileMetadata};
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

        // Step 1: Load remote state (cheap disk read) — done first to enable fast-check
        let mut remote_state = load_remote_state(vault_path).await.unwrap_or_default();

        // Step 2: Scan local notes and raw files
        let local_notes = match scan_vault(vault_path).await {
            Ok(notes) => notes,
            Err(e) => {
                self.update_status(SyncStatus::Error(format!("Local vault scan failed: {}", e)));
                return Err(e);
            }
        };

        // Raw files: attachments + history (history is excluded from dirty-check, but used in raw_plan)
        let local_raw = scan_local_raw_files(vault_path).await.unwrap_or_default();

        // Step 3: Fast-Check — check for local changes BEFORE any expensive operations
        let local_changed = has_local_changes(&local_notes, &local_raw, &remote_state);

        // Step 3: Fast-Check & Remote Device Pruning
        // Query the remote sync directory first to reconcile active remote devices and run fast-check
        let sync_dir_path = ".noda/sync";
        let mut active_remote_devices = std::collections::HashSet::new();
        let mut pruning_happened = false;

        let remote_sync_entries_res = client.propfind(sync_dir_path, 1).await;
        if let Ok(ref remote_sync_entries) = remote_sync_entries_res {
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
                    active_remote_devices.insert(device_name);
                }
            }

            // Prune local cached devices that are no longer active on the WebDAV server
            let local_device_keys: Vec<String> = remote_state.devices.keys().cloned().collect();
            for cached_device in local_device_keys {
                if cached_device != config.device_name && !active_remote_devices.contains(&cached_device) {
                    tracing::info!("Pruning ghost device {} from local state", cached_device);
                    remote_state.devices.remove(&cached_device);
                    pruning_happened = true;
                }
            }

            if pruning_happened {
                let _ = save_remote_state(vault_path, &remote_state).await;
            }
        }

        if !local_changed {
            tracing::info!("No local changes detected. Initiating Fast-Check remote validation.");
            match &remote_sync_entries_res {
                Ok(remote_sync_entries) => {
                    let mut fast_check_ok = true;
                    let mut remote_other_sync_paths = std::collections::HashSet::new();

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
                            // Skip our own device — we only care about OTHER devices' signatures
                            if device_name == config.device_name {
                                continue;
                            }
                            remote_other_sync_paths.insert(device_name.clone());

                            if let Some(cached_device) = remote_state.devices.get(&device_name) {
                                // Primary check: ETag equality (trimmed of quotes)
                                let etag_matches = match (&entry.etag, &cached_device.last_known_etag) {
                                    (Some(e1), Some(e2)) => e1.trim_matches('"') == e2.trim_matches('"'),
                                    _ => false,
                                };

                                // Secondary check: Last-Modified equality (strict == match for server clocks)
                                let lm_matches = if !etag_matches {
                                    let entry_lm = entry.last_modified.as_ref()
                                        .and_then(|s| crate::sync::delta::parse_last_modified(s));
                                    match (entry_lm, cached_device.last_known_modified) {
                                        (Some(t1), Some(t2)) => t1 == t2,
                                        _ => false,
                                    }
                                } else {
                                    true
                                };

                                if !etag_matches && !lm_matches {
                                    tracing::info!(
                                        "fast-check: remote device {} signature mismatch \
                                        (etag matches: {}, lm matches: {}), bypassing",
                                        device_name, etag_matches, lm_matches
                                    );
                                    fast_check_ok = false;
                                    break;
                                }
                            } else {
                                tracing::info!("fast-check: new remote device found: {}, bypassing", device_name);
                                fast_check_ok = false;
                                break;
                            }
                        }
                    }

                    if fast_check_ok {
                        for (device_name, _) in &remote_state.devices {
                            if device_name != &config.device_name && !remote_other_sync_paths.contains(device_name) {
                                tracing::info!("Cached remote device {} sync file no longer exists, bypassing fast-check", device_name);
                                fast_check_ok = false;
                                break;
                            }
                        }
                    }

                    if fast_check_ok {
                        tracing::info!("Fast-check succeeded. No local or remote changes. Early exit.");
                        self.update_status(SyncStatus::Idle);
                        if let Some(cb) = &*self.sync_finished_callback.read() {
                            cb(SyncReport::default());
                        }
                        return Ok(SyncReport::default());
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch remote sync directory for Fast-Check: {}. Proceeding to full sync.", e);
                }
            }
        }

        // Step 4: Fast-check bypassed — take pre-sync snapshots for modified notes before reconciliation.
        // This is intentionally done AFTER fast-check to avoid unnecessary snapshot creation on idle cycles.
        for note in &local_notes {
            if let Ok(snaps) = crate::history::list_snapshots(vault_path, note.id).await {
                let needs_snapshot = if let Some(latest_snap) = snaps.first() {
                    if let Ok(restored_note) = crate::history::restore(vault_path, latest_snap).await {
                        note.body != restored_note.body || note.title != restored_note.title || note.tags != restored_note.tags
                    } else {
                        true
                    }
                } else {
                    true
                };

                if needs_snapshot {
                    let _ = crate::history::snapshot(vault_path, note, "Pre-Sync").await;
                }
            }
        }

        let mut sync_queue = SyncQueue::load(vault_path).await?;
        let vault_service = VaultService::new(vault_path).map_err(|e| NodaError::Vault(e.to_string()))?;

        // Step 5: Full scan & traversal
        let remote_entries = match list_remote_tree(&client, "").await {
            Ok(entries) => entries,
            Err(e) => {
                self.update_status(SyncStatus::Error(format!("Remote traversal failed: {}", e)));
                return Err(e);
            }
        };

        // Re-scan local raw files to capture the newly taken Pre-Sync snapshots
        let local_raw = scan_local_raw_files(vault_path).await.unwrap_or_default();

        // Calculate delta plans
        let note_plan = calculate_delta(&local_notes, &remote_entries, &remote_state, "");
        let raw_plan = calculate_raw_delta(&local_raw, &remote_entries, &remote_state, "");

        // Queue all actions in a single batch
        let all_actions: Vec<SyncAction> = note_plan.actions.into_iter().chain(raw_plan.actions.into_iter()).collect();
        if !all_actions.is_empty() {
            sync_queue.enqueue_batch(all_actions).await?;
        }

        let mut report = SyncReport::default();
        let entries = sync_queue.take_entries();

        if !entries.is_empty() {
            let client_arc = Arc::new(WebDavClient::new(
                &config.webdav_url,
                &config.webdav_username,
                config.webdav_password.as_deref().unwrap_or(""),
            )?);

            let vault_path_arc = Arc::new(vault_path.to_path_buf());
            let database_clone = database.clone();
            let remote_entries_arc = Arc::new(remote_entries.clone());
            let remote_state_arc = Arc::new(parking_lot::Mutex::new(remote_state.clone()));
            let report_arc = Arc::new(parking_lot::Mutex::new(SyncReport::default()));
            let vault_service_arc = Arc::new(vault_service);
            let conflict_callback_clone = self.conflict_callback.read().clone();

            let mut join_set = tokio::task::JoinSet::new();
            let concurrency_limit = 10;
            let mut active_tasks = 0;
            let mut failed_entries = Vec::new();
            let mut sync_error: Option<NodaError> = None;

            for entry in entries {
                if active_tasks >= concurrency_limit {
                    if let Some(res) = join_set.join_next().await {
                        active_tasks -= 1;
                        match res {
                            Ok(Ok(())) => {}
                            Ok(Err((failed_entry, path, err))) => {
                                tracing::error!("Action failed for {}: {}", path, err);
                                failed_entries.push(failed_entry);
                                if sync_error.is_none() {
                                    sync_error = Some(err);
                                }
                            }
                            Err(join_err) => {
                                tracing::error!("Tokio join error: {}", join_err);
                            }
                        }
                    }
                }

                if sync_error.is_some() {
                    failed_entries.push(entry);
                    continue;
                }

                let client_c = client_arc.clone();
                let vp_c = vault_path_arc.clone();
                let db_c = database_clone.clone();
                let re_c = remote_entries_arc.clone();
                let rs_c = remote_state_arc.clone();
                let rep_c = report_arc.clone();
                let vs_c = vault_service_arc.clone();
                let cc_c = conflict_callback_clone.clone();

                join_set.spawn(async move {
                    let relative_path_for_error = match &entry.action {
                        SyncAction::Upload { relative_path } => relative_path.clone(),
                        SyncAction::Download { relative_path, .. } => relative_path.clone(),
                        SyncAction::DeleteRemote { relative_path } => relative_path.clone(),
                        SyncAction::DeleteLocal { relative_path } => relative_path.clone(),
                        SyncAction::Conflict { relative_path, .. } => relative_path.clone(),
                    };

                    let res = execute_single_action(
                        &entry.action,
                        &client_c,
                        &vp_c,
                        &db_c,
                        &re_c,
                        &rs_c,
                        &rep_c,
                        &vs_c,
                        &cc_c,
                    ).await;

                    match res {
                        Ok(()) => Ok(()),
                        Err(e) => Err((entry, relative_path_for_error, e)),
                    }
                });
                active_tasks += 1;
            }

            while let Some(res) = join_set.join_next().await {
                match res {
                    Ok(Ok(())) => {}
                    Ok(Err((failed_entry, path, err))) => {
                        tracing::error!("Action failed for {}: {}", path, err);
                        failed_entries.push(failed_entry);
                        if sync_error.is_none() {
                            sync_error = Some(err);
                        }
                    }
                    Err(join_err) => {
                        tracing::error!("Tokio join error: {}", join_err);
                    }
                }
            }

            remote_state = Arc::try_unwrap(remote_state_arc).unwrap().into_inner();
            report = Arc::try_unwrap(report_arc).unwrap().into_inner();

            if !failed_entries.is_empty() {
                if let Err(eq_err) = sync_queue.set_entries(failed_entries).await {
                    tracing::error!("Failed to re-enqueue failed actions: {}", eq_err);
                }
                if let Some(err) = sync_error {
                    self.update_status(SyncStatus::Error(format!("Sync failed: {}", err)));
                    return Err(err);
                }
            } else {
                if let Err(clear_err) = sync_queue.set_entries(Vec::new()).await {
                    tracing::error!("Failed to clear sync queue on completion: {}", clear_err);
                }
            }
        }

        // For any files that were already identical and had no action, ensure they are in remote_state
        let local_map: std::collections::HashMap<String, &Note> = local_notes
            .iter()
            .map(|n| (n.file_path.clone(), n))
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

        // For any raw/attachment files that were already identical and had no action, ensure they are in remote_state
        for local_raw_file in &local_raw {
            let path = &local_raw_file.relative_path;
            if !remote_state.files.contains_key(path) {
                if let Some(remote_entry) = remote_map.get(path) {
                    let remote_size = remote_entry.size.unwrap_or(0);
                    // Attachments are immutable; if size matches, they are identical.
                    if local_raw_file.size == remote_size {
                        let lm = remote_entry.last_modified.as_ref()
                            .and_then(|s| crate::sync::delta::parse_last_modified(s));
                        remote_state.files.insert(path.clone(), RemoteFileMetadata {
                            etag: remote_entry.etag.clone(),
                            last_modified: lm,
                            size: remote_size,
                            local_updated_at: Some(local_raw_file.modified),
                        });
                    }
                }
            }
        }

        // Upload our own empty (0-byte) .sync file only if local changes were made/uploaded
        if local_changed {
            let our_sync_file_name = format!("{}.sync", config.device_name);
            let our_sync_rel_path = format!(".noda/sync/{}", our_sync_file_name);

            if let Err(e) = client.put(&our_sync_rel_path, Vec::new()).await {
                tracing::error!("Failed to upload our sync file: {}", e);
            } else {
                // Fetch our newly uploaded .sync file ETag using propfind depth 0
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
        }

        // Fetch all remote .sync signatures and update remote_state.devices
        // Fetch all remote .sync signatures, update remote_state.devices, and prune missing ones
        if let Ok(remote_sync_entries) = client.propfind(".noda/sync", 1).await {
            let mut active_remote_devices = std::collections::HashSet::new();

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
                    active_remote_devices.insert(device_name.clone());

                    let lm = entry.last_modified.as_ref()
                        .and_then(|s| crate::sync::delta::parse_last_modified(s));

                    remote_state.devices.insert(device_name, crate::sync::remote_state::DeviceMetadata {
                        last_known_etag: entry.etag,
                        last_known_modified: lm,
                    });
                }
            }

            // Prune local cached devices not found on WebDAV
            let local_device_keys: Vec<String> = remote_state.devices.keys().cloned().collect();
            for cached_device in local_device_keys {
                if cached_device != config.device_name && !active_remote_devices.contains(&cached_device) {
                    tracing::info!("Post-sync: pruning ghost device {} from local state", cached_device);
                    remote_state.devices.remove(&cached_device);
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

fn has_local_changes(
    local_notes: &[Note],
    local_raw: &[LocalRawFile],
    remote_state: &RemoteState,
) -> bool {
    if remote_state.last_sync_time.is_none() {
        tracing::debug!("has_local_changes: no last_sync_time, returning true");
        return true;
    }

    let mut local_files = std::collections::HashSet::new();

    // Check note files for modifications.
    // We use a 3-second threshold to account for:
    // - WebDAV HTTP dates have 1-second granularity (RFC 2822)
    // - Server/client clock skew
    // - Sub-second precision loss when timestamps round-trip through HTTP headers
    const TIMESTAMP_THRESHOLD_SECS: i64 = 3;

    for note in local_notes {
        local_files.insert(note.file_path.clone());
        if let Some(meta) = remote_state.files.get(&note.file_path) {
            // ETag is authoritative — if it exists and matches our stored version,
            // the file content on the server matches what we last uploaded.
            // No need to check timestamps in that case.
            if let Some(stored_at) = meta.local_updated_at {
                let diff = note.updated_at.signed_duration_since(stored_at).num_seconds().abs();
                if diff > TIMESTAMP_THRESHOLD_SECS {
                    tracing::debug!(
                        "has_local_changes: note {} changed (updated_at diff={}s > {}s threshold)",
                        note.file_path, diff, TIMESTAMP_THRESHOLD_SECS
                    );
                    return true;
                }
            } else {
                tracing::debug!("has_local_changes: note {} has no local_updated_at in state", note.file_path);
                return true;
            }
        } else {
            tracing::debug!("has_local_changes: note {} not in remote_state.files (new local note)", note.file_path);
            return true;
        }
    }

    // Check attachment files only (NOT history files — they are created on every note save
    // and checking them causes fast-check to always be bypassed).
    for raw in local_raw {
        if raw.relative_path.starts_with(".noda/history/") {
            continue;
        }
        local_files.insert(raw.relative_path.clone());
        if let Some(meta) = remote_state.files.get(&raw.relative_path) {
            if let Some(last_saved) = meta.local_updated_at {
                let diff = raw.modified.signed_duration_since(last_saved).num_seconds().abs();
                if diff > TIMESTAMP_THRESHOLD_SECS {
                    tracing::debug!(
                        "has_local_changes: raw file {} changed (modified diff={}s > {}s threshold)",
                        raw.relative_path, diff, TIMESTAMP_THRESHOLD_SECS
                    );
                    return true;
                }
            } else {
                tracing::debug!("has_local_changes: raw file {} has no local_updated_at", raw.relative_path);
                return true;
            }
        } else {
            tracing::debug!("has_local_changes: raw file {} not in remote_state.files (new local file)", raw.relative_path);
            return true;
        }
    }

    // Check for files that existed remotely but are now deleted locally.
    // Skip our own sync control files and history files from deletion detection.
    for (path, _) in &remote_state.files {
        if path.starts_with(".noda/sync/") || path.starts_with(".noda/history/") {
            continue;
        }
        if !local_files.contains(path) {
            tracing::debug!("has_local_changes: remote file {} no longer exists locally (deleted)", path);
            return true;
        }
    }

    tracing::debug!("has_local_changes: no local changes detected — fast-check eligible");
    false
}

async fn execute_single_action(
    action: &SyncAction,
    client: &WebDavClient,
    vault_path: &Path,
    database: &Database,
    remote_entries: &[RemoteEntry],
    remote_state: &parking_lot::Mutex<RemoteState>,
    report: &parking_lot::Mutex<SyncReport>,
    vault_service: &VaultService,
    conflict_callback: &Option<Arc<dyn Fn(ConflictEntry) + Send + Sync + 'static>>,
) -> Result<(), NodaError> {
    match action {
        SyncAction::Upload { relative_path } if relative_path.starts_with(".noda/") => {
            let full_path = vault_path.join(relative_path);
            match tokio::fs::read(&full_path).await {
                Ok(bytes) => {
                    ensure_remote_parent_dirs_exist(client, relative_path, remote_entries, "").await?;
                    client.put(relative_path, bytes.clone()).await?;

                    let meta = tokio::fs::metadata(&full_path).await.ok();
                    let modified = meta.as_ref()
                        .and_then(|m| m.modified().ok())
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                        .unwrap_or_else(chrono::Utc::now);

                    let updated_entry = match client.propfind(relative_path, 0).await {
                        Ok(mut entries) => entries.pop(),
                        Err(_) => None,
                    };

                    {
                        let mut state_guard = remote_state.lock();
                        if let Some(remote_entry) = updated_entry {
                            let lm = remote_entry.last_modified.as_ref()
                                .and_then(|s| crate::sync::delta::parse_last_modified(s));
                            state_guard.files.insert(relative_path.clone(), RemoteFileMetadata {
                                etag: remote_entry.etag,
                                last_modified: lm,
                                size: remote_entry.size.unwrap_or(bytes.len() as u64),
                                local_updated_at: Some(modified),
                            });
                        } else {
                            state_guard.files.insert(relative_path.clone(), RemoteFileMetadata {
                                etag: None,
                                last_modified: Some(modified),
                                size: bytes.len() as u64,
                                local_updated_at: Some(modified),
                            });
                        }
                    }

                    {
                        let mut report_guard = report.lock();
                        report_guard.uploads += 1;
                        report_guard.uploaded_files.push(relative_path.clone());
                    }
                }
                Err(e) => {
                    tracing::warn!("Raw file {} was deleted locally before upload: {:?}", relative_path, e);
                }
            }
        }
        SyncAction::Upload { relative_path } => {
            let path_buf = std::path::Path::new(relative_path);
            let note_id_str = path_buf.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let note_id = if let Ok(ulid) = ulid::Ulid::from_string(note_id_str) {
                crate::models::note::NoteId(ulid)
            } else {
                return Ok(());
            };

            match vault_service.read_note(note_id).await {
                Ok(note) => {
                    let markdown = note.to_markdown()
                        .map_err(|e| NodaError::Vault(e.to_string()))?;

                    ensure_remote_parent_dirs_exist(client, relative_path, remote_entries, "").await?;
                    client.put(relative_path, markdown.as_bytes().to_vec()).await?;

                    let updated_entry = match client.propfind(relative_path, 0).await {
                        Ok(mut entries) => entries.pop(),
                        Err(_) => None,
                    };

                    {
                        let mut state_guard = remote_state.lock();
                        if let Some(remote_entry) = updated_entry {
                            let lm = remote_entry.last_modified.as_ref()
                                .and_then(|s| crate::sync::delta::parse_last_modified(s));
                            state_guard.files.insert(relative_path.clone(), RemoteFileMetadata {
                                etag: remote_entry.etag,
                                last_modified: lm,
                                size: remote_entry.size.unwrap_or(markdown.len() as u64),
                                local_updated_at: Some(note.updated_at),
                            });
                        } else {
                            state_guard.files.insert(relative_path.clone(), RemoteFileMetadata {
                                etag: None,
                                last_modified: Some(note.updated_at),
                                size: markdown.len() as u64,
                                local_updated_at: Some(note.updated_at),
                            });
                        }
                    }

                    {
                        let mut report_guard = report.lock();
                        report_guard.uploads += 1;
                        report_guard.uploaded_files.push(relative_path.clone());
                    }
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
                    if let Some(parent) = full_path.parent() {
                        tokio::fs::create_dir_all(parent).await.map_err(NodaError::Io)?;
                    }

                    tokio::fs::write(&full_path, &bytes).await.map_err(NodaError::Io)?;

                    let meta = tokio::fs::metadata(&full_path).await.ok();
                    let modified = meta.as_ref()
                        .and_then(|m| m.modified().ok())
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                        .unwrap_or_else(chrono::Utc::now);

                    let lm = remote_entry.last_modified.as_ref()
                        .and_then(|s| crate::sync::delta::parse_last_modified(s));

                    {
                        let mut state_guard = remote_state.lock();
                        state_guard.files.insert(relative_path.clone(), RemoteFileMetadata {
                            etag: remote_entry.etag.clone(),
                            last_modified: lm,
                            size: remote_entry.size.unwrap_or(bytes.len() as u64),
                            local_updated_at: Some(modified),
                        });
                    }

                    {
                        let mut report_guard = report.lock();
                        report_guard.downloads += 1;
                        report_guard.downloaded_files.push(relative_path.clone());
                    }
                }
                Err(NodaError::NotFound(msg)) => {
                    tracing::warn!("Download skipped - Raw file not found on remote: {}. Cleaning up remote state.", msg);
                    remote_state.lock().files.remove(relative_path);
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
                        match data.deserialize::<crate::models::note::Frontmatter>() {
                            Ok(frontmatter) => {
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
                                };

                                vault_service.write_note(&note).await?;

                                let db_result = {
                                    let db_conn = database.conn.lock();
                                    crate::database::queries::upsert_note(&db_conn, &note, relative_path, "dummy_hash")
                                };
                                db_result?;

                                let lm = remote_entry.last_modified.as_ref()
                                    .and_then(|s| crate::sync::delta::parse_last_modified(s));

                                {
                                    let mut state_guard = remote_state.lock();
                                    state_guard.files.insert(relative_path.clone(), RemoteFileMetadata {
                                        etag: remote_entry.etag.clone(),
                                        last_modified: lm,
                                        size: remote_entry.size.unwrap_or(bytes.len() as u64),
                                        local_updated_at: Some(note.updated_at),
                                    });
                                }

                                {
                                    let mut report_guard = report.lock();
                                    report_guard.downloads += 1;
                                    report_guard.downloaded_files.push(relative_path.clone());
                                }
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
                    remote_state.lock().files.remove(relative_path);
                }
                Err(e) => return Err(e),
            }
        }
        SyncAction::DeleteRemote { relative_path } => {
            client.delete(relative_path).await?;
            remote_state.lock().files.remove(relative_path);
            {
                let mut report_guard = report.lock();
                report_guard.deletes_remote += 1;
                report_guard.deleted_remote_files.push(relative_path.clone());
            }
        }
        SyncAction::DeleteLocal { relative_path } if relative_path.starts_with(".noda/") => {
            let full_path = vault_path.join(relative_path);
            if full_path.exists() {
                tokio::fs::remove_file(&full_path).await.map_err(NodaError::Io)?;
            }

            remote_state.lock().files.remove(relative_path);
            {
                let mut report_guard = report.lock();
                report_guard.deletes_local += 1;
                report_guard.deleted_local_files.push(relative_path.clone());
            }
        }
        SyncAction::DeleteLocal { relative_path } => {
            let full_path = vault_path.join(relative_path);
            if full_path.exists() {
                tokio::fs::remove_file(&full_path).await.map_err(NodaError::Io)?;
            }

            let db_result = {
                let db_conn = database.conn.lock();
                crate::database::queries::delete_note_by_path(&db_conn, relative_path)
            };
            db_result?;

            remote_state.lock().files.remove(relative_path);
            {
                let mut report_guard = report.lock();
                report_guard.deletes_local += 1;
                report_guard.deleted_local_files.push(relative_path.clone());
            }
        }
        SyncAction::Conflict { relative_path, local_note, remote_entry: _ } => {
            match client.get(relative_path).await {
                Ok(remote_bytes) => {
                    let conflict_entry = handle_conflict(vault_path, local_note, &remote_bytes).await?;
                    let markdown = local_note.to_markdown()
                        .map_err(|e| NodaError::Vault(e.to_string()))?;

                    client.put(relative_path, markdown.as_bytes().to_vec()).await?;

                    let updated_entry = match client.propfind(relative_path, 0).await {
                        Ok(mut entries) => entries.pop(),
                        Err(_) => None,
                    };

                    {
                        let mut state_guard = remote_state.lock();
                        if let Some(entry) = updated_entry {
                            let lm = entry.last_modified.as_ref()
                                .and_then(|s| crate::sync::delta::parse_last_modified(s));
                            state_guard.files.insert(relative_path.clone(), RemoteFileMetadata {
                                etag: entry.etag,
                                last_modified: lm,
                                size: entry.size.unwrap_or(markdown.len() as u64),
                                local_updated_at: Some(local_note.updated_at),
                            });
                        } else {
                            state_guard.files.insert(relative_path.clone(), RemoteFileMetadata {
                                etag: None,
                                last_modified: Some(local_note.updated_at),
                                size: markdown.len() as u64,
                                local_updated_at: Some(local_note.updated_at),
                            });
                        }
                    }

                    if let Some(cb) = conflict_callback {
                        cb(conflict_entry.clone());
                    }

                    tracing::info!("Conflict archived and resolved: {:?}", conflict_entry);
                    {
                        let mut report_guard = report.lock();
                        report_guard.conflicts += 1;
                        report_guard.conflict_files.push(relative_path.clone());
                    }
                }
                Err(NodaError::NotFound(msg)) => {
                    tracing::warn!("Conflict download failed with 404 - Remote file no longer exists: {}. Proceeding to upload local note.", msg);
                    let markdown = local_note.to_markdown()
                        .map_err(|e| NodaError::Vault(e.to_string()))?;

                    client.put(relative_path, markdown.as_bytes().to_vec()).await?;

                    let updated_entry = match client.propfind(relative_path, 0).await {
                        Ok(mut entries) => entries.pop(),
                        Err(_) => None,
                    };

                    {
                        let mut state_guard = remote_state.lock();
                        if let Some(entry) = updated_entry {
                            let lm = entry.last_modified.as_ref()
                                .and_then(|s| crate::sync::delta::parse_last_modified(s));
                            state_guard.files.insert(relative_path.clone(), RemoteFileMetadata {
                                etag: entry.etag,
                                last_modified: lm,
                                size: entry.size.unwrap_or(markdown.len() as u64),
                                local_updated_at: Some(local_note.updated_at),
                            });
                        } else {
                            state_guard.files.insert(relative_path.clone(), RemoteFileMetadata {
                                etag: None,
                                last_modified: Some(local_note.updated_at),
                                size: markdown.len() as u64,
                                local_updated_at: Some(local_note.updated_at),
                            });
                        }
                    }
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

async fn scan_local_raw_files<P: AsRef<Path>>(vault_path: P) -> Result<Vec<LocalRawFile>, NodaError> {
    let mut files = Vec::new();
    let vault_ref = vault_path.as_ref();

    // Scan directories for raw files to sync.
    // NOTE: .noda/history/ is intentionally EXCLUDED from this list.
    // History snapshot files are created on every note save and including them here
    // would cause has_local_changes() to always return true, permanently bypassing
    // fast-check. History files are still included in the raw_plan via delta.rs when
    // a full sync is triggered for other legitimate reasons (note content changes).
    let dirs_to_scan = [
        vault_ref.join(".noda").join("attachments"),
        vault_ref.join(".noda").join("history"),  // Kept for full-sync raw_plan calculation
    ];

    for dir in &dirs_to_scan {
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
