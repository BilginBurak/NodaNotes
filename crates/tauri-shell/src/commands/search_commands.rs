use tauri::State;
use shared::AppError;
use crate::state::AppState;
use noda_core::database::search::search_notes as core_search_notes;
use noda_core::models::note::SearchResult;

#[tauri::command]
pub async fn search_notes(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<SearchResult>, AppError> {
    let db_lock = state.database.read();
    let db = db_lock.as_ref().ok_or_else(|| AppError {
        code: "VAULT_NOT_OPEN".to_string(),
        message: "No active vault is currently open".to_string(),
    })?;

    let conn = db.conn.lock();
    let results = core_search_notes(&conn, &query)
        .map_err(AppError::from)?;

    Ok(results)
}
