use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};
use tokio::runtime::Runtime;

use noda_core::database::connection::Database;
use noda_core::vault::service::VaultService;
use noda_core::watcher::VaultWatcher;
use noda_core::sync::SyncEngine;
use noda_core::models::vault::Vault;
use noda_core::models::note::{Note, NoteId};
use shared::dtos::{VaultInfoDto, NoteDto, NoteListItemDto, NoteMetadataDto, SearchResultDto, SnapshotDto, SnapshotDiffDto, DiffChunk, TrashEntryDto};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static JVM: OnceLock<jni::JavaVM> = OnceLock::new();

struct BridgeState {
    vault_path: Option<PathBuf>,
    database: Option<Database>,
    vault_service: Option<VaultService>,
    watcher: Option<VaultWatcher>,
    sync_engine: Option<SyncEngine>,
}

static BRIDGE_STATE: RwLock<BridgeState> = RwLock::new(BridgeState {
    vault_path: None,
    database: None,
    vault_service: None,
    watcher: None,
    sync_engine: None,
});

/// Thread-safe helper to get or initialize a global Tokio runtime for JNI calls.
fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to initialize Tokio runtime for JNI")
    })
}

/// Helper function to create a JSON formatted error jstring.
fn error_string(env: &mut JNIEnv, message: &str) -> jstring {
    env.new_string(format!("{{\"error\":\"{}\"}}", message))
        .unwrap()
        .into_raw()
}

