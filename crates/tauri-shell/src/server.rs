use std::net::SocketAddr;
use std::sync::Arc;
use axum::{
    body::Body,
    extract::{Path, Query, State, ConnectInfo},
    http::{header, Method, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
    response::sse::{Event, Sse},
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
use rusqlite::OptionalExtension;

#[derive(RustEmbed)]
#[folder = "../../frontend/build"]
struct Asset;

#[derive(Clone)]
pub struct ServerState {
    pub app_handle: AppHandle,
    pub token: Arc<parking_lot::RwLock<String>>,
    pub mcp_sessions: Arc<tokio::sync::RwLock<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<axum::response::sse::Event>>>>,
}

#[derive(serde::Deserialize)]
struct AuthRequestPayload {
    device_name: String,
}

#[derive(serde::Serialize)]
struct AuthRequestResponse {
    id: String,
}

#[derive(serde::Serialize)]
struct AuthStatusResponse {
    status: String,
    token: Option<String>,
}

#[derive(serde::Deserialize)]
struct AuthStatusParams {
    id: String,
}

async fn auth_request_handler(
    State(state): State<ServerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AuthRequestPayload>,
) -> impl IntoResponse {
    let app_state = state.app_handle.state::<AppState>();
    let db = match &*app_state.database.read() {
        Some(d) => d.clone(),
        None => return (StatusCode::BAD_REQUEST, "No active database connection").into_response(),
    };

    let id = uuid::Uuid::new_v4().to_string();
    let ip_address = addr.ip().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let conn = db.conn.lock();
    if let Err(e) = conn.execute(
        "INSERT INTO trusted_devices (id, device_name, ip_address, status, device_token, created_at) VALUES (?1, ?2, ?3, 'pending', NULL, ?4)",
        [&id, &payload.device_name, &ip_address, &now],
    ) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to register request: {}", e)).into_response();
    }

    (StatusCode::OK, Json(AuthRequestResponse { id })).into_response()
}

async fn auth_status_handler(
    State(state): State<ServerState>,
    Query(params): Query<AuthStatusParams>,
) -> impl IntoResponse {
    let app_state = state.app_handle.state::<AppState>();
    let db = match &*app_state.database.read() {
        Some(d) => d.clone(),
        None => return (StatusCode::BAD_REQUEST, "No active database connection").into_response(),
    };

    let conn = db.conn.lock();
    let mut stmt = match conn.prepare("SELECT status, device_token FROM trusted_devices WHERE id = ?1") {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let result = stmt.query_row([&params.id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    });

    match result {
        Ok((status, token)) => {
            (StatusCode::OK, Json(AuthStatusResponse { status, token })).into_response()
        }
        Err(_) => {
            (StatusCode::NOT_FOUND, "Device request not found").into_response()
        }
    }
}

/// Start the Axum server on localhost:4040.
pub async fn run_server(app_handle: AppHandle, _app_state: AppState, token: Arc<parking_lot::RwLock<String>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mcp_sessions = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let state = ServerState {
        app_handle,
        token: token.clone(),
        mcp_sessions,
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_origin(AllowOrigin::predicate(|origin, _parts| {
            let origin_bytes = origin.as_bytes();
            if origin_bytes.starts_with(b"http://localhost") || origin_bytes.starts_with(b"http://127.0.0.1") {
                return true;
            }
            if origin_bytes.starts_with(b"http://192.168.") 
                || origin_bytes.starts_with(b"http://10.") 
                || origin_bytes.starts_with(b"http://172.") {
                return true;
            }
            if origin_bytes.starts_with(b"safari-extension://") 
                || origin_bytes.starts_with(b"safari-web-extension://") 
                || origin_bytes.starts_with(b"chrome-extension://")
                || origin_bytes.starts_with(b"moz-extension://") {
                return true;
            }
            false
        }));

    let state_for_middleware = state.clone();
    // Routes requiring Auth token
    let api_routes: Router<ServerState> = Router::new()
        .route("/clipper", post(clipper_handler))
        .route("/clipper/check", get(clipper_check_handler))
        .route("/attachments/upload", post(attachments_upload_handler))
        .layer(axum::extract::DefaultBodyLimit::disable())
        .route("/rpc", post(rpc_handler))
        .route("/validate", get(validate_token_handler))
        .route("/mcp/sse", get(mcp_sse_handler).post(mcp_post_handler))
        .route("/mcp/message", post(mcp_message_handler))
        .route_layer(middleware::from_fn(move |req, next| {
            let state = state_for_middleware.clone();
            async move {
                auth_middleware(state, req, next).await
            }
        }));

    let app = Router::new()
        .nest("/api", api_routes)
        .route("/api/auth/request", post(auth_request_handler))
        .route("/api/auth/status", get(auth_status_handler))
        .route("/attachments/:filename", get(attachments_handler))
        .fallback(static_asset_fallback)
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 4040));
    tracing::info!("Unified Noda Daemon Server starting on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}

async fn validate_token_handler() -> impl IntoResponse {
    StatusCode::OK
}

async fn auth_middleware(
    state: ServerState,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if req.method() == Method::OPTIONS {
        return next.run(req).await;
    }
    if let Some(auth_header) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let req_token = &auth_str[7..];
                
                // 1. Check primary master token
                let is_master = {
                    let master_token = state.token.read();
                    req_token == *master_token
                };
                if is_master {
                    return next.run(req).await;
                }

                // 2. Check database trusted_devices approved list
                let app_state = state.app_handle.state::<AppState>();
                let is_trusted = {
                    let db_guard = app_state.database.read();
                    if let Some(db) = &*db_guard {
                        let conn = db.conn.lock();
                        let query = "SELECT count(*) FROM trusted_devices WHERE device_token = ?1 AND status = 'approved'";
                        conn.query_row(query, [req_token], |row| row.get::<_, i32>(0)).unwrap_or(0) > 0
                    } else {
                        false
                    }
                };

                if is_trusted {
                    return next.run(req).await;
                }
            }
        }
    }
    StatusCode::UNAUTHORIZED.into_response()
}

