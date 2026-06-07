use tauri::State;
use shared::AppError;
use crate::state::AppState;
use noda_core::models::note::NoteId;
use noda_core::database::queries;
use noda_core::trash::soft_delete as core_soft_delete;
use ulid::Ulid;

#[tauri::command]
pub async fn list_folders(
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let folders = service.list_folders().map_err(AppError::from)?;
    Ok(folders)
}

#[tauri::command]
pub async fn create_folder(
    state: State<'_, AppState>,
    rel_path: String,
) -> Result<(), AppError> {
    let service = {
        let guard = state.vault_service.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    service.create_folder(&rel_path).await.map_err(|e| AppError {
        code: "IO_ERROR".to_string(),
        message: format!("Failed to create folder: {}", e),
    })?;

    Ok(())
}

#[tauri::command]
pub async fn delete_folder(
    state: State<'_, AppState>,
    rel_path: String,
) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

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

    // 1. Find all notes inside this folder recursively
    let notes_to_delete = {
        let conn = db.conn.lock();
        let all_notes = queries::list_notes(&conn).map_err(AppError::from)?;
        let prefix = format!("{}/", rel_path);
        all_notes
            .into_iter()
            .filter(|note| note.file_path.starts_with(&prefix) || note.file_path == rel_path)
            .collect::<Vec<_>>()
    };

    // 2. Soft-delete each note (moves to trash and removes from SQLite)
    for note in notes_to_delete {
        // Soft delete on disk
        core_soft_delete(&vault_path, &note.file_path).await
            .map_err(AppError::from)?;

        // Remove from SQLite
        {
            let conn = db.conn.lock();
            queries::delete_note(&conn, note.id, true).map_err(AppError::from)?;
        }
    }

    // 3. Physically delete the directory
    service.delete_folder(&rel_path).await.map_err(|e| AppError {
        code: "IO_ERROR".to_string(),
        message: format!("Failed to delete physical folder: {}", e),
    })?;

    Ok(())
}

#[tauri::command]
pub async fn move_note(
    state: State<'_, AppState>,
    id: String,
    target_dir: String,
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

    // 1. Get existing file_path from DB
    let old_rel_path = {
        let conn = db.conn.lock();
        let note_opt = queries::get_note(&conn, note_id).map_err(AppError::from)?;
        let note = note_opt.ok_or_else(|| AppError {
            code: "NOT_FOUND".to_string(),
            message: format!("Note not found in database: {}", id),
        })?;
        note.file_path
    };

    // 2. Compute new relative path
    let filename = format!("{}.md", id);
    let new_rel_path = if target_dir.is_empty() {
        filename
    } else {
        format!("{}/{}", target_dir.trim_end_matches('/'), filename)
    };

    if old_rel_path != new_rel_path {
        // 3. Rename file on disk
        service.rename_note_file(&old_rel_path, &new_rel_path).await
            .map_err(AppError::from)?;

        // 4. Update file_path in SQLite
        let conn = db.conn.lock();
        queries::update_note_file_path(&conn, note_id, &new_rel_path)
            .map_err(AppError::from)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn move_folder(
    state: State<'_, AppState>,
    src_dir: String,
    target_dir: String,
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

    let base_path = service.base_path();
    let src_path = base_path.join(&src_dir);

    // Compute the destination path
    let src_folder_name = src_path.file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AppError {
            code: "INVALID_PATH".to_string(),
            message: "Source folder path has an invalid name".to_string(),
        })?;

    let new_folder_rel_path = if target_dir.is_empty() {
        src_folder_name.to_string()
    } else {
        format!("{}/{}", target_dir.trim_end_matches('/'), src_folder_name)
    };

    let dest_path = base_path.join(&new_folder_rel_path);

    if src_path == dest_path {
        return Ok(());
    }

    if !src_path.exists() {
        return Err(AppError {
            code: "NOT_FOUND".to_string(),
            message: format!("Source directory does not exist: {:?}", src_path),
        });
    }

    if dest_path.exists() {
        return Err(AppError {
            code: "ALREADY_EXISTS".to_string(),
            message: format!("Destination already exists: {:?}", dest_path),
        });
    }

    // 1. Physically rename the directory on disk
    // Ensure the target parent directory exists
    if let Some(parent) = dest_path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| AppError {
                code: "IO_ERROR".to_string(),
                message: format!("Failed to create destination parent: {}", e),
            })?;
        }
    }

    tokio::fs::rename(&src_path, &dest_path).await.map_err(|e| AppError {
        code: "IO_ERROR".to_string(),
        message: format!("Failed to rename folder on disk: {}", e),
    })?;

    // 2. Recursively update file_path in SQLite for all notes inside that directory subtree
    let conn = db.conn.lock();
    let all_notes = queries::list_notes(&conn).map_err(AppError::from)?;
    let src_prefix = format!("{}/", src_dir);

    for note in all_notes {
        if note.file_path.starts_with(&src_prefix) {
            let relative_suffix = &note.file_path[src_prefix.len()..];
            let new_path = format!("{}/{}", new_folder_rel_path, relative_suffix);
            queries::update_note_file_path(&conn, note.id, &new_path)
                .map_err(AppError::from)?;
        } else if note.file_path == src_dir {
            queries::update_note_file_path(&conn, note.id, &new_folder_rel_path)
                .map_err(AppError::from)?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn rename_folder(
    state: State<'_, AppState>,
    src_dir: String,
    new_name: String,
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

    let base_path = service.base_path();
    let src_path = base_path.join(&src_dir);

    // Determine target relative path
    let parts: Vec<&str> = src_dir.split('/').collect();
    let mut new_parts = parts.clone();
    if new_parts.is_empty() {
        return Err(AppError {
            code: "INVALID_PATH".to_string(),
            message: "Source folder path is empty".to_string(),
        });
    }
    new_parts.pop();
    new_parts.push(&new_name);
    let new_folder_rel_path = new_parts.join("/");

    let dest_path = base_path.join(&new_folder_rel_path);

    if src_path == dest_path {
        return Ok(());
    }

    if !src_path.exists() {
        return Err(AppError {
            code: "NOT_FOUND".to_string(),
            message: format!("Source directory does not exist: {:?}", src_path),
        });
    }

    if dest_path.exists() {
        return Err(AppError {
            code: "ALREADY_EXISTS".to_string(),
            message: format!("Destination already exists: {:?}", dest_path),
        });
    }

    // 1. Physically rename the directory on disk
    tokio::fs::rename(&src_path, &dest_path).await.map_err(|e| AppError {
        code: "IO_ERROR".to_string(),
        message: format!("Failed to rename folder on disk: {}", e),
    })?;

    // 2. Recursively update file_path in SQLite for all notes inside that directory subtree
    let conn = db.conn.lock();
    let all_notes = queries::list_notes(&conn).map_err(AppError::from)?;
    let src_prefix = format!("{}/", src_dir);

    for note in all_notes {
        if note.file_path.starts_with(&src_prefix) {
            let relative_suffix = &note.file_path[src_prefix.len()..];
            let new_path = format!("{}/{}", new_folder_rel_path, relative_suffix);
            queries::update_note_file_path(&conn, note.id, &new_path)
                .map_err(AppError::from)?;
        } else if note.file_path == src_dir {
            queries::update_note_file_path(&conn, note.id, &new_folder_rel_path)
                .map_err(AppError::from)?;
        }
    }

    Ok(())
}
