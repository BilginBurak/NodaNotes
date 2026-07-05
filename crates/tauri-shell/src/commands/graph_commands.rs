use tauri::State;
use shared::AppError;
use shared::dtos::GraphNodeDto;
use crate::state::AppState;
use noda_core::models::note::{Note, NoteId};
use noda_core::database::queries;
use noda_core::trash::soft_delete as core_soft_delete;
use chrono::Utc;

#[tauri::command]
pub async fn get_graph_nodes(
    state: State<'_, AppState>,
) -> Result<GraphNodeDto, AppError> {
    let db = {
        let guard = state.database.read();
        guard.clone().ok_or_else(|| AppError {
            code: "VAULT_NOT_OPEN".to_string(),
            message: "No active vault is currently open".to_string(),
        })?
    };

    let nodes_db = {
        let conn = db.conn.lock();
        queries::list_active_graph_nodes(&conn).map_err(AppError::from)?
    };

    let mut root = GraphNodeDto {
        id: "root".to_string(),
        name: "/Vault".to_string(),
        file_path: "".to_string(),
        is_folder: true,
        is_encrypted: false,
        char_size: 0,
        last_modified: None,
        outline: None,
        children: Vec::new(),
    };

    for n in nodes_db {
        let parts: Vec<&str> = n.relative_path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }

        let mut current = &mut root;
        let mut current_path = String::new();

        for (i, part) in parts.iter().enumerate() {
            if !current_path.is_empty() {
                current_path.push('/');
            }
            current_path.push_str(part);

            let is_last = i == parts.len() - 1;
            if is_last {
                let leaf = GraphNodeDto {
                    id: n.note_id.clone(),
                    name: n.title.clone(),
                    file_path: n.relative_path.clone(),
                    is_folder: false,
                    is_encrypted: n.is_encrypted,
                    char_size: n.char_size,
                    last_modified: n.last_modified,
                    outline: n.outline.clone(),
                    children: Vec::new(),
                };
                current.children.push(leaf);
            } else {
                let part_str = part.to_string();
                let exists = current.children.iter().any(|c| c.is_folder && c.name == part_str);
                if !exists {
                    let folder = GraphNodeDto {
                        id: current_path.clone(),
                        name: part_str.clone(),
                        file_path: current_path.clone(),
                        is_folder: true,
                        is_encrypted: false,
                        char_size: 0,
                        last_modified: None,
                        outline: None,
                        children: Vec::new(),
                    };
                    current.children.push(folder);
                }
                current = current.children.iter_mut().find(|c| c.is_folder && c.name == part_str).unwrap();
            }
        }
    }

    Ok(root)
}

