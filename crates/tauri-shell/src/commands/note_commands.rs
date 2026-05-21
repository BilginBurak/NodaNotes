use tauri::State;
use shared::AppError;
use shared::dtos::{NoteDto, NoteListItemDto};
use crate::state::AppState;
use noda_core::models::note::{Note, NoteId};
use noda_core::database::queries;
use noda_core::history;
use ulid::Ulid;
use chrono::Utc;

#[tauri::command]
pub async fn create_note(
    state: State<'_, AppState>,
    title: String,
    body: String,
    parent_id: Option<String>,
    color: Option<String>,
    pinned: bool,
    tags: Vec<String>,
) -> Result<NoteDto, AppError> {
    let service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    let parsed_parent = match parent_id {
        Some(ref s) if !s.is_empty() => Some(NoteId(Ulid::from_string(s).map_err(|e| AppError {
            code: "INVALID_ID".to_string(),
            message: format!("Invalid parent NoteId: {}", e),
        })?)),
        _ => None,
    };

    let now = Utc::now();
    let note = Note {
        id: NoteId::new(),
        parent_id: parsed_parent,
        title,
        body,
        color,
        pinned,
        tags,
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
    };

    // 1. Write to local disk
    service.write_note(&note).await.map_err(AppError::from)?;

    // 2. Write to SQLite database
    let relative_path = format!("{}.md", note.id.0.to_string());
    {
        let conn = db.conn.lock();
        queries::upsert_note(&conn, &note, &relative_path, "dummy_hash")
            .map_err(AppError::from)?;
    }

    Ok(NoteDto::from(note))
}

#[tauri::command]
pub async fn get_note(
    state: State<'_, AppState>,
    id: String,
) -> Result<NoteDto, AppError> {
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    let note_id = NoteId(Ulid::from_string(&id).map_err(|e| AppError {
        code: "INVALID_ID".to_string(),
        message: format!("Invalid NoteId: {}", e),
    })?);

    let note = {
        let conn = db.conn.lock();
        queries::get_note(&conn, note_id)
            .map_err(AppError::from)?
            .ok_or_else(|| AppError {
                code: "NOT_FOUND".to_string(),
                message: format!("Note not found in DB: {}", id),
            })?
    };

    Ok(NoteDto::from(note))
}

#[tauri::command]
pub async fn update_note(
    state: State<'_, AppState>,
    id: String,
    title: String,
    body: String,
    parent_id: Option<String>,
    color: Option<String>,
    pinned: bool,
    tags: Vec<String>,
) -> Result<NoteDto, AppError> {
    let service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    let note_id = NoteId(Ulid::from_string(&id).map_err(|e| AppError {
        code: "INVALID_ID".to_string(),
        message: format!("Invalid NoteId: {}", e),
    })?);

    let parsed_parent = match parent_id {
        Some(ref s) if !s.is_empty() => Some(NoteId(Ulid::from_string(s).map_err(|e| AppError {
            code: "INVALID_ID".to_string(),
            message: format!("Invalid parent NoteId: {}", e),
        })?)),
        _ => None,
    };

    let now = Utc::now();
    
    // Get existing note first to preserve created_at
    let vault_path = service.base_path().clone();

    // Get existing note first to preserve created_at and for version history
    let existing_note_full = match service.read_note(note_id).await {
        Ok(n) => n,
        Err(_) => {
            return Err(AppError {
                code: "NOT_FOUND".to_string(),
                message: format!("Note file not found for: {}", id),
            });
        }
    };

    let note = Note {
        id: note_id,
        parent_id: parsed_parent,
        title,
        body,
        color,
        pinned,
        tags,
        status: existing_note_full.status.clone(),
        created_at: existing_note_full.created_at,
        updated_at: now,
    };

    // Check if content actually changed
    let content_changed = existing_note_full.title != note.title || existing_note_full.body != note.body || existing_note_full.color != note.color || existing_note_full.pinned != note.pinned || existing_note_full.tags != note.tags;

    if content_changed {
        // Take a snapshot of the PREVIOUS state before we overwrite it
        let _ = history::snapshot(&vault_path, &existing_note_full).await;
    }

    // 1. Write to local disk
    service.write_note(&note).await.map_err(AppError::from)?;

    // 2. Update in DB
    let relative_path = format!("{}.md", note.id.0.to_string());
    {
        let conn = db.conn.lock();
        queries::upsert_note(&conn, &note, &relative_path, "dummy_hash")
            .map_err(AppError::from)?;
    }

    Ok(NoteDto::from(note))
}

#[tauri::command]
pub async fn rename_note(
    state: State<'_, AppState>,
    id: String,
    new_title: String,
) -> Result<NoteDto, AppError> {
    let service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    let note_id = NoteId(Ulid::from_string(&id).map_err(|e| AppError {
        code: "INVALID_ID".to_string(),
        message: format!("Invalid NoteId: {}", e),
    })?);

    let mut note = {
        let conn = db.conn.lock();
        queries::get_note(&conn, note_id)
            .map_err(AppError::from)?
            .ok_or_else(|| AppError {
                code: "NOT_FOUND".to_string(),
                message: format!("Note not found: {}", id),
            })?
    };

    note.title = new_title;
    note.updated_at = Utc::now();

    // 1. Write back to disk
    service.write_note(&note).await.map_err(AppError::from)?;

    // 2. Update DB
    let relative_path = format!("{}.md", note.id.0.to_string());
    {
        let conn = db.conn.lock();
        queries::upsert_note(&conn, &note, &relative_path, "dummy_hash")
            .map_err(AppError::from)?;
    }

    Ok(NoteDto::from(note))
}

#[tauri::command]
pub async fn delete_note(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    let note_id = NoteId(Ulid::from_string(&id).map_err(|e| AppError {
        code: "INVALID_ID".to_string(),
        message: format!("Invalid NoteId: {}", e),
    })?);

    // 1. Delete from local disk
    service.delete_note(note_id).await.map_err(AppError::from)?;

    // 2. Delete from DB
    {
        let conn = db.conn.lock();
        queries::delete_note(&conn, note_id).map_err(AppError::from)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn list_notes(
    state: State<'_, AppState>,
) -> Result<Vec<NoteListItemDto>, AppError> {
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let notes = {
        let conn = db.conn.lock();
        queries::list_notes(&conn).map_err(AppError::from)?
    };
    
    let dtos = notes.into_iter().map(NoteListItemDto::from).collect();
    Ok(dtos)
}