/// Helper function to parse a JString parameter into a Rust String.
fn parse_string(env: &mut JNIEnv, s: &JString) -> Result<String, jstring> {
    match env.get_string(s) {
        Ok(jni_str) => Ok(jni_str.into()),
        Err(_) => Err(error_string(env, "Invalid input string")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_initVault(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jstring {
    if let Ok(vm) = env.get_java_vm() {
        let _ = JVM.set(vm);
    }
    let path_str: String = match env.get_string(&path) {
        Ok(s) => s.into(),
        Err(_) => return error_string(&mut env, "Invalid path string"),
    };

    let result = get_runtime().block_on(async {
        let canonical_path = match PathBuf::from(&path_str).canonicalize() {
            Ok(p) => p,
            Err(e) => return format!("{{\"error\":\"Canonicalize failed: {}\"}}", e),
        };

        // 1. Teardown existing services if active
        {
            let mut state = BRIDGE_STATE.write().unwrap();
            if let Some(engine) = state.sync_engine.take() {
                let _ = engine.stop_sync().await;
            }
            state.watcher.take();
        }

        // 2. Open or Rebuild Database & VaultService
        let db = match Database::open_or_rebuild(&canonical_path).await {
            Ok(d) => d,
            Err(e) => return format!("{{\"error\":\"Database failed: {}\"}}", e),
        };

        let service = match VaultService::new(&canonical_path) {
            Ok(s) => s,
            Err(e) => return format!("{{\"error\":\"VaultService failed: {}\"}}", e),
        };

        // 3. Start File Watcher
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);
        let watcher = match VaultWatcher::start(&canonical_path, event_tx) {
            Ok(w) => w,
            Err(e) => return format!("{{\"error\":\"Watcher failed: {}\"}}", e),
        };

        // 4. Background watcher task to sync changes to database
        let conn_clone = db.conn.clone();
        let canonical_path_clone = canonical_path.clone();
        tokio::spawn(async move {
            use noda_core::watcher::sync::sync_batch_with_db;
            while let Some(batch) = event_rx.recv().await {
                sync_batch_with_db(&canonical_path_clone, conn_clone.clone(), batch).await;
            }
        });

        // 5. Initialize SyncEngine
        let sync_config = noda_core::sync::SyncConfig::load(&canonical_path)
            .await
            .unwrap_or_else(|_| noda_core::sync::SyncConfig {
                webdav_url: "".to_string(),
                webdav_username: "".to_string(),
                webdav_password: None,
                interval_secs: 300,
                device_name: "".to_string(),
            });
        let sync_engine = SyncEngine::new(sync_config);
        sync_engine.set_progress_callback(move |progress| {
            if let Some(vm) = JVM.get() {
                if let Ok(mut env) = vm.attach_current_thread_as_daemon() {
                    if let Ok(class) = env.find_class("com/bubi/nodanotes/RustCore") {
                        let json_str = serde_json::to_string(&progress).unwrap_or_default();
                        if let Ok(jstr) = env.new_string(&json_str) {
                            let _ = env.call_static_method(
                                class,
                                "onSyncProgress",
                                "(Ljava/lang/String;)V",
                                &[jni::objects::JValue::from(&jstr)],
                            );
                        }
                    }
                }
            }
        });

        // 6. Update BRIDGE_STATE
        {
            let mut state = BRIDGE_STATE.write().unwrap();
            state.vault_path = Some(canonical_path);
            state.database = Some(db);
            state.vault_service = Some(service);
            state.watcher = Some(watcher);
            state.sync_engine = Some(sync_engine);
        }

        "{\"success\":true}".to_string()
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getVaultInfo(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let state = BRIDGE_STATE.read().unwrap();
    let path = match &state.vault_path {
        Some(p) => p,
        None => return error_string(&mut env, "Vault not initialized"),
    };

    let vault = Vault::new(path.clone());
    let dto = VaultInfoDto::from(vault);

    let result = serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e));
    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_refreshVault(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let (path, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let path = match &state.vault_path {
            Some(p) => p.clone(),
            None => return error_string(&mut env, "Vault not initialized"),
        };
        let db = match &state.database {
            Some(d) => d.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (path, db)
    };

    let result = get_runtime().block_on(async {
        let mut md_files = Vec::new();
        if let Ok(entries) = tokio::fs::read_dir(&path).await {
            let mut stack = vec![entries];
            while let Some(mut dir_entries) = stack.pop() {
                while let Ok(Some(entry)) = dir_entries.next_entry().await {
                    let file_type = match entry.file_type().await {
                        Ok(t) => t,
                        _ => continue,
                    };
                    let entry_path = entry.path();
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.') && name_str != ".templates" {
                        continue;
                    }
                    if file_type.is_dir() {
                        if let Ok(sub_entries) = tokio::fs::read_dir(entry_path).await {
                            stack.push(sub_entries);
                        }
                    } else if file_type.is_file() {
                        if let Some(ext) = entry_path.extension() {
                            if ext == "md" || ext == "markdown" {
                                md_files.push(entry_path);
                            }
                        }
                    }
                }
            }
        }

        let mut imported_count = 0;
        for file_path in md_files {
            let filename_stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let is_filename_valid_ulid = filename_stem.len() == 26 && ulid::Ulid::from_string(filename_stem).is_ok();

            let has_valid_frontmatter = if let Ok(content) = std::fs::read_to_string(&file_path) {
                let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
                let parsed = matter.parse(&content);
                if let Some(data) = &parsed.data {
                    if let Ok(fm) = data.deserialize::<noda_core::models::note::Frontmatter>() {
                        if is_filename_valid_ulid {
                            let expected_id = noda_core::models::note::NoteId(ulid::Ulid::from_string(filename_stem).unwrap());
                            fm.id == expected_id
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if !has_valid_frontmatter {
                imported_count += 1;
            }
        }

        let notes = match noda_core::vault::scan::scan_vault(&path).await {
            Ok(n) => n,
            Err(e) => return format!("{{\"error\":\"Vault scan failed: {}\"}}", e),
        };

        {
            let mut conn = db.conn.lock();
            if let Err(e) = noda_core::database::rebuild_database_sync(&notes, &mut conn) {
                return format!("{{\"error\":\"Database rebuild failed: {}\"}}", e);
            }
        }

        format!("{{\"imported_count\":{}}}", imported_count)
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Serialize)]
struct FolderDto {
    name: String,
    path: String,
    children: Vec<FolderDto>,
}

fn build_folder_tree(paths: &[String]) -> Vec<FolderDto> {
    let mut root: Vec<FolderDto> = Vec::new();
    for path in paths {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        add_to_tree(&mut root, &parts, "");
    }
    root
}

fn add_to_tree(nodes: &mut Vec<FolderDto>, parts: &[&str], current_prefix: &str) {
    if parts.is_empty() {
        return;
    }
    let name = parts[0];
    let path = if current_prefix.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", current_prefix, name)
    };

    if let Some(pos) = nodes.iter().position(|n| n.name == name) {
        add_to_tree(&mut nodes[pos].children, &parts[1..], &path);
    } else {
        let mut new_node = FolderDto {
            name: name.to_string(),
            path: path.clone(),
            children: Vec::new(),
        };
        add_to_tree(&mut new_node.children, &parts[1..], &path);
        nodes.push(new_node);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_listFolders(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let service = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        }
    };

    let folders = match service.list_folders() {
        Ok(f) => f,
        Err(e) => return error_string(&mut env, &format!("Failed to list folders: {}", e)),
    };

    let tree = build_folder_tree(&folders);
    let result = serde_json::to_string(&tree).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e));
    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct CreateFolderParams {
    parent_path: Option<String>,
    name: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_createFolder(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: CreateFolderParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let service = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        }
    };

    let rel_path = match &params.parent_path {
        Some(parent) if !parent.is_empty() => format!("{}/{}", parent.trim_end_matches('/'), params.name),
        _ => params.name,
    };

    let result = get_runtime().block_on(async {
        if let Err(e) = service.create_folder(&rel_path).await {
            return format!("{{\"error\":\"Failed to create folder: {}\"}}", e);
        }
        format!("{{\"path\":\"{}\"}}", rel_path)
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct RenameFolderParams {
    folder_path: String,
    new_name: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_renameFolder(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: RenameFolderParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (s, d)
    };

    let result = get_runtime().block_on(async {
        let base_path = service.base_path();
        let src_path = base_path.join(&params.folder_path);

        let parts: Vec<&str> = params.folder_path.split('/').collect();
        let mut new_parts = parts.clone();
        if new_parts.is_empty() {
            return "{\"error\":\"Source folder path is empty\"}".to_string();
        }
        new_parts.pop();
        new_parts.push(&params.new_name);
        let new_folder_rel_path = new_parts.join("/");

        let dest_path = base_path.join(&new_folder_rel_path);

        if src_path == dest_path {
            return format!("{{\"new_path\":\"{}\"}}", new_folder_rel_path);
        }

        if !src_path.exists() {
            return format!("{{\"error\":\"Source directory does not exist: {:?}\"}}", src_path);
        }

        if dest_path.exists() {
            return format!("{{\"error\":\"Destination already exists: {:?}\"}}", dest_path);
        }

        if let Err(e) = tokio::fs::rename(&src_path, &dest_path).await {
            return format!("{{\"error\":\"Failed to rename folder: {}\"}}", e);
        }

        let conn = db.conn.lock();
        let all_notes = match noda_core::database::queries::list_notes(&conn) {
            Ok(n) => n,
            Err(e) => return format!("{{\"error\":\"Database list failed: {}\"}}", e),
        };
        let src_prefix = format!("{}/", params.folder_path);

        for note in all_notes {
            if note.file_path.starts_with(&src_prefix) {
                let relative_suffix = &note.file_path[src_prefix.len()..];
                let new_path = format!("{}/{}", new_folder_rel_path, relative_suffix);
                if let Err(e) = noda_core::database::queries::update_note_file_path(&conn, note.id, &new_path) {
                    return format!("{{\"error\":\"Database path update failed: {}\"}}", e);
                }
            } else if note.file_path == params.folder_path {
                if let Err(e) = noda_core::database::queries::update_note_file_path(&conn, note.id, &new_folder_rel_path) {
                    return format!("{{\"error\":\"Database path update failed: {}\"}}", e);
                }
            }
        }

        format!("{{\"new_path\":\"{}\"}}", new_folder_rel_path)
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct DeleteFolderParams {
    folder_path: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_deleteFolder(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: DeleteFolderParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (path, service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (p, s, d)
    };

    let result = get_runtime().block_on(async {
        let notes_to_delete = {
            let conn = db.conn.lock();
            let all_notes = match noda_core::database::queries::list_notes(&conn) {
                Ok(n) => n,
                Err(e) => return format!("{{\"error\":\"Database list failed: {}\"}}", e),
            };
            let prefix = format!("{}/", params.folder_path);
            all_notes
                .into_iter()
                .filter(|note| note.file_path.starts_with(&prefix) || note.file_path == params.folder_path)
                .collect::<Vec<_>>()
        };

        let deleted_count = notes_to_delete.len();

        for note in notes_to_delete {
            if let Err(e) = noda_core::trash::soft_delete(&path, &note.file_path).await {
                return format!("{{\"error\":\"Soft delete failed for {}: {}\"}}", note.file_path, e);
            }
            let conn = db.conn.lock();
            if let Err(e) = noda_core::database::queries::delete_note(&conn, note.id, true) {
                return format!("{{\"error\":\"Database delete failed for {}: {}\"}}", note.id.0, e);
            }
        }

        if let Err(e) = service.delete_folder(&params.folder_path).await {
            return format!("{{\"error\":\"Failed to delete physical folder: {}\"}}", e);
        }

        format!("{{\"deleted_count\":{}}}", deleted_count)
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct MoveFolderParams {
    source_path: String,
    target_parent: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_moveFolder(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: MoveFolderParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (s, d)
    };

    let result = get_runtime().block_on(async {
        let base_path = service.base_path();
        let src_path = base_path.join(&params.source_path);

        let src_folder_name = match src_path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name,
            None => return "{\"error\":\"Source folder path has an invalid name\"}".to_string(),
        };

        let new_folder_rel_path = if params.target_parent.is_empty() {
            src_folder_name.to_string()
        } else {
            format!("{}/{}", params.target_parent.trim_end_matches('/'), src_folder_name)
        };

        let dest_path = base_path.join(&new_folder_rel_path);

        if src_path == dest_path {
            return format!("{{\"new_path\":\"{}\"}}", new_folder_rel_path);
        }

        if !src_path.exists() {
            return format!("{{\"error\":\"Source directory does not exist: {:?}\"}}", src_path);
        }

        if dest_path.exists() {
            return format!("{{\"error\":\"Destination already exists: {:?}\"}}", dest_path);
        }

        if let Some(parent) = dest_path.parent() {
            if !parent.exists() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    return format!("{{\"error\":\"Failed to create parent: {}\"}}", e);
                }
            }
        }

        if let Err(e) = tokio::fs::rename(&src_path, &dest_path).await {
            return format!("{{\"error\":\"Failed to rename folder on disk: {}\"}}", e);
        }

        let conn = db.conn.lock();
        let all_notes = match noda_core::database::queries::list_notes(&conn) {
            Ok(n) => n,
            Err(e) => return format!("{{\"error\":\"Database list failed: {}\"}}", e),
        };
        let src_prefix = format!("{}/", params.source_path);

        for note in all_notes {
            if note.file_path.starts_with(&src_prefix) {
                let relative_suffix = &note.file_path[src_prefix.len()..];
                let new_path = format!("{}/{}", new_folder_rel_path, relative_suffix);
                if let Err(e) = noda_core::database::queries::update_note_file_path(&conn, note.id, &new_path) {
                    return format!("{{\"error\":\"Database path update failed: {}\"}}", e);
                }
            } else if note.file_path == params.source_path {
                if let Err(e) = noda_core::database::queries::update_note_file_path(&conn, note.id, &new_folder_rel_path) {
                    return format!("{{\"error\":\"Database path update failed: {}\"}}", e);
                }
            }
        }

        format!("{{\"new_path\":\"{}\"}}", new_folder_rel_path)
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct ListNotesParams {
    folder_path: Option<String>,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_listNotes(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: ListNotesParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let db = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.database {
            Some(d) => d.clone(),
            None => return error_string(&mut env, "Vault not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        let notes = {
            let conn = db.conn.lock();
            match noda_core::database::queries::list_notes(&conn) {
                Ok(n) => n,
                Err(e) => return format!("{{\"error\":\"Database failed: {}\"}}", e),
            }
        };

        let filtered: Vec<NoteListItemDto> = notes
            .into_iter()
            .filter(|note| {
                let note_path = std::path::Path::new(&note.file_path);
                let note_parent = note_path.parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();

                match &params.folder_path {
                    Some(fp) if !fp.is_empty() => {
                        let fp_norm = fp.replace('\\', "/").trim_end_matches('/').to_string();
                        let parent_norm = note_parent.replace('\\', "/").trim_end_matches('/').to_string();
                        parent_norm == fp_norm
                    }
                    _ => {
                        note_parent.is_empty() || note_parent == "."
                    }
                }
            })
            .map(NoteListItemDto::from)
            .collect();

        serde_json::to_string(&filtered).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getAllNotes(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let db = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.database {
            Some(d) => d.clone(),
            None => return error_string(&mut env, "Vault not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        let notes = {
            let conn = db.conn.lock();
            match noda_core::database::queries::list_notes(&conn) {
                Ok(n) => n,
                Err(e) => return format!("{{\"error\":\"Database failed: {}\"}}", e),
            }
        };

        let dtos: Vec<NoteListItemDto> = notes.into_iter().map(NoteListItemDto::from).collect();
        serde_json::to_string(&dtos).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct GetNoteParams {
    note_id: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getNote(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: GetNoteParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let db = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.database {
            Some(d) => d.clone(),
            None => return error_string(&mut env, "Vault not initialized"),
        }
    };

    let note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let note_opt = {
            let conn = db.conn.lock();
            match noda_core::database::queries::get_note(&conn, note_id) {
                Ok(n) => n,
                Err(e) => return format!("{{\"error\":\"Database failed: {}\"}}", e),
            }
        };

        match note_opt {
            Some(note) => {
                let dto = NoteDto::from(note);
                serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
            }
            None => format!("{{\"error\":\"Note not found in DB: {}\"}}", params.note_id),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct CreateNoteParams {
    title: String,
    parent_folder: Option<String>,
    tags: Vec<String>,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_createNote(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: CreateNoteParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (s, d)
    };

    let result = get_runtime().block_on(async {
        let id = NoteId::new();
        let now = chrono::Utc::now();
        let file_path = match &params.parent_folder {
            Some(folder) if !folder.is_empty() => {
                format!("{}/{}.md", folder.trim_end_matches('/'), id.0.to_string())
            }
            _ => format!("{}.md", id.0.to_string()),
        };

        let note = Note {
            id,
            parent_id: None,
            title: params.title,
            body: "".to_string(),
            color: None,
            pinned: false,
            tags: params.tags,
            inline_tags: Vec::new(),
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
            file_path,
            is_encrypted: false,
            dek_encrypted: None,
            dek_nonce: None,
        };

        // 1. Write to local disk
        if let Err(e) = service.write_note(&note).await {
            return format!("{{\"error\":\"Disk write failed: {}\"}}", e);
        }

        // 2. Insert to SQLite DB cache
        let relative_path = note.file_path.clone();
        {
            let conn = db.conn.lock();
            if let Err(e) = noda_core::database::queries::upsert_note(&conn, &note, &relative_path, true) {
                return format!("{{\"error\":\"Database write failed: {}\"}}", e);
            }
        }

        let dto = NoteDto::from(note);
        serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct UpdateNoteParams {
    note_id: String,
    title: String,
    body: String,
    tags: Vec<String>,
    color: Option<String>,
    pinned: bool,
    trigger_snapshot: Option<bool>,
    snapshot_reason: Option<String>,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_updateNote(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: UpdateNoteParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (s, d)
    };

    let note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let existing_note = match service.read_note(note_id).await {
            Ok(n) => n,
            Err(e) => return format!("{{\"error\":\"Note not found on disk: {}\"}}", e),
        };

        let now = chrono::Utc::now();
        let note = Note {
            id: note_id,
            parent_id: existing_note.parent_id,
            title: params.title,
            inline_tags: Note::parse_inline_tags(&params.body),
            body: params.body,
            color: params.color,
            pinned: params.pinned,
            tags: params.tags,
            status: existing_note.status.clone(),
            created_at: existing_note.created_at,
            updated_at: now,
            file_path: existing_note.file_path.clone(),
            is_encrypted: existing_note.is_encrypted,
            dek_encrypted: existing_note.dek_encrypted.clone(),
            dek_nonce: existing_note.dek_nonce.clone(),
        };

        let trigger_snap = params.trigger_snapshot.unwrap_or(false);
        if trigger_snap {
            let vault_path = service.base_path();
            let reason = params.snapshot_reason.as_deref().unwrap_or("Android");
            // Snapshot the PREVIOUS state before we overwrite it
            let _ = noda_core::history::snapshot(vault_path, &existing_note, reason).await;
        }

        // 1. Write to local disk
        if let Err(e) = service.write_note(&note).await {
            return format!("{{\"error\":\"Disk write failed: {}\"}}", e);
        }

        // 2. Update DB
        let relative_path = note.file_path.clone();
        {
            let conn = db.conn.lock();
            if let Err(e) = noda_core::database::queries::upsert_note(&conn, &note, &relative_path, true) {
                return format!("{{\"error\":\"Database write failed: {}\"}}", e);
            }
        }

        format!("{{\"updated_at\":\"{}\"}}", now.to_rfc3339())
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct RenameNoteParams {
    note_id: String,
    new_title: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_renameNote(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: RenameNoteParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (s, d)
    };

    let note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let mut note = match service.read_note(note_id).await {
            Ok(n) => n,
            Err(e) => return format!("{{\"error\":\"Note not found on disk: {}\"}}", e),
        };

        note.title = params.new_title;
        note.updated_at = chrono::Utc::now();

        // 1. Write back to disk (filename does not change since Noda uses ULID.md filenames)
        if let Err(e) = service.write_note(&note).await {
            return format!("{{\"error\":\"Disk write failed: {}\"}}", e);
        }

        // 2. Update DB
        let relative_path = note.file_path.clone();
        {
            let conn = db.conn.lock();
            if let Err(e) = noda_core::database::queries::upsert_note(&conn, &note, &relative_path, true) {
                return format!("{{\"error\":\"Database write failed: {}\"}}", e);
            }
        }

        format!("{{\"new_file_path\":\"{}\"}}", relative_path)
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct DeleteNoteParams {
    note_id: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_deleteNote(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: DeleteNoteParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (path, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (p, d)
    };

    let note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let relative_path = {
            let conn = db.conn.lock();
            match noda_core::database::queries::get_note(&conn, note_id) {
                Ok(Some(n)) => n.file_path,
                Ok(None) => return format!("{{\"error\":\"Note not found in DB: {}\"}}", params.note_id),
                Err(e) => return format!("{{\"error\":\"Database read failed: {}\"}}", e),
            }
        };

        // 1. Soft delete on disk
        if let Err(e) = noda_core::trash::soft_delete(&path, &relative_path).await {
            return format!("{{\"error\":\"Soft delete failed: {}\"}}", e);
        }

        // 2. Remove from DB
        {
            let conn = db.conn.lock();
            if let Err(e) = noda_core::database::queries::delete_note(&conn, note_id, true) {
                return format!("{{\"error\":\"Database delete failed: {}\"}}", e);
            }
        }

        "{\"success\":true}".to_string()
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct MoveNoteParams {
    note_id: String,
    target_folder: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_moveNote(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: MoveNoteParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (s, d)
    };

    let note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let old_rel_path = {
            let conn = db.conn.lock();
            match noda_core::database::queries::get_note(&conn, note_id) {
                Ok(Some(n)) => n.file_path,
                Ok(None) => return format!("{{\"error\":\"Note not found in DB: {}\"}}", params.note_id),
                Err(e) => return format!("{{\"error\":\"Database read failed: {}\"}}", e),
            }
        };

        let filename = format!("{}.md", params.note_id);
        let new_rel_path = if params.target_folder.is_empty() {
            filename
        } else {
            format!("{}/{}", params.target_folder.trim_end_matches('/'), filename)
        };

        if old_rel_path != new_rel_path {
            // 1. Rename physical file
            if let Err(e) = service.rename_note_file(&old_rel_path, &new_rel_path).await {
                return format!("{{\"error\":\"Disk rename failed: {}\"}}", e);
            }

            // 2. Update DB path
            let conn = db.conn.lock();
            if let Err(e) = noda_core::database::queries::update_note_file_path(&conn, note_id, &new_rel_path) {
                return format!("{{\"error\":\"Database update failed: {}\"}}", e);
            }
        }

        format!("{{\"new_file_path\":\"{}\"}}", new_rel_path)
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct GetNoteMetadataParams {
    note_id: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getNoteMetadata(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: GetNoteMetadataParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (path, service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (p, s, d)
    };

    let note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let note = {
            let conn = db.conn.lock();
            match noda_core::database::queries::get_note(&conn, note_id) {
                Ok(Some(n)) => n,
                Ok(None) => return format!("{{\"error\":\"Note not found in DB: {}\"}}", params.note_id),
                Err(e) => return format!("{{\"error\":\"Database read failed: {}\"}}", e),
            }
        };

        let snapshots = noda_core::history::list_snapshots(&path, note_id).await.unwrap_or_default();
        let history_count = snapshots.len();

        let remote_state = {
            let conn = db.conn.lock();
            noda_core::sync::load_remote_state(&conn).unwrap_or_default()
        };
        let last_upload_time = remote_state.files.get(&note.file_path)
            .and_then(|meta| meta.last_modified)
            .map(|dt| dt.to_rfc3339());

        let abs_path = service.find_note_path(note_id);
        let file_name = abs_path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let file_size_bytes = tokio::fs::metadata(&abs_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        let char_count = note.body.chars().count();
        let word_count = note.body.split_whitespace().count();

        let dto = NoteMetadataDto {
            id: note.id.0.to_string(),
            title: note.title,
            file_name,
            relative_path: note.file_path,
            absolute_path: abs_path.to_string_lossy().into_owned(),
            created_at: note.created_at.to_rfc3339(),
            updated_at: note.updated_at.to_rfc3339(),
            tags: note.tags,
            history_count,
            last_upload_time,
            file_size_bytes,
            word_count,
            char_count,
        };

        serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getAllTags(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let db = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.database {
            Some(d) => d.clone(),
            None => return error_string(&mut env, "Vault not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        let notes = {
            let conn = db.conn.lock();
            match noda_core::database::queries::list_notes(&conn) {
                Ok(n) => n,
                Err(e) => return format!("{{\"error\":\"Database failed: {}\"}}", e),
            }
        };

        let mut tags_set = std::collections::HashSet::new();
        for note in notes {
            for tag in note.tags {
                if !tag.trim().is_empty() {
                    tags_set.insert(tag.trim().to_string());
                }
            }
        }

        let mut tags: Vec<String> = tags_set.into_iter().collect();
        tags.sort();

        serde_json::to_string(&tags).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct SearchNotesParams {
    query: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_searchNotes(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: SearchNotesParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let db = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.database {
            Some(d) => d.clone(),
            None => return error_string(&mut env, "Vault not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        let conn = db.conn.lock();
        let results = match noda_core::database::search::search_notes(&conn, &params.query) {
            Ok(r) => r,
            Err(e) => return format!("{{\"error\":\"Search failed: {}\"}}", e),
        };

        let dtos: Vec<SearchResultDto> = results
            .into_iter()
            .map(|r| {
                let match_type = if r.snippet.starts_with("ID: ") {
                    "id".to_string()
                } else if r.snippet.starts_with("File: ") {
                    "filename".to_string()
                } else {
                    "body".to_string()
                };

                SearchResultDto {
                    note_id: r.id.0.to_string(),
                    title: r.title,
                    snippet: r.snippet,
                    match_type,
                    score: r.score as f64,
                }
            })
            .collect();

        serde_json::to_string(&dtos).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct ListSnapshotsParams {
    note_id: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_listSnapshots(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: ListSnapshotsParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(p) => p.clone(),
            None => return error_string(&mut env, "Vault not initialized"),
        }
    };

    let note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let snaps = match noda_core::history::list_snapshots(&path, note_id).await {
            Ok(s) => s,
            Err(e) => return format!("{{\"error\":\"Failed to list snapshots: {}\"}}", e),
        };

        let dtos: Vec<SnapshotDto> = snaps
            .into_iter()
            .map(|s| {
                let size_bytes = std::fs::metadata(&s.absolute_path).map(|m| m.len()).unwrap_or(0);
                let relative_path = s.absolute_path
                    .strip_prefix(&path)
                    .unwrap_or(&s.absolute_path)
                    .to_string_lossy()
                    .into_owned();

                SnapshotDto {
                    note_id: s.note_id.0.to_string(),
                    timestamp: s.timestamp.to_rfc3339(),
                    file_path: relative_path,
                    size_bytes,
                    reason: s.reason.clone(),
                }
            })
            .collect();

        serde_json::to_string(&dtos).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct GetSnapshotDiffParams {
    note_id: String,
    timestamp: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getSnapshotDiff(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: GetSnapshotDiffParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (path, service) = {
        let state = BRIDGE_STATE.read().unwrap();
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        (p, s)
    };

    let note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let parsed_timestamp = match chrono::DateTime::parse_from_rfc3339(&params.timestamp) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(e) => return error_string(&mut env, &format!("Invalid timestamp format: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let snaps = match noda_core::history::list_snapshots(&path, note_id).await {
            Ok(s) => s,
            Err(e) => return format!("{{\"error\":\"Failed to list snapshots: {}\"}}", e),
        };

        let target_snap = match snaps.into_iter().find(|s| s.timestamp.timestamp_millis() == parsed_timestamp.timestamp_millis()) {
            Some(s) => s,
            None => return format!("{{\"error\":\"Snapshot not found for timestamp: {}\"}}", params.timestamp),
        };

        let current_note = match service.read_note(note_id).await {
            Ok(n) => n,
            Err(e) => return format!("{{\"error\":\"Current note not found: {}\"}}", e),
        };

        let diffs = match noda_core::history::compare(&path, &target_snap, &current_note).await {
            Ok(d) => d,
            Err(e) => return format!("{{\"error\":\"Compare failed: {}\"}}", e),
        };

        let body_chunks: Vec<DiffChunk> = diffs
            .into_iter()
            .map(|chunk| DiffChunk {
                tag: chunk.tag,
                text: chunk.text,
            })
            .collect();

        let dto = SnapshotDiffDto {
            note_id: params.note_id,
            timestamp: params.timestamp,
            body_chunks,
        };

        serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct RestoreSnapshotParams {
    note_id: String,
    timestamp: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_restoreSnapshot(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: RestoreSnapshotParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (path, service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (p, s, d)
    };

    let note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let parsed_timestamp = match chrono::DateTime::parse_from_rfc3339(&params.timestamp) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(e) => return error_string(&mut env, &format!("Invalid timestamp format: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let snaps = match noda_core::history::list_snapshots(&path, note_id).await {
            Ok(s) => s,
            Err(e) => return format!("{{\"error\":\"Failed to list snapshots: {}\"}}", e),
        };

        let target_snap = match snaps.into_iter().find(|s| s.timestamp.timestamp_millis() == parsed_timestamp.timestamp_millis()) {
            Some(s) => s,
            None => return format!("{{\"error\":\"Snapshot not found for timestamp: {}\"}}", params.timestamp),
        };

        let restored_note_from_snap = match noda_core::history::restore(&path, &target_snap).await {
            Ok(n) => n,
            Err(e) => return format!("{{\"error\":\"Restore failed: {}\"}}", e),
        };

        let current_note = {
            let conn = db.conn.lock();
            noda_core::database::queries::get_note(&conn, note_id).ok().flatten()
        };

        if let Some(ref current) = current_note {
            let _ = noda_core::history::snapshot(&path, current, "Pre-Restore").await;
        }

        let merged_note = match &current_note {
            Some(current) => Note {
                id: current.id,
                parent_id: current.parent_id.clone(),
                title: restored_note_from_snap.title,
                inline_tags: restored_note_from_snap.inline_tags.clone(),
                body: restored_note_from_snap.body,
                color: current.color.clone(),
                pinned: current.pinned,
                tags: current.tags.clone(),
                status: current.status.clone(),
                created_at: current.created_at,
                updated_at: chrono::Utc::now(),
                file_path: current.file_path.clone(),
                is_encrypted: current.is_encrypted,
                dek_encrypted: current.dek_encrypted.clone(),
                dek_nonce: current.dek_nonce.clone(),
            },
            None => {
                let mut note = restored_note_from_snap;
                note.updated_at = chrono::Utc::now();
                note
            }
        };

        let relative_path = merged_note.file_path.clone();

        // 1. Overwrite note on disk
        if let Err(e) = service.write_note(&merged_note).await {
            return format!("{{\"error\":\"Disk write failed: {}\"}}", e);
        }

        // 2. Update DB
        {
            let conn = db.conn.lock();
            if let Err(e) = noda_core::database::queries::upsert_note(&conn, &merged_note, &relative_path, true) {
                return format!("{{\"error\":\"Database update failed: {}\"}}", e);
            }
        }

        let dto = NoteDto::from(merged_note);
        serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct DeleteSnapshotParams {
    note_id: String,
    timestamp: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_deleteSnapshot(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: DeleteSnapshotParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(p) => p.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let parsed_timestamp = match chrono::DateTime::parse_from_rfc3339(&params.timestamp) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(e) => return error_string(&mut env, &format!("Invalid timestamp format: {}", e)),
    };

    let result = get_runtime().block_on(async {
        if let Err(e) = noda_core::history::delete_snapshot(&path, note_id, parsed_timestamp).await {
            return format!("{{\"error\":\"Delete snapshot failed: {}\"}}", e);
        }
        "{\"success\":true}".to_string()
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_listTrash(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(p) => p.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        let trash_list = match noda_core::trash::list_trash(&path).await {
            Ok(t) => t,
            Err(e) => return format!("{{\"error\":\"Failed to list trash: {}\"}}", e),
        };

        let dtos: Vec<TrashEntryDto> = trash_list
            .into_iter()
            .map(|entry| {
                let trash_filename = format!("{}.md", entry.note_id.0.to_string());
                let trash_path = format!(".noda/trash/{}", trash_filename);
                TrashEntryDto {
                    id: entry.note_id.0.to_string(),
                    title: entry.title,
                    original_path: entry.original_path,
                    deleted_at: entry.deleted_at.to_rfc3339(),
                    trash_path,
                }
            })
            .collect();

        serde_json::to_string(&dtos).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct RestoreFromTrashParams {
    note_id: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_restoreFromTrash(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: RestoreFromTrashParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (path, service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (p, s, d)
    };

    let parsed_note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let trash_list = match noda_core::trash::list_trash(&path).await {
            Ok(t) => t,
            Err(e) => return format!("{{\"error\":\"Failed to list trash: {}\"}}", e),
        };

        let target_entry = match trash_list.into_iter().find(|e| e.note_id == parsed_note_id) {
            Some(entry) => entry,
            None => return format!("{{\"error\":\"Note not found in trash: {}\"}}", params.note_id),
        };

        if let Err(e) = noda_core::trash::restore(&path, &target_entry).await {
            return format!("{{\"error\":\"Restore failed: {}\"}}", e);
        }

        let restored_note = match service.read_note(parsed_note_id).await {
            Ok(n) => n,
            Err(e) => return format!("{{\"error\":\"Read restored note failed: {}\"}}", e),
        };

        {
            let conn = db.conn.lock();
            if let Err(e) = noda_core::database::queries::upsert_note(&conn, &restored_note, &target_entry.original_path, true) {
                return format!("{{\"error\":\"Database sync failed: {}\"}}", e);
            }
        }

        format!("{{\"restored_path\":\"{}\"}}", target_entry.original_path)
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct PermanentDeleteParams {
    note_id: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_permanentDelete(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: PermanentDeleteParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let parsed_note_id = match ulid::Ulid::from_string(&params.note_id) {
        Ok(u) => NoteId(u),
        Err(e) => return error_string(&mut env, &format!("Invalid NoteId: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let trash_list = match noda_core::trash::list_trash(&path).await {
            Ok(t) => t,
            Err(e) => return format!("{{\"error\":\"Failed to list trash: {}\"}}", e),
        };

        let target_entry = match trash_list.into_iter().find(|e| e.note_id == parsed_note_id) {
            Some(entry) => entry,
            None => return format!("{{\"error\":\"Note not found in trash: {}\"}}", params.note_id),
        };

        if let Err(e) = noda_core::trash::permanent_delete(&path, &target_entry).await {
            return format!("{{\"error\":\"Permanent delete failed: {}\"}}", e);
        }

        "{\"success\":true}".to_string()
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_emptyTrash(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        let trash_list = match noda_core::trash::list_trash(&path).await {
            Ok(t) => t,
            Err(e) => return format!("{{\"error\":\"Failed to list trash: {}\"}}", e),
        };

        let deleted_count = trash_list.len();

        for entry in trash_list {
            let _ = noda_core::trash::permanent_delete(&path, &entry).await;
        }

        format!("{{\"deleted_count\":{}}}", deleted_count)
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct AddAttachmentParams {
    source_path: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_addAttachment(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: AddAttachmentParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (path, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let path = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let db = state.database.clone();
        (path, db)
    };

    let result = get_runtime().block_on(async {
        let source_path = std::path::Path::new(&params.source_path);
        let original_name = source_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "attachment".to_string());

        match noda_core::attachments::store_attachment(&path, source_path).await {
            Ok(uri) => {
                let attachment_name = uri.trim_start_matches("noda://attachments/").to_string();
                let markdown_link = format!("![{}]({})", original_name, uri);

                if let Some(ref db) = db {
                    let conn = db.conn.lock();
                    let rel_path = format!(".noda/attachments/{}", attachment_name);
                    let _ = noda_core::database::queries::set_file_dirty(&conn, &rel_path, true);
                }

                serde_json::json!({
                    "attachment_name": attachment_name,
                    "markdown_link": markdown_link
                }).to_string()
            }
            Err(e) => format!("{{\"error\":\"Store attachment failed: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct DeleteAttachmentParams {
    attachment_name: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_deleteAttachment(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: DeleteAttachmentParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (path, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let path = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let db = state.database.clone();
        (path, db)
    };

    let result = get_runtime().block_on(async {
        match noda_core::attachments::delete_attachment(&path, &params.attachment_name).await {
            Ok(_) => {
                if let Some(ref db) = db {
                    let conn = db.conn.lock();
                    let rel_path = format!(".noda/attachments/{}", params.attachment_name);
                    let _ = noda_core::database::queries::set_file_dirty(&conn, &rel_path, true);
                }
                "{\"success\":true}".to_string()
            }
            Err(e) => format!("{{\"error\":\"Delete attachment failed: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}


#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_listAttachments(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        match noda_core::attachments::list_attachments_with_metadata(&path).await {
            Ok(list) => {
                let dtos: Vec<shared::dtos::AttachmentInfoDto> = list.into_iter().map(|item| {
                    let mime = mime_guess::from_path(&item.name)
                        .first_or_octet_stream()
                        .to_string();
                    shared::dtos::AttachmentInfoDto {
                        name: item.name,
                        size_bytes: item.size,
                        mime_type: mime,
                        modified_at: item.modified_at,
                    }
                }).collect();
                serde_json::to_string(&dtos).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
            }
            Err(e) => format!("{{\"error\":\"List attachments failed: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct GetAttachmentDataParams {
    attachment_name: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getAttachmentData(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: GetAttachmentDataParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        let uri = format!("noda://attachments/{}", params.attachment_name);
        let resolved = match noda_core::attachments::resolve_path(&path, &uri) {
            Ok(p) => p,
            Err(e) => return format!("{{\"error\":\"Invalid path: {}\"}}", e),
        };

        match tokio::fs::read(&resolved).await {
            Ok(bytes) => {
                use base64::Engine;
                let base64_data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                let mime = mime_guess::from_path(&params.attachment_name)
                    .first_or_octet_stream()
                    .to_string();
                serde_json::json!({
                    "data_base64": base64_data,
                    "mime_type": mime
                }).to_string()
            }
            Err(e) => format!("{{\"error\":\"Failed to read attachment file: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct SaveSyncConfigParams {
    webdav_url: String,
    username: String,
    password: Option<String>,
    interval_secs: u64,
    #[serde(default)]
    device_name: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_saveSyncConfig(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: SaveSyncConfigParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (sync_engine, path) = {
        let state = BRIDGE_STATE.read().unwrap();
        let engine = match &state.sync_engine {
            Some(e) => e.clone(),
            None => return error_string(&mut env, "Sync engine not initialized"),
        };
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        (engine, p)
    };

    let result = get_runtime().block_on(async {
        let new_config = noda_core::sync::SyncConfig {
            webdav_url: params.webdav_url,
            webdav_username: params.username,
            webdav_password: params.password,
            interval_secs: params.interval_secs,
            device_name: params.device_name,
        };

        let _ = sync_engine.stop_sync().await;

        sync_engine.set_config(new_config.clone());

        if let Err(e) = new_config.save(&path).await {
            return format!("{{\"error\":\"Failed to save config: {}\"}}", e);
        }

        "{\"success\":true}".to_string()
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_loadSyncConfig(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let sync_engine = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.sync_engine {
            Some(e) => e.clone(),
            None => return error_string(&mut env, "Sync engine not initialized"),
        }
    };

    let config = sync_engine.get_config();
    let is_configured = !config.webdav_url.is_empty();

    let result = serde_json::json!({
        "webdav_url": config.webdav_url,
        "username": config.webdav_username,
        "device_name": config.device_name,
        "interval_secs": config.interval_secs,
        "is_configured": is_configured
    }).to_string();

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct TestWebdavConnectionParams {
    webdav_url: String,
    username: String,
    password: Option<String>,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_testWebdavConnection(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: TestWebdavConnectionParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let result = get_runtime().block_on(async {
        let client = match noda_core::sync::WebDavClient::new(
            &params.webdav_url,
            &params.username,
            params.password.as_deref().unwrap_or(""),
        ) {
            Ok(c) => c,
            Err(e) => return format!("{{\"error\":\"Client build failed: {}\"}}", e),
        };

        match client.propfind("", 0).await {
            Ok(_) => "{\"success\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"Connection failed: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_syncNow(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let (sync_engine, path, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let engine = match &state.sync_engine {
            Some(e) => e.clone(),
            None => return error_string(&mut env, "Sync engine not initialized"),
        };
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (engine, p, d)
    };

    let result = get_runtime().block_on(async {
        match sync_engine.sync_now(&path, db.clone()).await {
            Ok(report) => {
                let conn = db.conn.lock();
                
                let resolve_files = |files: Vec<String>| -> Vec<String> {
                    files.into_iter().map(|f| {
                        let path_buf = std::path::Path::new(&f);
                        if let Some(stem) = path_buf.file_stem().and_then(|s| s.to_str()) {
                            // Check if the filename is a ULID, or has ULID as a prefix
                            let id_part = stem.split('_').next().unwrap_or(stem);
                            if let Ok(ulid) = ulid::Ulid::from_string(id_part) {
                                let note_id = NoteId(ulid);
                                
                                // 1. Check live notes database cache
                                if let Ok(Some(note)) = noda_core::database::queries::get_note(&conn, note_id) {
                                    return format!("{} ({})", note.title, f);
                                }
                                
                                // 2. Check trash sidecar json for title
                                let trash_json_path = path.join(".noda").join("trash").join(format!("{}.json", ulid.to_string()));
                                if trash_json_path.exists() {
                                    if let Ok(content) = std::fs::read_to_string(&trash_json_path) {
                                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                                            if let Some(title) = val.get("title").and_then(|t| t.as_str()) {
                                                return format!("{} (Deleted: {})", title, f);
                                            }
                                        }
                                    }
                                }

                                // 3. Try parsing frontmatter directly from the live or conflict file if it exists
                                let full_file_path = path.join(&f);
                                if full_file_path.exists() {
                                    if let Ok(content) = std::fs::read_to_string(&full_file_path) {
                                        let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
                                        let parsed = matter.parse(&content);
                                        if let Some(data) = parsed.data {
                                            if let Ok(fm) = data.deserialize::<noda_core::models::note::Frontmatter>() {
                                                return format!("{} ({})", fm.title, f);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Fallback: If it's a history/conflict file, look up the note ID in the path/stem
                        // Example: .noda/history/01KSBMXRFKZP1NJVR953Z3WBTA/20260531_002807_163.md
                        // We can extract a 26-char ULID sequence from the path string.
                        let mut resolved_title = None;
                        for segment in f.split('/') {
                            if segment.len() == 26 {
                                if let Ok(ulid) = ulid::Ulid::from_string(segment) {
                                    let note_id = NoteId(ulid);
                                    if let Ok(Some(note)) = noda_core::database::queries::get_note(&conn, note_id) {
                                        resolved_title = Some(note.title.clone());
                                        break;
                                    }
                                    // Or try parsing live file or trash sidecar
                                    let trash_json_path = path.join(".noda").join("trash").join(format!("{}.json", ulid.to_string()));
                                    if trash_json_path.exists() {
                                        if let Ok(content) = std::fs::read_to_string(&trash_json_path) {
                                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                                                if let Some(title) = val.get("title").and_then(|t| t.as_str()) {
                                                    resolved_title = Some(title.to_string());
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(title) = resolved_title {
                            format!("{} ({})", title, f)
                        } else {
                            f
                        }
                    }).collect()
                };

                let now_str = chrono::Utc::now().to_rfc3339();

                let dto = shared::dtos::SyncReportDto {
                    uploads: report.uploads,
                    downloads: report.downloads,
                    deletes_local: report.deletes_local,
                    deletes_remote: report.deletes_remote,
                    conflicts: report.conflicts,
                    uploaded_files: resolve_files(report.uploaded_files),
                    downloaded_files: resolve_files(report.downloaded_files),
                    deleted_local_files: resolve_files(report.deleted_local_files),
                    deleted_remote_files: resolve_files(report.deleted_remote_files),
                    conflict_files: resolve_files(report.conflict_files),
                };

                // Inject a custom field for sync_time in JSON directly or add a field if present in DTO.
                // Since SyncReportDto doesn't have a sync_time field in shared library, we can serialize the DTO,
                // and then parse it as a JSON Object, insert "sync_time": now_str, and serialize back to string!
                if let Ok(mut val) = serde_json::to_value(&dto) {
                    if let Some(obj) = val.as_object_mut() {
                        obj.insert("sync_time".to_string(), serde_json::Value::String(now_str));
                    }
                    serde_json::to_string(&val).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
                } else {
                    serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
                }
            }
            Err(e) => format!("{{\"error\":\"Sync failed: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getSyncStatus(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let (sync_engine, path, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let engine = match &state.sync_engine {
            Some(e) => e.clone(),
            None => return error_string(&mut env, "Sync engine not initialized"),
        };
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (engine, p, d)
    };

    let status = sync_engine.get_status();
    let is_syncing = status == noda_core::sync::SyncStatus::Syncing;

    let result = get_runtime().block_on(async {
        let remote_state = {
            let conn = db.conn.lock();
            noda_core::sync::load_remote_state(&conn).unwrap_or_default()
        };
        let last_sync_at = remote_state.last_sync_time
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "Never".to_string());

        let pending_count = match noda_core::sync::queue::SyncQueue::load(&path).await {
            Ok(q) => q.len(),
            Err(_) => 0,
        };

        let quarantined_files = {
            let conn = db.conn.lock();
            let mut list = Vec::new();
            if let Ok(mut stmt) = conn.prepare("SELECT path, retry_count, sync_error FROM sync_file_states WHERE retry_count > 0") {
                if let Ok(rows) = stmt.query_map([], |row| {
                    Ok(serde_json::json!({
                        "path": row.get::<_, String>(0)?,
                        "retry_count": row.get::<_, i32>(1)?,
                        "sync_error": row.get::<_, Option<String>>(2)?,
                    }))
                }) {
                    for r in rows {
                        if let Ok(val) = r {
                            list.push(val);
                        }
                    }
                }
            }
            list
        };

        serde_json::json!({
            "is_syncing": is_syncing,
            "last_sync_at": last_sync_at,
            "pending_count": pending_count,
            "quarantined_files": quarantined_files
        }).to_string()
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_listConflicts(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        match noda_core::sync::conflict::list_conflicts(&path).await {
            Ok(list) => {
                let dtos: Vec<shared::dtos::ConflictEntryDto> = list.into_iter().map(|item| {
                    shared::dtos::ConflictEntryDto {
                        id: item.note_id.0.to_string(),
                        title: item.local_title,
                        file_path: item.relative_path,
                        archived_path: item.archived_path,
                        detected_at: item.detected_at.to_rfc3339(),
                    }
                }).collect();
                serde_json::to_string(&dtos).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
            }
            Err(e) => format!("{{\"error\":\"Failed to list conflicts: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct GetConflictNoteParams {
    archived_path: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getConflictNote(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: GetConflictNoteParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        let full_path = path.join(&params.archived_path);
        match VaultService::read_note_from_absolute_path(&full_path, &params.archived_path).await {
            Ok(note) => {
                let dto = NoteDto::from(note);
                serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
            }
            Err(e) => format!("{{\"error\":\"Failed to read conflict note: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct ResolveConflictParams {
    note_id: String,
    resolution: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_resolveConflict(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: ResolveConflictParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (service, path, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (s, p, d)
    };

    let result = get_runtime().block_on(async {
        let conflicts_dir = path.join(".noda/conflicts");
        if !conflicts_dir.exists() {
            return "{\"success\":true}".to_string();
        }

        let mut conflict_files = Vec::new();
        if let Ok(mut dir) = tokio::fs::read_dir(&conflicts_dir).await {
            while let Ok(Some(entry)) = dir.next_entry().await {
                let path = entry.path();
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                        if filename.starts_with(&format!("{}_", params.note_id)) && filename.ends_with(".md") {
                            let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                            let parts: Vec<&str> = stem.split('_').collect();
                            let timestamp = if parts.len() >= 2 {
                                parts[1].parse::<i64>().unwrap_or(0)
                            } else {
                                0
                            };
                            conflict_files.push((path.clone(), timestamp));
                        }
                    }
                }
            }
        }

        if conflict_files.is_empty() {
            return format!("{{\"error\":\"No conflict found for note {}\"}}", params.note_id);
        }

        conflict_files.sort_by(|a, b| b.1.cmp(&a.1));

        if params.resolution == "use_remote" {
            let (latest_path, _) = &conflict_files[0];
            let remote_note = match VaultService::read_note_from_absolute_path(latest_path, "").await {
                Ok(n) => n,
                Err(e) => return format!("{{\"error\":\"Failed to read conflict note: {}\"}}", e),
            };

            let local_note_id = match ulid::Ulid::from_string(&params.note_id) {
                Ok(u) => NoteId(u),
                Err(e) => return format!("{{\"error\":\"Invalid NoteId: {}\"}}", e),
            };

            let relative_path = {
                let conn = db.conn.lock();
                match noda_core::database::queries::get_note(&conn, local_note_id) {
                    Ok(Some(n)) => n.file_path,
                    _ => format!("{}.md", params.note_id),
                }
            };

            let mut restored_note = remote_note;
            restored_note.file_path = relative_path.clone();
            restored_note.updated_at = chrono::Utc::now();

            if let Err(e) = service.write_note(&restored_note).await {
                return format!("{{\"error\":\"Failed to overwrite local note: {}\"}}", e);
            }

            {
                let conn = db.conn.lock();
                if let Err(e) = noda_core::database::queries::upsert_note(&conn, &restored_note, &relative_path, true) {
                    return format!("{{\"error\":\"Failed to update DB: {}\"}}", e);
                }
            }
        }

        for (file_path, _) in conflict_files {
            let _ = tokio::fs::remove_file(file_path).await;
        }

        "{\"success\":true}".to_string()
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_rebuildCache(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let (path, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (p, d)
    };

    let result = get_runtime().block_on(async {
        let mut conn = db.conn.lock();
        match noda_core::database::rebuild::rebuild_database(&path, &mut conn).await {
            Ok(_) => {
                let count = match noda_core::database::queries::list_notes(&conn) {
                    Ok(list) => list.len(),
                    Err(_) => 0,
                };
                serde_json::json!({
                    "success": true,
                    "note_count": count
                }).to_string()
            }
            Err(e) => format!("{{\"error\":\"Rebuild failed: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_optimizeFts(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let db = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        let conn = db.conn.lock();
        let _ = conn.execute("INSERT INTO notes_fts(notes_fts) VALUES('optimize');", []);
        match noda_core::diagnostics::vacuum_database(&conn) {
            Ok(_) => "{\"success\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"Optimize failed: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getDuplicateNotes(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        match noda_core::diagnostics::get_duplicate_notes(&path).await {
            Ok(list) => {
                let mapped = serde_json::json!(list.iter().map(|g| {
                    serde_json::json!({
                        "note_id": g.note_id,
                        "title": g.title,
                        "files": g.files.iter().map(|f| {
                            serde_json::json!({
                                "path": f.relative_path,
                                "size_bytes": f.size_bytes,
                                "modified_at": f.last_modified
                            })
                        }).collect::<Vec<_>>()
                    })
                }).collect::<Vec<_>>());
                mapped.to_string()
            }
            Err(e) => format!("{{\"error\":\"Failed to get duplicate notes: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct DeleteDuplicateFileParams {
    file_path: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_deleteDuplicateFile(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: DeleteDuplicateFileParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        match noda_core::diagnostics::delete_duplicate_note_file(&path, &params.file_path).await {
            Ok(_) => "{\"success\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"Delete duplicate file failed: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getOrphanedRemnants(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        match noda_core::diagnostics::get_orphaned_remnants(&path).await {
            Ok(remnants) => serde_json::to_string(&remnants).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e)),
            Err(e) => format!("{{\"error\":\"Failed to get orphaned remnants: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct DeleteOrphanedFileParams {
    file_path: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_deleteOrphanedFile(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: DeleteOrphanedFileParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        if params.file_path.contains("..") || params.file_path.contains('\\') {
            return "{\"error\":\"Path traversal attempt detected\"}".to_string();
        }

        let full_path = path.join(&params.file_path);
        if full_path.exists() && full_path.is_file() {
            match tokio::fs::remove_file(&full_path).await {
                Ok(_) => {
                    // Check if parent dir is an empty history snapshot folder
                    if params.file_path.starts_with(".noda/history/") {
                        if let Some(parent) = full_path.parent() {
                            if parent.exists() && parent.is_dir() {
                                if let Ok(mut entries) = tokio::fs::read_dir(parent).await {
                                    let mut empty = true;
                                    while let Ok(Some(_)) = entries.next_entry().await {
                                        empty = false;
                                        break;
                                    }
                                    if empty {
                                        let _ = tokio::fs::remove_dir(parent).await;
                                    }
                                }
                            }
                        }
                    }
                    "{\"success\":true}".to_string()
                }
                Err(e) => format!("{{\"error\":\"Failed to delete file: {}\"}}", e),
            }
        } else {
            "{\"error\":\"File not found\"}".to_string()
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_deleteAllOrphanedRemnants(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        match noda_core::diagnostics::get_orphaned_remnants(&path).await {
            Ok(remnants) => {
                let count = remnants.files.len();
                let bytes = remnants.total_recovered_bytes;
                match noda_core::diagnostics::delete_orphaned_remnants(&path, remnants).await {
                    Ok(_) => serde_json::json!({
                        "deleted_count": count,
                        "freed_bytes": bytes
                    }).to_string(),
                    Err(e) => format!("{{\"error\":\"Failed to delete remnants: {}\"}}", e),
                }
            }
            Err(e) => format!("{{\"error\":\"Failed to scan remnants: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getOrphanedAttachments(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        match noda_core::diagnostics::get_orphaned_attachments(&path).await {
            Ok(list) => {
                let mapped = serde_json::json!(list.iter().map(|att| {
                    serde_json::json!({
                        "name": att.filename,
                        "size_bytes": att.size_bytes
                    })
                }).collect::<Vec<_>>());
                mapped.to_string()
            }
            Err(e) => format!("{{\"error\":\"Failed to get orphaned attachments: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_clearRemoteTrackingCache(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let (path, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (p, d)
    };

    let result = {
        let conn = db.conn.lock();
        match noda_core::diagnostics::clear_sync_cache(&conn, &path) {
            Ok(_) => "{\"success\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"Failed to clear cache: {}\"}}", e),
        }
    };

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_resetSyncQueue(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        let pending_count = match noda_core::sync::queue::SyncQueue::load(&path).await {
            Ok(q) => q.len(),
            Err(_) => 0,
        };

        match noda_core::diagnostics::clear_sync_queue(&path).await {
            Ok(_) => serde_json::json!({
                "success": true,
                "cleared_count": pending_count
            }).to_string(),
            Err(e) => format!("{{\"error\":\"Failed to clear queue: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_getSettings(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let path = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        match noda_core::settings::AppConfig::load(&path).await {
            Ok(config) => {
                let dto = shared::dtos::SettingsDto {
                    appearance: shared::dtos::AppearanceSettingsDto {
                        theme: config.appearance.theme,
                        accent_color: config.appearance.accent_color,
                    },
                    editor: shared::dtos::EditorSettingsDto {
                        font_size: config.editor.font_size,
                        typography: config.editor.typography,
                        show_word_count: config.editor.show_word_count,
                        auto_save_delay_ms: config.editor.auto_save_delay_ms,
                        default_daily_template: config.editor.default_daily_template,
                    },
                    sync: shared::dtos::SyncConfigDto {
                        webdav_url: config.sync.webdav_url,
                        webdav_username: config.sync.webdav_username,
                        webdav_password: config.sync.webdav_password,
                        interval_secs: config.sync.interval_secs,
                        device_name: config.sync.device_name,
                    },
                    history: shared::dtos::HistorySettingsDto {
                        retention_days: config.history.retention_days,
                        max_snapshots_per_note: config.history.max_snapshots_per_note,
                        empty_trash_after_days: config.history.empty_trash_after_days,
                        snapshot_interval_mins: config.history.snapshot_interval_mins,
                    },
                };
                serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
            }
            Err(e) => format!("{{\"error\":\"Failed to load settings: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_updateSettings(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let (sync_engine, path) = {
        let state = BRIDGE_STATE.read().unwrap();
        let engine = match &state.sync_engine {
            Some(e) => e.clone(),
            None => return error_string(&mut env, "Sync engine not initialized"),
        };
        let p = match &state.vault_path {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        (engine, p)
    };

    let result = get_runtime().block_on(async {
        let mut config = match noda_core::settings::AppConfig::load(&path).await {
            Ok(c) => c,
            Err(e) => return format!("{{\"error\":\"Failed to load settings: {}\"}}", e),
        };

        let value: serde_json::Value = match serde_json::from_str(&input) {
            Ok(v) => v,
            Err(e) => return format!("{{\"error\":\"Invalid JSON: {}\"}}", e),
        };

        if let Some(appearance) = value.get("appearance") {
            if let Some(theme) = appearance.get("theme").and_then(|t| t.as_str()) {
                config.appearance.theme = theme.to_string();
            }
            if let Some(accent) = appearance.get("accent_color").and_then(|a| a.as_str()) {
                config.appearance.accent_color = accent.to_string();
            }
        }

        if let Some(editor) = value.get("editor") {
            if let Some(size) = editor.get("font_size").and_then(|s| s.as_u64()) {
                config.editor.font_size = size as u32;
            }
            if let Some(typo) = editor.get("typography").and_then(|t| t.as_str()) {
                config.editor.typography = typo.to_string();
            }
            if let Some(word_count) = editor.get("show_word_count").and_then(|w| w.as_bool()) {
                config.editor.show_word_count = word_count;
            }
            if let Some(delay) = editor.get("auto_save_delay_ms").and_then(|d| d.as_u64()) {
                config.editor.auto_save_delay_ms = delay as u32;
            }
            if let Some(template) = editor.get("default_daily_template") {
                config.editor.default_daily_template = template.as_str().map(|s| s.to_string());
            }
        }

        if let Some(sync) = value.get("sync") {
            if let Some(url) = sync.get("webdav_url").and_then(|u| u.as_str()) {
                config.sync.webdav_url = url.to_string();
            }
            if let Some(username) = sync.get("webdav_username").and_then(|u| u.as_str()) {
                config.sync.webdav_username = username.to_string();
            }
            if let Some(password) = sync.get("webdav_password") {
                config.sync.webdav_password = password.as_str().map(|s| s.to_string());
            }
            if let Some(interval) = sync.get("interval_secs").and_then(|i| i.as_u64()) {
                config.sync.interval_secs = interval;
            }
            if let Some(device_name) = sync.get("device_name").and_then(|d| d.as_str()) {
                config.sync.device_name = device_name.to_string();
            }
        }

        if let Some(history) = value.get("history") {
            if let Some(retention) = history.get("retention_days").and_then(|r| r.as_u64()) {
                config.history.retention_days = retention as u32;
            }
            if let Some(max_snaps) = history.get("max_snapshots_per_note").and_then(|m| m.as_u64()) {
                config.history.max_snapshots_per_note = max_snaps as u32;
            }
            if let Some(empty_days) = history.get("empty_trash_after_days").and_then(|e| e.as_u64()) {
                config.history.empty_trash_after_days = empty_days as u32;
            }
            if let Some(interval) = history.get("snapshot_interval_mins").and_then(|i| i.as_u64()) {
                config.history.snapshot_interval_mins = interval as u32;
            }
        }

        sync_engine.set_config(config.sync.clone());

        match config.save(&path).await {
            Ok(_) => "{\"success\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"Failed to save settings: {}\"}}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_triggerDailyNote(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let (service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (s, d)
    };

    let result = get_runtime().block_on(async {
        use chrono::{Local, Utc};
        use ulid::Ulid;
        use noda_core::settings::AppConfig;
        use noda_core::database::queries;
        use noda_core::models::note::{Note, NoteId};

        #[derive(serde::Deserialize)]
        struct TriggerDailyNoteParams {
            date: Option<String>,
        }

        let local_now = Local::now();
        let params: TriggerDailyNoteParams = serde_json::from_str(&_input).unwrap_or(TriggerDailyNoteParams { date: None });
        let date_str = params.date.unwrap_or_else(|| local_now.format("%Y-%m-%d").to_string());
        let time_str = local_now.format("%H:%M").to_string();

        let existing_note = {
            let conn = db.conn.lock();
            match queries::find_daily_note_id(&conn, &date_str) {
                Ok(n) => n,
                Err(e) => return format!("{{\"error\":\"find_daily_note_id failed: {}\"}}", e),
            }
        };

        let note = if let Some(note_id) = existing_note {
            let mut note = match service.read_note(note_id).await {
                Ok(n) => n,
                Err(e) => return format!("{{\"error\":\"read_note failed: {}\"}}", e),
            };
            let section = format!("\n\n## 📌 {}\n\n", time_str);
            note.body.push_str(&section);
            note.inline_tags = Note::parse_inline_tags(&note.body);
            note.updated_at = Utc::now();

            if let Err(e) = service.write_note(&note).await {
                return format!("{{\"error\":\"write_note failed: {}\"}}", e);
            }

            {
                let conn = db.conn.lock();
                if let Err(e) = queries::upsert_note(&conn, &note, &note.file_path, true) {
                    return format!("{{\"error\":\"upsert_note failed: {}\"}}", e);
                }
            }

            note
        } else {
            let _ = service.create_folder("Daily Notes").await;

            let settings = AppConfig::load(service.base_path()).await.unwrap_or_default();
            let mut template_body = String::new();

            if let Some(ref template_id_str) = settings.editor.default_daily_template {
                if !template_id_str.is_empty() {
                    if let Ok(template_id) = Ulid::from_string(template_id_str) {
                        if let Ok(template_note) = service.read_note(NoteId(template_id)).await {
                            template_body = template_note.body;
                        }
                    }
                }
            }

            let body = template_body
                .replace("{{date}}", &date_str)
                .replace("{{time}}", &time_str);

            let new_id = NoteId::new();
            let relative_path = format!("Daily Notes/{}.md", new_id.0.to_string());
            let now = Utc::now();

            let note = Note {
                id: new_id,
                parent_id: None,
                title: date_str,
                body: body.clone(),
                color: None,
                pinned: false,
                tags: Vec::new(),
                inline_tags: Note::parse_inline_tags(&body),
                status: "active".to_string(),
                created_at: now,
                updated_at: now,
                file_path: relative_path.clone(),
                is_encrypted: false,
                dek_encrypted: None,
                dek_nonce: None,
            };

            if let Err(e) = service.write_note(&note).await {
                return format!("{{\"error\":\"write_note failed: {}\"}}", e);
            }

            {
                let conn = db.conn.lock();
                if let Err(e) = queries::upsert_note(&conn, &note, &relative_path, true) {
                    return format!("{{\"error\":\"upsert_note failed: {}\"}}", e);
                }
            }

            note
        };

        let dto = NoteDto::from(note);
        serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[derive(serde::Deserialize)]
struct ToggleTaskStatusParams {
    note_id: String,
    line_content: String,
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_toggleTaskStatus(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let params: ToggleTaskStatusParams = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (service, db) = {
        let state = BRIDGE_STATE.read().unwrap();
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        (s, d)
    };

    let result = get_runtime().block_on(async {
        use ulid::Ulid;
        use noda_core::models::note::{Note, NoteId};
        use noda_core::database::queries;
        use chrono::Utc;

        let parsed_id = match Ulid::from_string(&params.note_id) {
            Ok(u) => NoteId(u),
            Err(e) => return format!("{{\"error\":\"Invalid NoteId: {}\"}}", e),
        };

        let mut note = match service.read_note(parsed_id).await {
            Ok(n) => n,
            Err(e) => return format!("{{\"error\":\"Note read failed: {}\"}}", e),
        };

        let mut lines: Vec<String> = note.body.lines().map(|s| s.to_string()).collect();
        let mut modified = false;

        let cleaned_target = params.line_content.trim().to_lowercase();

        for line in &mut lines {
            let trimmed_line = line.trim();
            if trimmed_line.starts_with("- [ ]") || trimmed_line.starts_with("- [x]") || trimmed_line.starts_with("- [X]")
               || trimmed_line.starts_with("* [ ]") || trimmed_line.starts_with("* [x]") || trimmed_line.starts_with("* [X]")
               || trimmed_line.starts_with("+ [ ]") || trimmed_line.starts_with("+ [x]") || trimmed_line.starts_with("+ [X]") {
                
                let text_part = if trimmed_line.len() > 5 {
                    trimmed_line[5..].trim().to_lowercase()
                } else {
                    continue;
                };

                if text_part == cleaned_target {
                    if trimmed_line.contains("[ ]") {
                        *line = line.replace("[ ]", "[x]");
                    } else if trimmed_line.contains("[x]") {
                        *line = line.replace("[x]", "[ ]");
                    } else if trimmed_line.contains("[X]") {
                        *line = line.replace("[X]", "[ ]");
                    }
                    modified = true;
                    break;
                }
            }
        }

        if !modified {
            for line in &mut lines {
                let trimmed_line = line.trim();
                if trimmed_line.starts_with("- [ ") || trimmed_line.starts_with("* [ ") || trimmed_line.starts_with("+ [ ") {
                    if trimmed_line.to_lowercase().contains(&cleaned_target) {
                        if trimmed_line.contains("[ ]") {
                            *line = line.replace("[ ]", "[x]");
                        } else if trimmed_line.contains("[x]") {
                            *line = line.replace("[x]", "[ ]");
                        } else if trimmed_line.contains("[X]") {
                            *line = line.replace("[X]", "[ ]");
                        }
                        modified = true;
                        break;
                    }
                }
            }
        }

        if modified {
            note.body = lines.join("\n");
            note.inline_tags = Note::parse_inline_tags(&note.body);
            note.updated_at = Utc::now();

            if let Err(e) = service.write_note(&note).await {
                return format!("{{\"error\":\"write_note failed: {}\"}}", e);
            }

            let relative_path = note.file_path.clone();
            {
                let conn = db.conn.lock();
                if let Err(e) = queries::upsert_note(&conn, &note, &relative_path, true) {
                    return format!("{{\"error\":\"upsert_note failed: {}\"}}", e);
                }
            }
        }

        let dto = NoteDto::from(note);
        serde_json::to_string(&dto).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_listTagsWithCounts(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let _input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let db = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.database {
            Some(d) => d.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        use noda_core::database::queries;
        let tags = {
            let conn = db.conn.lock();
            match queries::list_tags_with_counts(&conn) {
                Ok(t) => t,
                Err(e) => return format!("{{\"error\":\"list_tags_with_counts failed: {}\"}}", e),
            }
        };

        let dtos: Vec<shared::dtos::TagWithCountDto> = tags
            .into_iter()
            .map(|(name, count)| shared::dtos::TagWithCountDto {
                name,
                count: count as u32,
            })
            .collect();

        serde_json::to_string(&dtos).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    });

    env.new_string(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_check_1url_1history(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
) -> jstring {
    let url_str: String = match env.get_string(&url) {
        Ok(s) => s.into(),
        Err(_) => return error_string(&mut env, "Invalid URL string"),
    };

    let db = {
        let state = BRIDGE_STATE.read().unwrap();
        match &state.database {
            Some(d) => d.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        }
    };

    let result = get_runtime().block_on(async {
        match noda_core::vault::clipper::check_url_history(&db, &url_str).await {
            Ok(Some(id)) => format!("{{\"exists\":true,\"note_id\":\"{}\"}}", id),
            Ok(None) => "{\"exists\":false}".to_string(),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        }
    });

    env.new_string(result).unwrap_or_else(|_| env.new_string("{\"error\":\"JNI string creation failed\"}").unwrap()).into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_clipUrl(
    mut env: JNIEnv,
    _class: JClass,
    input_json: JString,
) -> jstring {
    let input: String = match parse_string(&mut env, &input_json) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let payload: noda_core::vault::clipper::ClipperPayload = match serde_json::from_str(&input) {
        Ok(p) => p,
        Err(e) => return error_string(&mut env, &format!("Parse error: {}", e)),
    };

    let (service, db, base_path) = {
        let state = BRIDGE_STATE.read().unwrap();
        let s = match &state.vault_service {
            Some(v) => v.clone(),
            None => return error_string(&mut env, "Vault service not initialized"),
        };
        let d = match &state.database {
            Some(db_ref) => db_ref.clone(),
            None => return error_string(&mut env, "Database not initialized"),
        };
        let p = match &state.vault_path {
            Some(path) => path.clone(),
            None => return error_string(&mut env, "Vault path not initialized"),
        };
        (s, d, p)
    };

    let result = get_runtime().block_on(async {
        match noda_core::vault::clipper::clip_url(&service, &db, &base_path, payload).await {
            Ok(true) => "{\"success\":true}".to_string(),
            Ok(false) => "{\"success\":false}".to_string(),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        }
    });

    env.new_string(result).unwrap_or_else(|_| env.new_string("{\"error\":\"JNI string creation failed\"}").unwrap()).into_raw()
}