/// Fallback route: Serves embedded static assets or SPA index.html for client-side routing.
async fn static_asset_fallback(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<ServerState>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');
    let is_spa = !path.contains('.') || path == "index.html";

    if is_spa {
        // 1. Extract token from query or cookie
        let mut token_val = "".to_string();
        if let Some(query) = req.uri().query() {
            for part in query.split('&') {
                let kv: Vec<&str> = part.split('=').collect();
                if kv.len() == 2 && kv[0] == "token" {
                    token_val = kv[1].to_string();
                    break;
                }
            }
        }

        if token_val.is_empty() {
            if let Some(cookie_header) = req.headers().get(header::COOKIE) {
                if let Ok(cookie_str) = cookie_header.to_str() {
                    for cookie in cookie_str.split(';') {
                        let parts: Vec<&str> = cookie.trim().split('=').collect();
                        if parts.len() == 2 && parts[0] == "noda_token" {
                            token_val = parts[1].to_string();
                            break;
                        }
                    }
                }
            }
        }

        // 2. Validate token or check if local loopback
        let is_local = addr.ip().is_loopback();
        let is_valid = if is_local {
            true
        } else if token_val.is_empty() {
            false
        } else {
            // Check primary master token
            let master_token = state.token.read();
            if token_val == *master_token {
                true
            } else {
                // Check sqlite database
                let app_state = state.app_handle.state::<AppState>();
                let db_guard = app_state.database.read();
                if let Some(db) = &*db_guard {
                    let conn = db.conn.lock();
                    let query = "SELECT count(*) FROM trusted_devices WHERE device_token = ?1 AND status = 'approved'";
                    conn.query_row(query, [&token_val], |row| row.get::<_, i32>(0)).unwrap_or(0) > 0
                } else {
                    false
                }
            }
        };

        if is_valid {
            if let Some(index_asset) = Asset::get("index.html") {
                let mut html = String::from_utf8(index_asset.data.into_owned()).unwrap_or_default();
                let actual_token = if is_local {
                    (*state.token.read()).clone()
                } else {
                    token_val
                };

                let script = format!("<script>window.__NODA_TOKEN__ = \"{}\";</script>", actual_token);
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
            // Serve the pairing.html page
            Response::builder()
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(include_str!("pairing.html")))
                .unwrap()
                .into_response()
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
    if token != *state.token.read() {
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
struct ClipperCheckQuery {
    title: String,
}

#[derive(serde::Serialize)]
struct ClipperCheckResponse {
    exists: bool,
}

async fn clipper_check_handler(
    State(state): State<ServerState>,
    Query(query): Query<ClipperCheckQuery>,
) -> impl IntoResponse {
    let app_state = state.app_handle.state::<AppState>();
    let db = {
        let guard = app_state.database.read();
        match guard.clone() {
            Some(d) => d,
            None => return (StatusCode::BAD_REQUEST, "No active database connection").into_response(),
        }
    };

    let conn = db.conn.lock();
    let exists = match conn.query_row(
        "SELECT 1 FROM notes WHERE title = ?1 AND status = 'active' LIMIT 1",
        rusqlite::params![query.title],
        |_| Ok(true)
    ).optional() {
        Ok(Some(true)) => true,
        _ => false,
    };

    (StatusCode::OK, Json(ClipperCheckResponse { exists })).into_response()
}

#[derive(serde::Deserialize)]
struct UploadQuery {
    filename: Option<String>,
}

#[derive(serde::Serialize)]
struct UploadResponse {
    url: String,
}

async fn attachments_upload_handler(
    State(state): State<ServerState>,
    Query(query): Query<UploadQuery>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let app_state = state.app_handle.state::<AppState>();
    let service = {
        let guard = app_state.vault_service.read();
        match guard.clone() {
            Some(s) => s,
            None => return (StatusCode::BAD_REQUEST, "No active vault is open").into_response(),
        }
    };

    let filename = query.filename
        .or_else(|| {
            headers.get("x-filename")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "screenshot.png".to_string());

    let vault_path = service.base_path();

    match noda_core::attachments::store_attachment_bytes(vault_path, &body, &filename).await {
        Ok(url) => (StatusCode::OK, Json(UploadResponse { url })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save attachment: {}", e)).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ClipperPayload {
    title: String,
    url: String,
    content_markdown: String,
    tags: Vec<String>,
    append: Option<bool>,
    author: Option<String>,
    published_date: Option<String>,
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

    let existing_note_info = {
        let conn = db.conn.lock();
        conn.query_row(
            "SELECT id, file_path FROM notes WHERE title = ?1 AND status = 'active' LIMIT 1",
            rusqlite::params![payload.title],
            |row| {
                let id_str: String = row.get(0)?;
                let file_path: String = row.get(1)?;
                let id = ulid::Ulid::from_string(&id_str)
                    .map(NoteId)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
                Ok((id, file_path))
            }
        ).optional().unwrap_or(None)
    };

    let now = Utc::now();
    let note = if payload.append.unwrap_or(false) && existing_note_info.is_some() {
        let (existing_id, _) = existing_note_info.unwrap();
        let mut n = match service.read_note(existing_id).await {
            Ok(note) => note,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read existing note: {}", e)).into_response(),
        };

        let marker = "<!-- noda-webclipper -->";
        let now_local = chrono::Local::now().format("%d.%m.%Y %H:%M").to_string();
        let appended_text = format!(
            "\n***\n*Appended on {}:*\n\n{}\n\n",
            now_local,
            payload.content_markdown
        );

        if let Some(idx) = n.body.find(marker) {
            let (before, after) = n.body.split_at(idx);
            let mut new_body = String::new();
            new_body.push_str(before.trim_end());
            new_body.push_str(&appended_text);
            new_body.push_str(after);
            n.body = new_body;
        } else {
            n.body = format!("{}\n\n{}", n.body.trim_end(), appended_text);
        }
        n.updated_at = now;
        n
    } else {
        let id = NoteId::new();
        let file_path = format!("clipper/{}.md", id.0.to_string());
        
        let author = payload.author.filter(|a| !a.is_empty()).unwrap_or_else(|| "Unknown".to_string());
        let published = payload.published_date.filter(|p| !p.is_empty()).unwrap_or_else(|| "Unknown".to_string());
        let added_date = chrono::Local::now().format("%d.%m.%Y %H:%M").to_string();

        let source_link = if payload.url.is_empty() {
            "Unknown".to_string()
        } else {
            format!("[Link]({})", payload.url)
        };

        let footer = format!(
            "<!-- noda-webclipper -->\n\n---\n*Added via NodaNotes #webclipper*\n\n**Source:** {}  \n**Author:** {}  \n**Published:** {}  \n**Added:** {}",
            source_link,
            author,
            published,
            added_date
        );

        let body = format!("{}\n\n{}", payload.content_markdown, footer);
        let inline_tags = Note::parse_inline_tags(&body);

        Note {
            id,
            parent_id: None,
            title: payload.title,
            body: body.clone(),
            color: None,
            pinned: false,
            tags: payload.tags,
            inline_tags,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
            file_path,
            is_encrypted: false,
            dek_encrypted: None,
            dek_nonce: None,
            outline: Some(Note::parse_outline(&body)),
        }
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
struct OpenUrlArgs {
    url: String,
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
struct SetMasterPasswordArgs {
    password: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnlockVaultSessionArgs {
    password: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToggleNoteEncryptionArgs {
    id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetVaultTimeoutSettingArgs {
    timeout: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeMasterPasswordArgs {
    old_password: String,
    new_password: String,
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
        "open_external_url" => rpc_match!(payload, OpenUrlArgs, |a: OpenUrlArgs| commands::vault_commands::open_external_url(a.url)),


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

        // Device authorization commands
        "get_trusted_devices" => rpc_match_no_args!(commands::device_commands::get_trusted_devices(tauri_state.clone())),
        "approve_device" => rpc_match!(payload, IdArgs, |a: IdArgs| commands::device_commands::approve_device(tauri_state.clone(), a.id)),
        "revoke_device" => rpc_match!(payload, IdArgs, |a: IdArgs| commands::device_commands::revoke_device(tauri_state.clone(), a.id)),
        "regenerate_daemon_token" => rpc_match_no_args!(commands::device_commands::regenerate_daemon_token(tauri_state.clone())),

        // Cryptography and Vault commands
        "set_master_password" => rpc_match!(payload, SetMasterPasswordArgs, |a: SetMasterPasswordArgs| async move {
            commands::crypto_commands::set_master_password(tauri_state.clone(), a.password).await
        }),
        "unlock_vault_session" => rpc_match!(payload, UnlockVaultSessionArgs, |a: UnlockVaultSessionArgs| async move {
            commands::crypto_commands::unlock_vault_session(tauri_state.clone(), a.password).await
        }),
        "lock_vault_instantly" => rpc_match_no_args!(async move {
            commands::crypto_commands::lock_vault_instantly().await
        }),
        "toggle_note_encryption" => rpc_match!(payload, ToggleNoteEncryptionArgs, |a: ToggleNoteEncryptionArgs| async move {
            commands::crypto_commands::toggle_note_encryption(tauri_state.clone(), a.id).await
        }),
        "is_vault_session_unlocked" => rpc_match_no_args!(commands::crypto_commands::is_vault_session_unlocked()),
        "is_vault_configured" => rpc_match_no_args!(commands::crypto_commands::is_vault_configured(tauri_state.clone())),
        "get_vault_timeout_setting" => rpc_match_no_args!(commands::crypto_commands::get_vault_timeout_setting(tauri_state.clone())),
        "set_vault_timeout_setting" => rpc_match!(payload, SetVaultTimeoutSettingArgs, |a: SetVaultTimeoutSettingArgs| async move {
            commands::crypto_commands::set_vault_timeout_setting(tauri_state.clone(), a.timeout).await
        }),
        "change_master_password" => rpc_match!(payload, ChangeMasterPasswordArgs, |a: ChangeMasterPasswordArgs| async move {
            commands::crypto_commands::change_master_password(tauri_state.clone(), a.old_password, a.new_password).await
        }),


        _ => Err(AppError {
            code: "UNKNOWN_ACTION".to_string(),
            message: format!("The action '{}' is not supported via browser RPC", action),
        }),
    }
}

// ==========================================
// MCP (Model Context Protocol) Integration
// ==========================================

#[derive(serde::Serialize, Debug)]
pub struct McpNoteRow {
    pub note_id: String,
    pub relative_path: String,
    pub title: String,
    pub last_modified: Option<i64>,
    pub char_size: Option<i64>,
    pub outline: Option<String>,
}

pub struct McpStream {
    pub session_id: String,
    pub sessions: Arc<tokio::sync::RwLock<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<axum::response::sse::Event>>>>,
    pub rx: tokio_stream::wrappers::UnboundedReceiverStream<axum::response::sse::Event>,
}

impl futures_util::stream::Stream for McpStream {
    type Item = Result<axum::response::sse::Event, std::convert::Infallible>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.rx).poll_next(cx).map(|opt| opt.map(Ok))
    }
}

impl Drop for McpStream {
    fn drop(&mut self) {
        let session_id = self.session_id.clone();
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            let mut lock = sessions.write().await;
            lock.remove(&session_id);
            tracing::info!("Cleared MCP session context: {}", session_id);
        });
    }
}

#[derive(serde::Deserialize)]
struct McpMessageParams {
    session_id: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

#[derive(serde::Serialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<serde_json::Value>,
    id: serde_json::Value,
}

#[derive(serde::Serialize)]
struct McpToolListResponse {
    tools: Vec<McpTool>,
}

#[derive(serde::Serialize)]
#[allow(non_snake_case)]
struct McpTool {
    name: String,
    description: String,
    inputSchema: serde_json::Value,
}

async fn mcp_sse_handler(
    State(state): State<ServerState>,
) -> impl IntoResponse {
    let session_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

    // Send initial endpoint event
    let initial_endpoint_url = format!("/api/mcp/message?session_id={}", session_id);
    let _ = tx.send(Event::default().event("endpoint").data(initial_endpoint_url));

    {
        let mut sessions = state.mcp_sessions.write().await;
        sessions.insert(session_id.clone(), tx);
    }

    let stream = McpStream {
        session_id,
        sessions: state.mcp_sessions.clone(),
        rx: tokio_stream::wrappers::UnboundedReceiverStream::new(rx),
    };

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

async fn mcp_message_handler(
    State(state): State<ServerState>,
    Query(params): Query<McpMessageParams>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let tx = {
        let sessions = state.mcp_sessions.read().await;
        match sessions.get(&params.session_id).cloned() {
            Some(t) => t,
            None => return (StatusCode::BAD_REQUEST, "Invalid or expired MCP session").into_response(),
        }
    };

    let jsonrpc_response = handle_jsonrpc_request(request, &state).await;

    if let Ok(res_str) = serde_json::to_string(&jsonrpc_response) {
        let _ = tx.send(Event::default().event("message").data(res_str));
    }

    StatusCode::OK.into_response()
}

async fn mcp_post_handler(
    State(state): State<ServerState>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let jsonrpc_response = handle_jsonrpc_request(request, &state).await;
    Json(jsonrpc_response).into_response()
}

async fn handle_jsonrpc_request(
    request: JsonRpcRequest,
    state: &ServerState,
) -> JsonRpcResponse {
    let req_id = request.id.clone().unwrap_or(serde_json::Value::Null);

    match request.method.as_str() {
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "NodaNotes-MCP",
                    "version": "1.0.0"
                }
            });
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(result),
                error: None,
                id: req_id,
            }
        }
        "tools/list" => {
            let tools = vec![
                McpTool {
                    name: "search_notes".to_string(),
                    description: "This tool searches notes in the vault matching the query against relative path, title, and heading outlines. It returns relative_path, last_modified, and outline metadata for matching notes. You are STRICTLY FORBIDDEN from guessing or inventing search results that are not returned by the database view.".to_string(),
                    inputSchema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search term to query against titles, paths, and outlines."
                            }
                        },
                        "required": ["query"]
                    }),
                },
                McpTool {
                    name: "read_note".to_string(),
                    description: "This tool reads note data by ID. If the requested note ID is missing or returns null from the database view, throw a NOT_FOUND error instantly. You are STRICTLY FORBIDDEN from guessing file content strings, generating phantom text parameters, or assuming structure.".to_string(),
                    inputSchema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "note_id": {
                                "type": "string",
                                "description": "The unique ULID of the note."
                            }
                        },
                        "required": ["note_id"]
                    }),
                },
                McpTool {
                    name: "write_note".to_string(),
                    description: "This tool writes a new markdown note onto disk. You are STRICTLY FORBIDDEN from creating a file outside the sandbox boundaries, or guessing parameters. Ensure all arguments are explicitly provided.".to_string(),
                    inputSchema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "The title of the new note."
                            },
                            "content": {
                                "type": "string",
                                "description": "The raw markdown content of the new note."
                            },
                            "category": {
                                "type": "string",
                                "description": "Optional category/folder prefix path (e.g. 'Projects' or 'Work/Sub')."
                            }
                        },
                        "required": ["title", "content"]
                    }),
                },
                McpTool {
                    name: "edit_note".to_string(),
                    description: "This tool appends text or overwrites note content. You are STRICTLY FORBIDDEN from assuming previous note content if the note does not exist, or inventing note text not provided by the client.".to_string(),
                    inputSchema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "note_id": {
                                "type": "string",
                                "description": "The unique ULID of the note to edit."
                            },
                            "append_text": {
                                "type": "string",
                                "description": "Optional text to append to the end of the note."
                            },
                            "overwrite_content": {
                                "type": "string",
                                "description": "Optional content to overwrite the entire note with."
                            }
                        },
                        "required": ["note_id"]
                    }),
                },
                McpTool {
                    name: "delete_note".to_string(),
                    description: "This tool soft-deletes a note by its note_id, archiving it to trash. You are STRICTLY FORBIDDEN from trying to delete arbitrary files outside the vault boundaries.".to_string(),
                    inputSchema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "note_id": {
                                "type": "string",
                                "description": "The unique ULID of the note to delete."
                            }
                        },
                        "required": ["note_id"]
                    }),
                },
            ];

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::to_value(McpToolListResponse { tools }).unwrap()),
                error: None,
                id: req_id,
            }
        }
        "tools/call" => {
            let params = request.params.unwrap_or(serde_json::Value::Null);
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(serde_json::Value::Null);

            match handle_mcp_tool_call(name, arguments, state).await {
                Ok(res) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(serde_json::json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&res).unwrap() }] })),
                    error: None,
                    id: req_id,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(serde_json::json!({
                        "code": -32603,
                        "message": e,
                    })),
                    id: req_id,
                },
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(serde_json::json!({
                "code": -32601,
                "message": format!("Method not found: {}", request.method)
            })),
            id: req_id,
        }
    }
}

