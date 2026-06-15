use tauri::State;
use shared::AppError;
use shared::dtos::TrustedDeviceDto;
use crate::state::AppState;

#[tauri::command]
pub async fn get_trusted_devices(
    state: State<'_, AppState>,
) -> Result<Vec<TrustedDeviceDto>, AppError> {
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "DATABASE_NOT_INITIALIZED".to_string(),
            message: "No active database connection".to_string(),
        })?
    };

    let conn = db.conn.lock();
    let mut stmt = conn.prepare(
        "SELECT id, device_name, ip_address, status, device_token, created_at FROM trusted_devices ORDER BY created_at DESC"
    ).map_err(|e| AppError {
        code: "SQL_ERROR".to_string(),
        message: e.to_string(),
    })?;

    let rows = stmt.query_map([], |row| {
        Ok(TrustedDeviceDto {
            id: row.get(0)?,
            device_name: row.get(1)?,
            ip_address: row.get(2)?,
            status: row.get(3)?,
            device_token: row.get(4)?,
            created_at: row.get(5)?,
        })
    }).map_err(|e| AppError {
        code: "SQL_ERROR".to_string(),
        message: e.to_string(),
    })?;

    let mut list = Vec::new();
    for row in rows {
        if let Ok(item) = row {
            list.push(item);
        }
    }

    Ok(list)
}

#[tauri::command]
pub async fn approve_device(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "DATABASE_NOT_INITIALIZED".to_string(),
            message: "No active database connection".to_string(),
        })?
    };

    let device_token = uuid::Uuid::new_v4().simple().to_string(); // 32-char cryptographically secure token

    let conn = db.conn.lock();
    conn.execute(
        "UPDATE trusted_devices SET status = 'approved', device_token = ?1 WHERE id = ?2",
        [&device_token, &id],
    ).map_err(|e| AppError {
        code: "SQL_ERROR".to_string(),
        message: e.to_string(),
    })?;

    Ok(())
}

#[tauri::command]
pub async fn revoke_device(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "DATABASE_NOT_INITIALIZED".to_string(),
            message: "No active database connection".to_string(),
        })?
    };

    let conn = db.conn.lock();
    // Neutralize access immediately by setting status to revoked
    conn.execute(
        "UPDATE trusted_devices SET status = 'revoked', device_token = NULL WHERE id = ?1",
        [&id],
    ).map_err(|e| AppError {
        code: "SQL_ERROR".to_string(),
        message: e.to_string(),
    })?;

    Ok(())
}

#[tauri::command]
pub async fn regenerate_daemon_token(
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let new_token = uuid::Uuid::new_v4().simple().to_string();
    noda_core::vault::persistence::save_daemon_token_sync(&new_token)
        .map_err(|e| AppError {
            code: "PERSIST_FAIL".to_string(),
            message: e.to_string(),
        })?;
    
    *state.daemon_token.write() = new_token.clone();
    Ok(new_token)
}
