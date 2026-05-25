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
    let id = NoteId::new();
    let note = Note {
        id,
        parent_id: parsed_parent,
        title,
        body,
        color,
        pinned,
        tags,
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
        file_path: format!("{}.md", id.0.to_string()),
    };

    // 1. Write to local disk
    service.write_note(&note).await.map_err(AppError::from)?;

    // 2. Write to SQLite database
    let relative_path = note.file_path.clone();
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
        file_path: existing_note_full.file_path.clone(),
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
    let relative_path = note.file_path.clone();
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
    let relative_path = note.file_path.clone();
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

#[tauri::command]
pub async fn import_note(
    state: State<'_, AppState>,
    source_path: String,
    target_dir: Option<String>,
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

    let source_path_buf = std::path::PathBuf::from(&source_path);
    if !source_path_buf.exists() {
        return Err(AppError {
            code: "FILE_NOT_FOUND".to_string(),
            message: format!("Source file does not exist: {}", source_path),
        });
    }

    // Determine target folder path
    let vault_root = service.base_path();
    let folder_path = match &target_dir {
        Some(dir) if !dir.is_empty() => vault_root.join(dir),
        _ => vault_root.clone(),
    };

    // Ensure the folder exists
    if !folder_path.exists() {
        tokio::fs::create_dir_all(&folder_path).await.map_err(|e| AppError {
            code: "IO_ERROR".to_string(),
            message: format!("Failed to create folder: {}", e),
        })?;
    }

    // Determine temp copied file name
    let filename = source_path_buf.file_name().ok_or_else(|| AppError {
        code: "INVALID_PATH".to_string(),
        message: "Invalid source file path".to_string(),
    })?;

    // To prevent overwriting any existing file with the same name before we convert it, 
    // let's give it a temporary unique name, e.g. import_xxxx.md
    let temp_filename = format!("import_{}_{}", ulid::Ulid::new().to_string(), filename.to_string_lossy());
    let temp_dest_path = folder_path.join(&temp_filename);

    // Copy file to vault folder
    tokio::fs::copy(&source_path_buf, &temp_dest_path).await.map_err(|e| AppError {
        code: "IO_ERROR".to_string(),
        message: format!("Failed to copy source file: {}", e),
    })?;

    // Now call the core scanner function to convert/import it!
    // Note: It will parse the content, determine title/dates/id, write standard {ulid}.md file, 
    // and delete the temp file temp_dest_path!
    let note = noda_core::vault::parse_or_create_note_from_file(&temp_dest_path, vault_root).await
        .map_err(|e| AppError {
            code: "IMPORT_FAILED".to_string(),
            message: format!("Core import failed: {}", e),
        })?;

    // Make sure the title is the original file stem if it was empty/defaulted to the temp filename stem
    let mut final_note = note;
    let original_stem = source_path_buf.file_stem().and_then(|s| s.to_str()).unwrap_or("Imported Note").to_string();
    if final_note.title.starts_with("import_") {
        final_note.title = original_stem;
        
        // Write the note again with the corrected title in its frontmatter
        service.write_note(&final_note).await.map_err(AppError::from)?;
    }

    // Insert/upsert into SQLite DB cache
    let rel_path = final_note.file_path.clone();
    {
        let conn = db.conn.lock();
        queries::upsert_note(&conn, &final_note, &rel_path, "dummy_hash")
            .map_err(AppError::from)?;
    }

    Ok(NoteDto::from(final_note))
}

#[tauri::command]
pub async fn import_note_from_content(
    state: State<'_, AppState>,
    title: String,
    content: String,
    target_dir: Option<String>,
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

    let target_dir_str = target_dir.unwrap_or_default();
    
    // Call the public core parsing function
    let note = noda_core::vault::parse_or_create_note_from_content(&title, &content, &target_dir_str).await
        .map_err(|e| AppError {
            code: "IMPORT_FAILED".to_string(),
            message: format!("Core import failed: {}", e),
        })?;

    // 1. Write to vault disk
    service.write_note(&note).await.map_err(AppError::from)?;

    // 2. Insert into SQLite DB cache
    let rel_path = note.file_path.clone();
    {
        let conn = db.conn.lock();
        queries::upsert_note(&conn, &note, &rel_path, "dummy_hash")
            .map_err(AppError::from)?;
    }

    Ok(NoteDto::from(note))
}