async fn handle_mcp_tool_call(
    name: &str,
    args: serde_json::Value,
    state: &ServerState,
) -> Result<serde_json::Value, String> {
    let app_state = state.app_handle.state::<AppState>();
    
    // Get vault root
    let vault_path = {
        let guard = app_state.vault_path.read();
        guard.clone().ok_or("No vault open")?
    };
    let canonical_vault = vault_path.canonicalize().map_err(|e| format!("Failed to canonicalize vault root: {}", e))?;

    // Get database
    let db = {
        let guard = app_state.database.read();
        guard.clone().ok_or("No database connection")?
    };

    match name {
        "search_notes" => {
            let query = args.get("query")
                .and_then(|v| v.as_str())
                .ok_or("Missing required query argument")?;

            let conn = db.conn.lock();
            let mut stmt = conn.prepare(
                "SELECT note_id, relative_path, title, last_modified, char_size, outline \
                  FROM mcp_vault_view \
                  WHERE title LIKE ?1 OR outline LIKE ?2 OR relative_path LIKE ?3"
             ).map_err(|e| e.to_string())?;

            let query_param = format!("%{}%", query);
            let rows = stmt.query_map([&query_param, &query_param, &query_param], |row| {
                let last_modified_str: Option<String> = row.get("last_modified")?;
                let last_modified = last_modified_str.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.timestamp())
                        .ok()
                });
                
                Ok(McpNoteRow {
                    note_id: row.get("note_id")?,
                    relative_path: row.get("relative_path")?,
                    title: row.get("title")?,
                    last_modified,
                    char_size: row.get("char_size")?,
                    outline: row.get("outline")?,
                })
            }).map_err(|e| e.to_string())?;

            let mut results = Vec::new();
            for r in rows {
                if let Ok(item) = r {
                    results.push(item);
                }
            }

            Ok(serde_json::to_value(results).unwrap())
        }
        "read_note" => {
            let note_id = args.get("note_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing required note_id argument")?;

            let note_id_parsed = NoteId(ulid::Ulid::from_string(note_id).map_err(|e| e.to_string())?);
            let relative_path = {
                let conn = db.conn.lock();
                queries::get_note(&conn, note_id_parsed)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "NOT_FOUND".to_string())?
                    .file_path
            };

            let target_path = vault_path.join(&relative_path);
            let canonical_target = target_path.canonicalize().map_err(|e| format!("NOT_FOUND: {}", e))?;
            if !canonical_target.starts_with(&canonical_vault) {
                return Err("ACCESS_DENIED: Path traversal detected".to_string());
            }

            let content = tokio::fs::read_to_string(&canonical_target)
                .await
                .map_err(|e| e.to_string())?;

            Ok(serde_json::json!({ "content": content }))
        }
        "write_note" => {
            let title = args.get("title")
                .and_then(|v| v.as_str())
                .ok_or("Missing required title argument")?;
            let content = args.get("content")
                .and_then(|v| v.as_str())
                .ok_or("Missing required content argument")?;
            let category = args.get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let clean_title = title.replace("/", "_").replace("..", "_");
            let relative_path = if category.is_empty() {
                format!("{}.md", clean_title)
            } else {
                let clean_cat = category.trim_start_matches('/').trim_end_matches('/');
                format!("{}/{}.md", clean_cat, clean_title)
            };

            let target_path = vault_path.join(&relative_path);
            let parent_dir = target_path.parent().ok_or("Invalid path: no parent directory")?;
            tokio::fs::create_dir_all(parent_dir).await.map_err(|e| e.to_string())?;
            let canonical_parent = parent_dir.canonicalize().map_err(|e| e.to_string())?;
            let resolved_target = canonical_parent.join(target_path.file_name().ok_or("Invalid filename")?);
            
            if !resolved_target.starts_with(&canonical_vault) {
                return Err("ACCESS_DENIED: Path traversal detected".to_string());
            }

            let id = NoteId::new();
            let now = Utc::now();
            let outline = Note::parse_outline(content);

            let note = Note {
                id,
                parent_id: None,
                title: title.to_string(),
                body: content.to_string(),
                color: None,
                pinned: false,
                tags: Vec::new(),
                inline_tags: Note::parse_inline_tags(content),
                status: "active".to_string(),
                created_at: now,
                updated_at: now,
                file_path: relative_path.clone(),
                is_encrypted: false,
                dek_encrypted: None,
                dek_nonce: None,
                outline: Some(outline),
            };

            let service = {
                let guard = app_state.vault_service.read();
                guard.clone().ok_or("No vault service open")?
            };
            service.write_note(&note).await.map_err(|e| e.to_string())?;

            {
                let conn = db.conn.lock();
                queries::upsert_note(&conn, &note, &note.file_path, true).map_err(|e| e.to_string())?;
            }

            // Instantly snapshot Version 0
            noda_core::history::snapshot(&vault_path, &note, "Version 0").await.map_err(|e| e.to_string())?;

            Ok(serde_json::json!({ "id": id.0.to_string(), "file_path": relative_path }))
        }
        "edit_note" => {
            let note_id = args.get("note_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing required note_id argument")?;
            let append_text = args.get("append_text").and_then(|v| v.as_str());
            let overwrite_content = args.get("overwrite_content").and_then(|v| v.as_str());

            let note_id_parsed = NoteId(ulid::Ulid::from_string(note_id).map_err(|e| e.to_string())?);
            let existing_note = {
                let conn = db.conn.lock();
                queries::get_note(&conn, note_id_parsed)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "NOT_FOUND: Note not found in DB".to_string())?
            };

            let target_path = vault_path.join(&existing_note.file_path);
            let canonical_target = target_path.canonicalize().map_err(|e| e.to_string())?;
            if !canonical_target.starts_with(&canonical_vault) {
                return Err("ACCESS_DENIED: Path traversal detected".to_string());
            }

            let mut new_body = existing_note.body.clone();
            if let Some(overwrite) = overwrite_content {
                new_body = overwrite.to_string();
            } else if let Some(append) = append_text {
                new_body.push_str(append);
            }

            let outline = Note::parse_outline(&new_body);
            let mut updated_note = existing_note.clone();
            updated_note.body = new_body;
            updated_note.outline = Some(outline);
            updated_note.updated_at = Utc::now();

            // Native snapshot BEFORE writing markdown buffer to physical storage drive
            if !existing_note.is_encrypted {
                noda_core::history::snapshot(&vault_path, &existing_note, "Backup").await.map_err(|e| e.to_string())?;
            }

            let service = {
                let guard = app_state.vault_service.read();
                guard.clone().ok_or("No vault service open")?
            };
            service.write_note(&updated_note).await.map_err(|e| e.to_string())?;

            {
                let conn = db.conn.lock();
                queries::upsert_note(&conn, &updated_note, &updated_note.file_path, true).map_err(|e| e.to_string())?;
            }

            Ok(serde_json::json!({ "id": note_id, "status": "edited" }))
        }
        "delete_note" => {
            let note_id = args.get("note_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing required note_id argument")?;

            let note_id_parsed = NoteId(ulid::Ulid::from_string(note_id).map_err(|e| e.to_string())?);
            let relative_path = {
                let conn = db.conn.lock();
                queries::get_note(&conn, note_id_parsed)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "NOT_FOUND: Note not found in DB".to_string())?
                    .file_path
            };

            let target_path = vault_path.join(&relative_path);
            let canonical_target = target_path.canonicalize().map_err(|e| e.to_string())?;
            if !canonical_target.starts_with(&canonical_vault) {
                return Err("ACCESS_DENIED: Path traversal detected".to_string());
            }

            noda_core::trash::soft_delete(&vault_path, &relative_path).await.map_err(|e| e.to_string())?;

            {
                let conn = db.conn.lock();
                queries::delete_note(&conn, note_id_parsed, true).map_err(|e| e.to_string())?;
            }

            Ok(serde_json::json!({ "id": note_id, "status": "deleted" }))
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}
