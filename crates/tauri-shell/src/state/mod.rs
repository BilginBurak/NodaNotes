use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use noda_core::database::connection::Database;
use noda_core::vault::service::VaultService;
use noda_core::watcher::VaultWatcher;
use noda_core::sync::SyncEngine;
use noda_core::errors::NodaError;
use tauri::AppHandle;

/// Thread-safe global application state managed by Tauri
#[derive(Clone)]
pub struct AppState {
    pub vault_path: Arc<RwLock<Option<PathBuf>>>,
    pub database: Arc<RwLock<Option<Database>>>,
    pub vault_service: Arc<RwLock<Option<VaultService>>>,
    pub watcher: Arc<RwLock<Option<VaultWatcher>>>,
    pub sync_engine: Arc<RwLock<Option<SyncEngine>>>,
    pub active_note_id: Arc<RwLock<Option<noda_core::models::note::NoteId>>>,
    pub daemon_token: String,
}

impl Default for AppState {
    fn default() -> Self {
        let token = noda_core::vault::persistence::get_or_create_daemon_token_sync()
            .unwrap_or_else(|_| uuid::Uuid::new_v4().simple().to_string());
        Self {
            vault_path: Arc::new(RwLock::new(None)),
            database: Arc::new(RwLock::new(None)),
            vault_service: Arc::new(RwLock::new(None)),
            watcher: Arc::new(RwLock::new(None)),
            sync_engine: Arc::new(RwLock::new(None)),
            active_note_id: Arc::new(RwLock::new(None)),
            daemon_token: token,
        }
    }
}

impl AppState {
    /// Initializes all services for a given vault path
    pub async fn init_vault<P: AsRef<Path>>(&self, path: P, app_handle: AppHandle) -> Result<(), NodaError> {
        let path = path.as_ref().to_path_buf();
        let canonical_path = path.canonicalize().map_err(NodaError::Io)?;

        // 1. Teardown existing services if active
        let engine_to_stop = {
            let mut engine_lock = self.sync_engine.write();
            engine_lock.take()
        };
        if let Some(engine) = engine_to_stop {
            let _ = engine.stop_sync().await;
        }
        {
            let mut watcher_lock = self.watcher.write();
            if let Some(watcher) = watcher_lock.take() {
                drop(watcher);
            }
        }

        // 2. Open or Rebuild Database & VaultService
        let db = Database::open_or_rebuild(&canonical_path).await?;
        let service = VaultService::new(&canonical_path)
            .map_err(|e| NodaError::Vault(format!("Failed to initialize VaultService: {}", e)))?;

        // 3. Start File Watcher
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);
        let watcher = VaultWatcher::start(&canonical_path, event_tx)?;

        // 4. Spawn background event processor to sync disk changes with DB and emit to frontend
        let app_handle_clone = app_handle.clone();
        let conn_clone = db.conn.clone();
        let canonical_path_clone = canonical_path.clone();

        tauri::async_runtime::spawn(async move {
            use noda_core::watcher::sync::sync_batch_with_db;
            use crate::events::emit_vault_updated;
            
            while let Some(batch) = event_rx.recv().await {
                // Sync with DB
                sync_batch_with_db(&canonical_path_clone, conn_clone.clone(), batch.clone()).await;
                
                // Emit event to frontend
                let mut payload = std::collections::HashMap::new();
                for (file_path, state) in batch {
                    let rel_path = file_path
                        .strip_prefix(&canonical_path_clone)
                        .unwrap_or(&file_path)
                        .to_string_lossy()
                        .to_string();
                        
                    let state_str = match state {
                        noda_core::watcher::BatchState::Created => "Created",
                        noda_core::watcher::BatchState::Modified => "Modified",
                        noda_core::watcher::BatchState::Deleted => "Deleted",
                        noda_core::watcher::BatchState::Renamed(_) => "Renamed",
                    };
                    payload.insert(rel_path, state_str.to_string());
                }
                emit_vault_updated(&app_handle_clone, payload);
            }
        });

        // 5. Initialize SyncEngine with loaded or default config
        let sync_config = noda_core::sync::SyncConfig::load(&canonical_path)
            .await
            .unwrap_or_else(|_| noda_core::sync::SyncConfig {
                webdav_url: "".to_string(),
                webdav_username: "".to_string(),
                webdav_password: None,
                interval_secs: 300, // 5 minutes default
                device_name: "".to_string(),
            });
        let sync_engine = SyncEngine::new(sync_config);

        // Hook status callbacks to emit status to frontend
        let app_handle_clone2 = app_handle.clone();
        sync_engine.set_status_callback(move |status| {
            crate::events::emit_sync_status(&app_handle_clone2, status);
        });

        // Hook sync finished callbacks to emit sync report to frontend
        let app_handle_clone3 = app_handle.clone();
        sync_engine.set_sync_finished_callback(move |report| {
            crate::events::emit_sync_finished(&app_handle_clone3, report);
        });

        // Hook conflict callback to emit conflicts to frontend
        let app_handle_clone4 = app_handle.clone();
        sync_engine.set_conflict_callback(move |conflict| {
            crate::events::emit_sync_conflict(&app_handle_clone4, conflict);
        });

        // Hook progress callback to emit sync progress to frontend
        let app_handle_clone5 = app_handle.clone();
        sync_engine.set_progress_callback(move |progress| {
            crate::events::emit_sync_progress(&app_handle_clone5, progress);
        });

        // 6. Update AppState fields
        *self.vault_path.write() = Some(canonical_path);
        *self.database.write() = Some(db);
        *self.vault_service.write() = Some(service);
        *self.watcher.write() = Some(watcher);
        *self.sync_engine.write() = Some(sync_engine);
        *self.active_note_id.write() = None;

        Ok(())
    }

    /// Cleanly shuts down all active services (sync engine, watcher)
    pub async fn shutdown(&self) {
        if let Some(vault_path) = &*self.vault_path.read() {
            let active_note_id_opt = *self.active_note_id.read();
            if let Some(note_id) = active_note_id_opt {
                if let Some(service) = &*self.vault_service.read() {
                    if let Ok(note) = service.read_note(note_id).await {
                        if let Ok(snaps) = noda_core::history::list_snapshots(vault_path, note.id).await {
                            let needs_snapshot = if let Some(latest_snap) = snaps.first() {
                                if let Ok(restored_note) = noda_core::history::restore(vault_path, latest_snap).await {
                                    note.body != restored_note.body || note.title != restored_note.title || note.tags != restored_note.tags
                                } else {
                                    true
                                }
                            } else {
                                true
                            };

                            if needs_snapshot {
                                let _ = noda_core::history::snapshot(vault_path, &note, "App-Exit").await;
                            }
                        }
                    }
                }
            }
        }

        let engine_to_stop = {
            let mut engine_lock = self.sync_engine.write();
            engine_lock.take()
        };
        if let Some(engine) = engine_to_stop {
            let _ = engine.stop_sync().await;
        }
        {
            let mut watcher_lock = self.watcher.write();
            if let Some(watcher) = watcher_lock.take() {
                drop(watcher);
            }
        }
    }
}