#[tauri::command]
pub async fn graft_node(
    state: State<'_, AppState>,
    old_path: String,
    new_path: String,
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
    let src = base_path.join(&old_path);
    let dest = base_path.join(&new_path);

    if src == dest {
        return Ok(());
    }

    if !src.exists() {
        return Err(AppError {
            code: "NOT_FOUND".to_string(),
            message: format!("Source path not found: {}", old_path),
        });
    }

    if dest.exists() {
        return Err(AppError {
            code: "ALREADY_EXISTS".to_string(),
            message: format!("Destination already exists: {}", new_path),
        });
    }

    // Ensure parent folder of destination exists
    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| AppError {
                code: "IO_ERROR".to_string(),
                message: format!("Failed to create parent folder: {}", e),
            })?;
        }
    }

    // Rename physically
    tokio::fs::rename(&src, &dest).await.map_err(|e| AppError {
        code: "IO_ERROR".to_string(),
        message: format!("Failed to rename on disk: {}", e),
    })?;

    // Update SQLite path mappings
    let conn = db.conn.lock();
    let is_note = old_path.ends_with(".md") || queries::get_note_id_by_path(&conn, &old_path).map_err(AppError::from)?.is_some();

    if is_note {
        if let Some(note_id) = queries::get_note_id_by_path(&conn, &old_path).map_err(AppError::from)? {
            queries::update_note_file_path(&conn, note_id, &new_path).map_err(AppError::from)?;
        }
    } else {
        // It's a folder, recursively update all sub-notes paths
        let all_notes = queries::list_notes(&conn).map_err(AppError::from)?;
        let src_prefix = format!("{}/", old_path);
        for note in all_notes {
            if note.file_path.starts_with(&src_prefix) {
                let suffix = &note.file_path[src_prefix.len()..];
                let updated = format!("{}/{}", new_path, suffix);
                queries::update_note_file_path(&conn, note.id, &updated).map_err(AppError::from)?;
            } else if note.file_path == old_path {
                queries::update_note_file_path(&conn, note.id, &new_path).map_err(AppError::from)?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn sprout_node(
    state: State<'_, AppState>,
    parent_path: String,
    name: String,
    node_type: String,
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
    let clean_parent = parent_path.trim_start_matches('/').trim_end_matches('/');
    
    if node_type == "folder" {
        let folder_rel = if clean_parent.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", clean_parent, name)
        };
        let abs_dir = base_path.join(&folder_rel);
        tokio::fs::create_dir_all(&abs_dir).await.map_err(|e| AppError {
            code: "IO_ERROR".to_string(),
            message: format!("Failed to create folder: {}", e),
        })?;
    } else {
        // Create note
        let id = NoteId::new();
        let file_name = format!("{}.md", id.0.to_string());
        let relative_path = if clean_parent.is_empty() {
            file_name
        } else {
            format!("{}/{}", clean_parent, file_name)
        };

        let now = Utc::now();
        let body = format!("# {}\n", name);
        let note = Note {
            id,
            parent_id: None,
            title: name,
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
            outline: Some(Note::parse_outline(&body)),
        };

        // Write note to disk
        service.write_note(&note).await.map_err(AppError::from)?;

        // Save in DB
        {
            let conn = db.conn.lock();
            queries::upsert_note(&conn, &note, &relative_path, true).map_err(AppError::from)?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn prune_node(
    state: State<'_, AppState>,
    target_path: String,
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

    let base_path = service.base_path();
    let full_target = base_path.join(&target_path);

    if !full_target.exists() {
        return Err(AppError {
            code: "NOT_FOUND".to_string(),
            message: format!("Target path does not exist: {}", target_path),
        });
    }

    let is_note = {
        let conn = db.conn.lock();
        target_path.ends_with(".md") || queries::get_note_id_by_path(&conn, &target_path).map_err(AppError::from)?.is_some()
    };

    if is_note {
        let note_id_opt = {
            let conn = db.conn.lock();
            queries::get_note_id_by_path(&conn, &target_path).map_err(AppError::from)?
        };
        if let Some(note_id) = note_id_opt {
            // Soft delete on disk (no lock held!)
            core_soft_delete(&vault_path, &target_path).await.map_err(AppError::from)?;
            // Delete in DB
            let conn = db.conn.lock();
            queries::delete_note(&conn, note_id, true).map_err(AppError::from)?;
        }
    } else {
        // Recursive folder delete
        let notes_to_delete = {
            let conn = db.conn.lock();
            let all_notes = queries::list_notes(&conn).map_err(AppError::from)?;
            let prefix = format!("{}/", target_path);
            
            all_notes
                .into_iter()
                .filter(|n| n.file_path.starts_with(&prefix) || n.file_path == target_path)
                .collect::<Vec<_>>()
        };

        for note in notes_to_delete {
            core_soft_delete(&vault_path, &note.file_path).await.map_err(AppError::from)?;
            let conn = db.conn.lock();
            queries::delete_note(&conn, note.id, true).map_err(AppError::from)?;
        }

        // Physically delete empty directories
        service.delete_folder(&target_path).await.map_err(|e| AppError {
            code: "IO_ERROR".to_string(),
            message: format!("Failed to delete folder on disk: {}", e),
        })?;
    }

    Ok(())
}
