use std::net::SocketAddr;
use axum::{
    body::Body,
    extract::{Path, Query, Request, State},
    http::{header, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rust_embed::RustEmbed;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tauri::{AppHandle, Manager};
use shared::AppError;
use serde_json::Value;
use crate::commands;
use crate::state::AppState;
use chrono::Utc;
use noda_core::models::note::{Note, NoteId};
use noda_core::database::queries;

#[derive(RustEmbed)]
#[folder = "../../frontend/build"]
struct Asset;

#[derive(Clone)]
pub struct ServerState {
    pub app_handle: AppHandle,
    pub token: String,
}

/// Start the Axum server on localhost:4040.
pub async fn run_server(app_handle: AppHandle, _app_state: AppState, token: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = ServerState {
        app_handle,
        token: token.clone(),
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_origin(AllowOrigin::predicate(|origin, _parts| {
            let origin_bytes = origin.as_bytes();
            if origin_bytes.starts_with(b"http://localhost") || origin_bytes.starts_with(b"http://127.0.0.1") {
                return true;
            }
            if origin_bytes.starts_with(b"safari-extension://") 
                || origin_bytes.starts_with(b"safari-web-extension://") 
                || origin_bytes.starts_with(b"chrome-extension://") {
                return true;
            }
            false
        }));

    // Routes requiring Auth token
    let api_routes = Router::new()
        .route("/clipper", post(clipper_handler))
        .route("/rpc", post(rpc_handler))
        .route("/validate", get(validate_token_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let app = Router::new()
        .nest("/api", api_routes)
        .route("/attachments/:filename", get(attachments_handler))
        .fallback(static_asset_fallback)
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 4040));
    tracing::info!("Unified Noda Daemon Server starting on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn validate_token_handler() -> impl IntoResponse {
    StatusCode::OK
}

async fn auth_middleware(
    State(state): State<ServerState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.method() == Method::OPTIONS {
        return Ok(next.run(req).await);
    }
    if let Some(auth_header) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let req_token = &auth_str[7..];
                if req_token == state.token {
                    return Ok(next.run(req).await);
                }
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

/// Fallback route: Serves embedded static assets or SPA index.html for client-side routing.
async fn static_asset_fallback(
    State(state): State<ServerState>,
    req: Request,
) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');
    
    // Serve index.html (with token injection) for empty path, index.html, or SPA routes (no dot in path)
    let is_spa = !path.contains('.') || path == "index.html";

    if is_spa {
        if let Some(index_asset) = Asset::get("index.html") {
            let mut html = String::from_utf8(index_asset.data.into_owned()).unwrap_or_default();
            // Inject token into Svelte window context securely
            let script = format!("<script>window.__NODA_TOKEN__ = \"{}\";</script>", state.token);
            if let Some(pos) = html.find("<head>") {
                html.insert_str(pos + 6, &script);
            } else {
                html = format!("{}{}", script, html);
            }

            Response::builder()
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(html))
                .unwrap()
                .into_response()
        } else {
            StatusCode::NOT_FOUND.into_response()
        }
    } else {
        // Try serving physical static asset
        if let Some(asset) = Asset::get(path) {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(asset.data))
                .unwrap()
                .into_response()
        } else {
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

/// Serves attachments from the vault securely.
async fn attachments_handler(
    Path(filename): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    let token = params.get("token").cloned().unwrap_or_default();
    if token != state.token {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let app_state = state.app_handle.state::<AppState>();
    let vault_path = {
        let guard = app_state.vault_path.read();
        match &*guard {
            Some(p) => p.clone(),
            None => return (StatusCode::BAD_REQUEST, "No vault open").into_response(),
        }
    };

    let uri = format!("noda://attachments/{}", filename);
    match noda_core::protocol::serve_file(vault_path, &uri).await {
        Ok((bytes, mime)) => {
            Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(bytes))
                .unwrap()
        }
        Err(e) => {
            (StatusCode::NOT_FOUND, format!("Error serving file: {}", e)).into_response()
        }
    }
}

#[derive(serde::Deserialize)]
struct ClipperPayload {
    title: String,
    url: String,
    content_markdown: String,
    tags: Vec<String>,
}

/// Ingest clipped notes securely.
async fn clipper_handler(
    State(state): State<ServerState>,
    Json(payload): Json<ClipperPayload>,
) -> impl IntoResponse {
    let app_state = state.app_handle.state::<AppState>();
    
    let service = {
        let guard = app_state.vault_service.read();
        match guard.clone() {
            Some(s) => s,
            None => return (StatusCode::BAD_REQUEST, "No active vault is open").into_response(),
        }
    };
    
    let db = {
        let guard = app_state.database.read();
        match guard.clone() {
            Some(d) => d,
            None => return (StatusCode::BAD_REQUEST, "No active database connection").into_response(),
        }
    };

    let id = NoteId::new();
    let now = Utc::now();
    let file_path = format!("clipper/{}.md", id.0.to_string());
    
    let mut body = payload.content_markdown;
    if !payload.url.is_empty() {
        body = format!("{}\n\nSource: [{}]({})", body, payload.url, payload.url);
    }

    let inline_tags = Note::parse_inline_tags(&body);
    let note = Note {
        id,
        parent_id: None,
        title: payload.title,
        body,
        color: None,
        pinned: false,
        tags: payload.tags,
        inline_tags,
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
        file_path: file_path.clone(),
    };

    // 1. Write note to disk
    if let Err(e) = service.write_note(&note).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write note: {}", e)).into_response();
    }

    // 2. Read physical file details
    let abs_path = service.find_note_path(note.id);
    let size = match std::fs::metadata(&abs_path) {
        Ok(m) => m.len(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read note metadata: {}", e)).into_response(),
    };

    // 3. Compute hash and build markdown representation
    let markdown = match note.to_markdown() {
        Ok(md) => md,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize note: {}", e)).into_response(),
    };
    let raw_hash = xxhash_rust::xxh3::xxh3_64(markdown.as_bytes());
    let hash_hex = format!("{:016x}", raw_hash);

    // 4. Autocommit database insert (upsert note & direct sync_file_states insert)
    {
        let conn = db.conn.lock();
        if let Err(e) = queries::upsert_note(&conn, &note, &note.file_path, false) {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to index note: {}", e)).into_response();
        }

        if let Err(e) = conn.execute(
            "INSERT INTO sync_file_states (path, etag, last_modified, size, local_updated_at, hash, is_dirty, retry_count, sync_error) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 0, NULL) \
             ON CONFLICT(path) DO UPDATE SET \
                etag = excluded.etag, \
                last_modified = excluded.last_modified, \
                size = excluded.size, \
                local_updated_at = excluded.local_updated_at, \
                hash = excluded.hash, \
                is_dirty = 1, \
                retry_count = 0, \
                sync_error = NULL",
            rusqlite::params![
                note.file_path,
                None::<String>,
                None::<String>,
                size as i64,
                now.to_rfc3339(),
                hash_hex,
            ],
        ) {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to log sync state: {}", e)).into_response();
        }
    }

    (StatusCode::OK, "Clipped successfully").into_response()
}

#[derive(serde::Deserialize)]
struct RpcEnvelope {
    action: String,
    payload: Value,
}

/// Route Browser RPC Translation calls to the tauri commands.
async fn rpc_handler(
    State(state): State<ServerState>,
    Json(envelope): Json<RpcEnvelope>,
) -> impl IntoResponse {
    match handle_rpc_action(&envelope.action, envelope.payload, &state.app_handle).await {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(e)).into_response(),
    }
}

// Structs to deserialize RPC payloads using camelCase matching Svelte's ts arguments
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathArgs {
    path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelPathArgs {
    rel_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateNoteArgs {
    title: String,
    body: String,
    parent_id: Option<String>,
    color: Option<String>,
    pinned: bool,
    tags: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdArgs {
    id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateNoteArgs {
    id: String,
    title: String,
    body: String,
    parent_id: Option<String>,
    color: Option<String>,
    pinned: bool,
    tags: Vec<String>,
    trigger_snapshot: bool,
    snapshot_reason: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameNoteArgs {
    id: String,
    new_title: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToggleTaskStatusArgs {
    note_id: String,
    line_content: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchArgs {
    query: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSyncConfigArgs {
    config: noda_core::sync::SyncConfig,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NoteIdArgs {
    note_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotArgs {
    note_id: String,
    timestamp: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConflictNoteArgs {
    archived_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveConflictKeepRemoteArgs {
    note_id: String,
    archived_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddAttachmentArgs {
    source_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddAttachmentBytesArgs {
    file_name: String,
    bytes: Vec<u8>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NameArgs {
    name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveSettingsArgs {
    config: noda_core::settings::AppConfig,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteOrphanedAttachmentsArgs {
    filenames: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteDuplicateNoteFileArgs {
    relative_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteOrphanedRemnantsArgs {
    remnants: noda_core::diagnostics::OrphanedRemnants,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateFolderArgs {
    rel_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteFolderArgs {
    rel_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveNoteArgs {
    id: String,
    target_dir: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveFolderArgs {
    src_dir: String,
    target_dir: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameFolderArgs {
    src_dir: String,
    new_name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportNoteArgs {
    source_path: String,
    target_dir: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportNoteFromContentArgs {
    title: String,
    content: String,
    target_dir: Option<String>,
}

macro_rules! rpc_match {
    ($payload:expr, $args_type:ty, $expr:expr) => {{
        let args: $args_type = serde_json::from_value($payload).map_err(|e| AppError {
            code: "INVALID_ARGS".to_string(),
            message: format!("Payload deserialization error: {}", e),
        })?;
        let result = $expr(args).await?;
        Ok(serde_json::to_value(result).unwrap())
    }};
}

macro_rules! rpc_match_no_args {
    ($expr:expr) => {{
        let result = $expr.await?;
        Ok(serde_json::to_value(result).unwrap())
    }};
}

async fn handle_rpc_action(
    action: &str,
    payload: Value,
    app_handle: &AppHandle,
) -> Result<Value, AppError> {
    let tauri_state = app_handle.state::<AppState>();

    match action {
        // Vault commands
        "open_vault" => rpc_match!(payload, PathArgs, |a: PathArgs| commands::vault_commands::open_vault(tauri_state.clone(), a.path, app_handle.clone())),
        "create_vault" => rpc_match!(payload, PathArgs, |a: PathArgs| commands::vault_commands::create_vault(tauri_state.clone(), a.path, app_handle.clone())),
        "get_vault_info" => rpc_match_no_args!(commands::vault_commands::get_vault_info(tauri_state.clone())),
        "reveal_in_file_manager" => rpc_match!(payload, RelPathArgs, |a: RelPathArgs| commands::vault_commands::reveal_in_file_manager(tauri_state.clone(), a.rel_path)),

        // Note commands
        "create_note" => rpc_match!(payload, CreateNoteArgs, |a: CreateNoteArgs| commands::note_commands::create_note(tauri_state.clone(), a.title, a.body, a.parent_id, a.color, a.pinned, a.tags)),
        "get_note" => rpc_match!(payload, IdArgs, |a: IdArgs| commands::note_commands::get_note(tauri_state.clone(), a.id)),
        "get_note_metadata" => rpc_match!(payload, IdArgs, |a: IdArgs| commands::note_commands::get_note_metadata(tauri_state.clone(), a.id)),
        "update_note" => rpc_match!(payload, UpdateNoteArgs, |a: UpdateNoteArgs| commands::note_commands::update_note(tauri_state.clone(), a.id, a.title, a.body, a.parent_id, a.color, a.pinned, a.tags, a.trigger_snapshot, a.snapshot_reason)),
        "rename_note" => rpc_match!(payload, RenameNoteArgs, |a: RenameNoteArgs| commands::note_commands::rename_note(tauri_state.clone(), a.id, a.new_title)),
        "delete_note" => rpc_match!(payload, IdArgs, |a: IdArgs| async move {
            commands::note_commands::delete_note(tauri_state.clone(), a.id).await.map(|_| ())
        }),
        "toggle_task_status" => rpc_match!(payload, ToggleTaskStatusArgs, |a: ToggleTaskStatusArgs| commands::note_commands::toggle_task_status(tauri_state.clone(), a.note_id, a.line_content)),
        "list_notes" => rpc_match_no_args!(commands::note_commands::list_notes(tauri_state.clone())),
        "list_tags_with_counts" => rpc_match_no_args!(commands::note_commands::list_tags_with_counts(tauri_state.clone())),
        "trigger_daily_note" => rpc_match_no_args!(commands::note_commands::trigger_daily_note(tauri_state.clone())),

        // Search commands
        "search_notes" => rpc_match!(payload, SearchArgs, |a: SearchArgs| commands::search_commands::search_notes(tauri_state.clone(), a.query)),

        // Sync commands
        "start_sync" => rpc_match_no_args!(async move {
            commands::sync_commands::start_sync(tauri_state.clone()).await.map(|_| ())
        }),
        "stop_sync" => rpc_match_no_args!(async move {
            commands::sync_commands::stop_sync(tauri_state.clone()).await.map(|_| ())
        }),
        "sync_now" => rpc_match_no_args!(commands::sync_commands::sync_now(tauri_state.clone())),
        "get_sync_status" => rpc_match_no_args!(commands::sync_commands::get_sync_status(tauri_state.clone())),
        "update_sync_config" => rpc_match!(payload, UpdateSyncConfigArgs, |a: UpdateSyncConfigArgs| async move {
            commands::sync_commands::update_sync_config(tauri_state.clone(), a.config).await.map(|_| ())
        }),
        "validate_sync_config" => rpc_match!(payload, UpdateSyncConfigArgs, |a: UpdateSyncConfigArgs| async move {
            commands::sync_commands::validate_sync_config(tauri_state.clone(), a.config).await.map(|_| ())
        }),
        "get_sync_config" => rpc_match_no_args!(commands::sync_commands::get_sync_config(tauri_state.clone())),
        "list_conflicts" => rpc_match_no_args!(commands::sync_commands::list_conflicts(tauri_state.clone())),
        "get_conflict_note" => rpc_match!(payload, ConflictNoteArgs, |a: ConflictNoteArgs| commands::sync_commands::get_conflict_note(tauri_state.clone(), a.archived_path)),
        "resolve_conflict_keep_local" => rpc_match!(payload, ConflictNoteArgs, |a: ConflictNoteArgs| async move {
            commands::sync_commands::resolve_conflict_keep_local(tauri_state.clone(), a.archived_path).await.map(|_| ())
        }),
        "resolve_conflict_keep_remote" => rpc_match!(payload, ResolveConflictKeepRemoteArgs, |a: ResolveConflictKeepRemoteArgs| async move {
            commands::sync_commands::resolve_conflict_keep_remote(tauri_state.clone(), a.note_id, a.archived_path).await.map(|_| ())
        }),

        // History commands
        "list_snapshots" => rpc_match!(payload, NoteIdArgs, |a: NoteIdArgs| commands::history_commands::list_snapshots(tauri_state.clone(), a.note_id)),
        "restore_snapshot" => rpc_match!(payload, SnapshotArgs, |a: SnapshotArgs| commands::history_commands::restore_snapshot(tauri_state.clone(), a.note_id, a.timestamp)),
        "compare_snapshot" => rpc_match!(payload, SnapshotArgs, |a: SnapshotArgs| commands::history_commands::compare_snapshot(tauri_state.clone(), a.note_id, a.timestamp)),
        "delete_snapshot" => rpc_match!(payload, SnapshotArgs, |a: SnapshotArgs| async move {
            commands::history_commands::delete_snapshot(tauri_state.clone(), a.note_id, a.timestamp).await.map(|_| ())
        }),

        // Trash commands
        "list_trash" => rpc_match_no_args!(commands::trash_commands::list_trash(tauri_state.clone())),
        "trash_note" => rpc_match!(payload, IdArgs, |a: IdArgs| async move {
            commands::trash_commands::trash_note(tauri_state.clone(), a.id).await.map(|_| ())
        }),
        "restore_from_trash" => rpc_match!(payload, NoteIdArgs, |a: NoteIdArgs| async move {
            commands::trash_commands::restore_from_trash(tauri_state.clone(), a.note_id).await.map(|_| ())
        }),
        "permanent_delete" => rpc_match!(payload, NoteIdArgs, |a: NoteIdArgs| async move {
            commands::trash_commands::permanent_delete(tauri_state.clone(), a.note_id).await.map(|_| ())
        }),
        "get_trash_note" => rpc_match!(payload, IdArgs, |a: IdArgs| commands::trash_commands::get_trash_note(tauri_state.clone(), a.id)),

        // Attachment commands
        "add_attachment" => rpc_match!(payload, AddAttachmentArgs, |a: AddAttachmentArgs| commands::attachment_commands::add_attachment(tauri_state.clone(), a.source_path)),
        "add_attachment_bytes" => rpc_match!(payload, AddAttachmentBytesArgs, |a: AddAttachmentBytesArgs| commands::attachment_commands::add_attachment_bytes(tauri_state.clone(), a.file_name, a.bytes)),
        "list_attachments" => rpc_match_no_args!(commands::attachment_commands::list_attachments(tauri_state.clone())),
        "delete_attachment" => rpc_match!(payload, NameArgs, |a: NameArgs| async move {
            commands::attachment_commands::delete_attachment(tauri_state.clone(), a.name).await.map(|_| ())
        }),
        "list_attachments_with_metadata" => rpc_match_no_args!(commands::attachment_commands::list_attachments_with_metadata(tauri_state.clone())),

        // Settings commands
        "get_settings" => rpc_match_no_args!(commands::settings_commands::get_settings(tauri_state.clone())),
        "save_settings" => rpc_match!(payload, SaveSettingsArgs, |a: SaveSettingsArgs| async move {
            commands::settings_commands::save_settings(tauri_state.clone(), a.config).await.map(|_| ())
        }),

        // Maintenance commands
        "rebuild_database_cache" => rpc_match_no_args!(async move {
            commands::maintenance_commands::rebuild_database_cache(tauri_state.clone()).await.map(|_| ())
        }),
        "vacuum_database_cache" => rpc_match_no_args!(async move {
            commands::maintenance_commands::vacuum_database_cache(tauri_state.clone()).await.map(|_| ())
        }),
        "get_orphaned_attachments" => rpc_match_no_args!(commands::maintenance_commands::get_orphaned_attachments(tauri_state.clone())),
        "delete_orphaned_attachments" => rpc_match!(payload, DeleteOrphanedAttachmentsArgs, |a: DeleteOrphanedAttachmentsArgs| async move {
            commands::maintenance_commands::delete_orphaned_attachments(tauri_state.clone(), a.filenames).await.map(|_| ())
        }),
        "clear_sync_queue" => rpc_match_no_args!(async move {
            commands::maintenance_commands::clear_sync_queue(tauri_state.clone()).await.map(|_| ())
        }),
        "clear_sync_cache" => rpc_match_no_args!(async move {
            commands::maintenance_commands::clear_sync_cache(tauri_state.clone()).await.map(|_| ())
        }),
        "get_duplicate_notes" => rpc_match_no_args!(commands::maintenance_commands::get_duplicate_notes(tauri_state.clone())),
        "delete_duplicate_note_file" => rpc_match!(payload, DeleteDuplicateNoteFileArgs, |a: DeleteDuplicateNoteFileArgs| async move {
            commands::maintenance_commands::delete_duplicate_note_file(tauri_state.clone(), a.relative_path).await.map(|_| ())
        }),
        "get_orphaned_remnants" => rpc_match_no_args!(commands::maintenance_commands::get_orphaned_remnants(tauri_state.clone())),
        "delete_orphaned_remnants" => rpc_match!(payload, DeleteOrphanedRemnantsArgs, |a: DeleteOrphanedRemnantsArgs| async move {
            commands::maintenance_commands::delete_orphaned_remnants(tauri_state.clone(), a.remnants).await.map(|_| ())
        }),
        "delete_orphaned_file" => rpc_match!(payload, DeleteDuplicateNoteFileArgs, |a: DeleteDuplicateNoteFileArgs| async move {
            commands::maintenance_commands::delete_orphaned_file(tauri_state.clone(), a.relative_path).await.map(|_| ())
        }),

        // Folder commands
        "list_folders" => rpc_match_no_args!(commands::folder_commands::list_folders(tauri_state.clone())),
        "create_folder" => rpc_match!(payload, CreateFolderArgs, |a: CreateFolderArgs| async move {
            commands::folder_commands::create_folder(tauri_state.clone(), a.rel_path).await.map(|_| ())
        }),
        "delete_folder" => rpc_match!(payload, DeleteFolderArgs, |a: DeleteFolderArgs| async move {
            commands::folder_commands::delete_folder(tauri_state.clone(), a.rel_path).await.map(|_| ())
        }),
        "move_note" => rpc_match!(payload, MoveNoteArgs, |a: MoveNoteArgs| async move {
            commands::folder_commands::move_note(tauri_state.clone(), a.id, a.target_dir).await.map(|_| ())
        }),
        "move_folder" => rpc_match!(payload, MoveFolderArgs, |a: MoveFolderArgs| async move {
            commands::folder_commands::move_folder(tauri_state.clone(), a.src_dir, a.target_dir).await.map(|_| ())
        }),
        "rename_folder" => rpc_match!(payload, RenameFolderArgs, |a: RenameFolderArgs| async move {
            commands::folder_commands::rename_folder(tauri_state.clone(), a.src_dir, a.new_name).await.map(|_| ())
        }),

        // Import commands
        "import_note" => rpc_match!(payload, ImportNoteArgs, |a: ImportNoteArgs| commands::note_commands::import_note(tauri_state.clone(), a.source_path, a.target_dir)),
        "import_note_from_content" => rpc_match!(payload, ImportNoteFromContentArgs, |a: ImportNoteFromContentArgs| commands::note_commands::import_note_from_content(tauri_state.clone(), a.title, a.content, a.target_dir)),

        _ => Err(AppError {
            code: "UNKNOWN_ACTION".to_string(),
            message: format!("The action '{}' is not supported via browser RPC", action),
        }),
    }
}
