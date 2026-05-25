use tauri::State;
use shared::AppError;
use crate::state::AppState;
use noda_core::diagnostics::{OrphanedAttachment, DuplicateNoteGroup};

#[tauri::command]
pub async fn rebuild_database_cache(state: State<'_, AppState>) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    // Scan the vault files asynchronously first WITHOUT locking the database.
    // This resolves the Send-bound thread-safety issue because the MutexGuard is not held across an await.
    let notes = noda_core::vault::scan::scan_vault(&vault_path)
        .await
        .map_err(AppError::from)?;
    
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "DATABASE_NOT_INITIALIZED".to_string(),
            message: "Database cache is not initialized".to_string(),
        })?
    };
    
    let mut conn = db.conn.lock();
    noda_core::database::rebuild_database_sync(&notes, &mut conn)
        .map_err(AppError::from)?;
        
    Ok(())
}

#[tauri::command]
pub async fn vacuum_database_cache(state: State<'_, AppState>) -> Result<(), AppError> {
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "DATABASE_NOT_INITIALIZED".to_string(),
            message: "Database cache is not initialized".to_string(),
        })?
    };
    
    let conn = db.conn.lock();
    noda_core::diagnostics::vacuum_database(&conn).map_err(AppError::from)?;
    Ok(())
}

#[tauri::command]
pub async fn get_orphaned_attachments(
    state: State<'_, AppState>,
) -> Result<Vec<OrphanedAttachment>, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    noda_core::diagnostics::get_orphaned_attachments(&vault_path)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn delete_orphaned_attachments(
    state: State<'_, AppState>,
    filenames: Vec<String>,
) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    noda_core::diagnostics::delete_orphaned_attachments(&vault_path, filenames)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn clear_sync_queue(state: State<'_, AppState>) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    noda_core::diagnostics::clear_sync_queue(&vault_path)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn clear_sync_cache(state: State<'_, AppState>) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    noda_core::diagnostics::clear_sync_cache(&vault_path)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn get_duplicate_notes(
    state: State<'_, AppState>,
) -> Result<Vec<DuplicateNoteGroup>, AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    noda_core::diagnostics::get_duplicate_notes(&vault_path)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn delete_duplicate_note_file(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<(), AppError> {
    let vault_path = {
        let guard = state.vault_path.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };
    
    // 1. Physically delete file
    noda_core::diagnostics::delete_duplicate_note_file(&vault_path, &relative_path)
        .await
        .map_err(AppError::from)?;
        
    // 2. Clean from SQLite cache database
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "DATABASE_NOT_INITIALIZED".to_string(),
            message: "Database cache is not initialized".to_string(),
        })?
    };
    
    let conn = db.conn.lock();
    noda_core::database::queries::delete_note_by_path(&conn, &relative_path)
        .map_err(AppError::from)?;
        
    Ok(())
}
