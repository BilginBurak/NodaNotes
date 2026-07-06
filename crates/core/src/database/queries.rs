//! Database queries for Note CRUD operations

use crate::errors::NodaError;
use std::path::Path;
use tracing::info;
use crate::models::note::{Note, NoteId, NoteMeta};
use rusqlite::{params, Connection, OptionalExtension, Row};
use ulid::Ulid;
use chrono::{DateTime, Utc};

fn parse_ulid(s: &str) -> Result<NoteId, rusqlite::Error> {
    Ulid::from_string(s)
        .map(NoteId)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))
}

fn row_to_note(row: &Row) -> Result<Note, rusqlite::Error> {
    let id_str: String = row.get("id")?;
    let parent_id_str: Option<String> = row.get("parent_id")?;
    let yaml_tags_json: String = row.get("yaml_tags")?;
    let inline_tags_json: String = row.get("inline_tags")?;
    let created_str: String = row.get("created")?;
    let updated_str: String = row.get("updated")?;
    let file_path: String = row.get("file_path")?;
    let is_encrypted: bool = row.get("is_encrypted")?;
    let dek_encrypted: Option<String> = row.get("dek_encrypted")?;
    let dek_nonce: Option<String> = row.get("dek_nonce")?;
    let outline: Option<String> = row.get("outline").ok();
    
    let created_at = DateTime::parse_from_rfc3339(&created_str)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
        
    let updated_at = DateTime::parse_from_rfc3339(&updated_str)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
        
    let tags: Vec<String> = serde_json::from_str(&yaml_tags_json).unwrap_or_default();
    let inline_tags: Vec<String> = serde_json::from_str(&inline_tags_json).unwrap_or_default();

    Ok(Note {
        id: parse_ulid(&id_str)?,
        parent_id: match parent_id_str {
            Some(s) => Some(parse_ulid(&s)?),
            None => None,
        },
        title: row.get("title")?,
        body: row.get("body")?,
        color: row.get("color")?,
        pinned: row.get("pinned")?,
        tags,
        inline_tags,
        status: row.get("status")?,
        created_at,
        updated_at,
        file_path,
        is_encrypted,
        dek_encrypted,
        dek_nonce,
        outline,
    })
}

fn row_to_note_meta(row: &Row) -> Result<NoteMeta, rusqlite::Error> {
    let id_str: String = row.get("id")?;
    let parent_id_str: Option<String> = row.get("parent_id")?;
    let yaml_tags_json: String = row.get("yaml_tags")?;
    let inline_tags_json: String = row.get("inline_tags")?;
    let updated_str: String = row.get("updated")?;
    let file_path: String = row.get("file_path")?;
    let is_encrypted: bool = row.get("is_encrypted")?;
    
    let updated_at = DateTime::parse_from_rfc3339(&updated_str)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
        
    let tags: Vec<String> = serde_json::from_str(&yaml_tags_json).unwrap_or_default();
    let inline_tags: Vec<String> = serde_json::from_str(&inline_tags_json).unwrap_or_default();

    Ok(NoteMeta {
        id: parse_ulid(&id_str)?,
        parent_id: match parent_id_str {
            Some(s) => Some(parse_ulid(&s)?),
            None => None,
        },
        title: row.get("title")?,
        color: row.get("color")?,
        pinned: row.get("pinned")?,
        tags,
        inline_tags,
        status: row.get("status")?,
        updated_at,
        file_path,
        is_encrypted,
    })
}

pub fn get_note(conn: &Connection, id: NoteId) -> Result<Option<Note>, NodaError> {
    let mut stmt = conn.prepare(r#"
        SELECT 
            id, 
            parent_id, 
            title, 
            body, 
            color, 
            pinned, 
            (
                SELECT COALESCE(json_group_array(t.name), '[]')
                FROM note_tags nt
                JOIN tags t ON nt.tag_id = t.id
                WHERE nt.note_id = notes.id AND nt.source = 'yaml'
            ) as yaml_tags, 
            (
                SELECT COALESCE(json_group_array(t.name), '[]')
                FROM note_tags nt
                JOIN tags t ON nt.tag_id = t.id
                WHERE nt.note_id = notes.id AND nt.source = 'inline'
            ) as inline_tags, 
            status, 
            created, 
            updated, 
            file_path,
            is_encrypted,
            dek_encrypted,
            dek_nonce,
            outline
        FROM notes 
        WHERE id = ?1
    "#)
        .map_err(|e| NodaError::Database(format!("Prepare get_note failed: {}", e)))?;
    
    let note = stmt.query_row(params![id.0.to_string()], row_to_note)
        .optional()
        .map_err(|e| NodaError::Database(format!("Query get_note failed: {}", e)))?;
        
    Ok(note)
}

pub fn insert_note(conn: &Connection, note: &Note, file_path: &str) -> Result<(), NodaError> {
    let parent_id_str = note.parent_id.map(|id| id.0.to_string());
    
    let created_str = note.created_at.to_rfc3339();
    let updated_str = note.updated_at.to_rfc3339();
    let outline_str = Note::parse_outline(&note.body);

    conn.execute(
        "INSERT INTO notes (id, parent_id, title, body, color, pinned, status, created, updated, file_path, is_encrypted, dek_encrypted, dek_nonce, outline) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            note.id.0.to_string(),
            parent_id_str,
            note.title,
            note.body,
            note.color,
            note.pinned,
            note.status,
            created_str,
            updated_str,
            file_path,
            note.is_encrypted,
            note.dek_encrypted,
            note.dek_nonce,
            outline_str
        ],
    ).map_err(|e| NodaError::Database(format!("Failed to insert note: {}", e)))?;
    
    sync_note_tags(conn, note)?;
    
    Ok(())
}

pub fn update_note(conn: &Connection, note: &Note, file_path: &str) -> Result<(), NodaError> {
    let parent_id_str = note.parent_id.map(|id| id.0.to_string());
    
    let created_str = note.created_at.to_rfc3339();
    let updated_str = note.updated_at.to_rfc3339();
    let outline_str = Note::parse_outline(&note.body);

    conn.execute(
        "UPDATE notes SET 
            parent_id = ?2, title = ?3, body = ?4, color = ?5, pinned = ?6, status = ?7, created = ?8, updated = ?9, file_path = ?10,
            is_encrypted = ?11, dek_encrypted = ?12, dek_nonce = ?13, outline = ?14
         WHERE id = ?1",
        params![
            note.id.0.to_string(),
            parent_id_str,
            note.title,
            note.body,
            note.color,
            note.pinned,
            note.status,
            created_str,
            updated_str,
            file_path,
            note.is_encrypted,
            note.dek_encrypted,
            note.dek_nonce,
            outline_str
        ],
    ).map_err(|e| NodaError::Database(format!("Failed to update note: {}", e)))?;
    
    sync_note_tags(conn, note)?;
    
    Ok(())
}

pub fn upsert_note(conn: &Connection, note: &Note, file_path: &str, mark_dirty: bool) -> Result<(), NodaError> {
    let parent_id_str = note.parent_id.map(|id| id.0.to_string());
    
    let created_str = note.created_at.to_rfc3339();
    let updated_str = note.updated_at.to_rfc3339();
    let outline_str = Note::parse_outline(&note.body);

    conn.execute(
        "INSERT INTO notes (id, parent_id, title, body, color, pinned, status, created, updated, file_path, is_encrypted, dek_encrypted, dek_nonce, outline) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(id) DO UPDATE SET
            parent_id = excluded.parent_id,
            title = excluded.title,
            body = excluded.body,
            color = excluded.color,
            pinned = excluded.pinned,
            status = excluded.status,
            created = excluded.created,
            updated = excluded.updated,
            file_path = excluded.file_path,
            is_encrypted = excluded.is_encrypted,
            dek_encrypted = excluded.dek_encrypted,
            dek_nonce = excluded.dek_nonce,
            outline = excluded.outline",
        params![
            note.id.0.to_string(),
            parent_id_str,
            note.title,
            note.body,
            note.color,
            note.pinned,
            note.status,
            created_str,
            updated_str,
            file_path,
            note.is_encrypted,
            note.dek_encrypted,
            note.dek_nonce,
            outline_str
        ],
    ).map_err(|e| NodaError::Database(format!("Failed to upsert note: {}", e)))?;
    
    sync_note_tags(conn, note)?;
    
    if mark_dirty {
        set_file_dirty(conn, file_path, true)?;
    }
    
    Ok(())
}

pub fn delete_note(conn: &Connection, id: NoteId, mark_dirty: bool) -> Result<(), NodaError> {
    let file_path: Option<String> = conn.query_row(
        "SELECT file_path FROM notes WHERE id = ?1",
        params![id.0.to_string()],
        |row| row.get(0)
    ).optional().map_err(|e| NodaError::Database(format!("Failed to retrieve file_path for deletion: {}", e)))?;

    conn.execute("DELETE FROM notes WHERE id = ?1", params![id.0.to_string()])
        .map_err(|e| NodaError::Database(format!("Failed to delete note: {}", e)))?;

    // Clean up orphaned tags
    conn.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM note_tags)",
        [],
    ).ok();

    if mark_dirty {
        if let Some(path) = file_path {
            set_file_dirty(conn, &path, true)?;
        }
    }

    Ok(())
}

pub fn get_note_id_by_path(conn: &Connection, file_path: &str) -> Result<Option<NoteId>, NodaError> {
    let mut stmt = conn.prepare("SELECT id FROM notes WHERE file_path = ?1")
        .map_err(|e| NodaError::Database(format!("Prepare get_note_id_by_path failed: {}", e)))?;
        
    let id_str: Option<String> = stmt.query_row(params![file_path], |row| row.get(0))
        .optional()
        .map_err(|e| NodaError::Database(format!("Query get_note_id_by_path failed: {}", e)))?;
        
    match id_str {
        Some(s) => Ok(Some(parse_ulid(&s).map_err(|e| NodaError::Database(e.to_string()))?)),
        None => Ok(None),
    }
}


pub fn list_notes(conn: &Connection) -> Result<Vec<NoteMeta>, NodaError> {
    let mut stmt = conn.prepare(r#"
        SELECT 
            id, 
            parent_id, 
            title, 
            color, 
            pinned, 
            (
                SELECT COALESCE(json_group_array(t.name), '[]')
                FROM note_tags nt
                JOIN tags t ON nt.tag_id = t.id
                WHERE nt.note_id = notes.id AND nt.source = 'yaml'
            ) as yaml_tags, 
            (
                SELECT COALESCE(json_group_array(t.name), '[]')
                FROM note_tags nt
                JOIN tags t ON nt.tag_id = t.id
                WHERE nt.note_id = notes.id AND nt.source = 'inline'
            ) as inline_tags, 
            status, 
            updated, 
            file_path,
            is_encrypted
        FROM notes 
        ORDER BY pinned DESC, updated DESC
    "#)
        .map_err(|e| NodaError::Database(format!("Prepare list_notes failed: {}", e)))?;
        
    let rows = stmt.query_map([], row_to_note_meta)
        .map_err(|e| NodaError::Database(format!("Query map list_notes failed: {}", e)))?;
        
    let mut notes = Vec::new();
    for row in rows {
        match row {
            Ok(n) => notes.push(n),
            Err(e) => return Err(NodaError::Database(format!("Row parsing failed: {}", e))),
        }
    }
    
    Ok(notes)
}

pub fn delete_note_by_path(conn: &Connection, file_path: &str, mark_dirty: bool) -> Result<(), NodaError> {
    conn.execute("DELETE FROM notes WHERE file_path = ?1", params![file_path])
        .map_err(|e| NodaError::Database(format!("Failed to delete note by path: {}", e)))?;

    // Clean up orphaned tags
    conn.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM note_tags)",
        [],
    ).ok();

    if mark_dirty {
        set_file_dirty(conn, file_path, true)?;
    }

    Ok(())
}

pub fn purge_note_fts_body(conn: &Connection, id: NoteId) -> Result<(), NodaError> {
    conn.execute(
        "UPDATE notes_fts SET body = '' WHERE rowid = (SELECT rowid FROM notes WHERE id = ?1)",
        params![id.0.to_string()],
    ).map_err(|e| NodaError::Database(format!("Failed to purge FTS5 body: {}", e)))?;
    Ok(())
}

pub fn update_note_file_path(conn: &Connection, id: NoteId, file_path: &str) -> Result<(), NodaError> {
    conn.execute(
        "UPDATE notes SET file_path = ?2 WHERE id = ?1",
        params![id.0.to_string(), file_path],
    ).map_err(|e| NodaError::Database(format!("Failed to update note file_path: {}", e)))?;
    Ok(())
}

pub fn sync_note_tags(conn: &Connection, note: &Note) -> Result<(), NodaError> {
    let note_id = note.id.0.to_string();
    
    // 1. Delete existing note_tags relationships for this note only.
    // Do NOT delete the tags themselves, as we will clean up orphaned tags at the end.
    conn.execute("DELETE FROM note_tags WHERE note_id = ?1", params![note_id])
        .map_err(|e| NodaError::Database(format!("Failed to clear note tags: {}", e)))?;
        
    // 2. Parse inline tags using unified robust logic
    let inline_tags = Note::parse_inline_tags(&note.body);
    
    // 3. Clean YAML tags
    let mut yaml_tags = Vec::new();
    for t in &note.tags {
        let cleaned = t.trim().trim_start_matches('#').to_string();
        if !cleaned.is_empty() {
            yaml_tags.push(cleaned);
        }
    }
    
    // 4. Insert tags into `tags` table using ON CONFLICT DO NOTHING, and link in `note_tags`
    for tag_name in &yaml_tags {
        // Insert into tags if not exists
        conn.execute(
            "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            params![tag_name],
        ).map_err(|e| NodaError::Database(format!("Failed to insert tag: {}", e)))?;
        
        // Get tag id
        let tag_id: i64 = conn.query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![tag_name],
            |row| row.get(0),
        ).map_err(|e| NodaError::Database(format!("Failed to get tag id: {}", e)))?;
        
        // Insert link
        conn.execute(
            "INSERT OR REPLACE INTO note_tags (note_id, tag_id, source) VALUES (?1, ?2, ?3)",
            params![note_id, tag_id, "yaml"],
        ).map_err(|e| NodaError::Database(format!("Failed to link note tag (yaml): {}", e)))?;
    }
    
    for tag_name in &inline_tags {
        // Skip if already inserted as yaml to respect source prioritization
        if yaml_tags.contains(tag_name) {
            continue;
        }
        
        // Insert into tags if not exists
        conn.execute(
            "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            params![tag_name],
        ).map_err(|e| NodaError::Database(format!("Failed to insert tag: {}", e)))?;
        
        // Get tag id
        let tag_id: i64 = conn.query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![tag_name],
            |row| row.get(0),
        ).map_err(|e| NodaError::Database(format!("Failed to get tag id: {}", e)))?;
        
        // Insert link
        conn.execute(
            "INSERT OR REPLACE INTO note_tags (note_id, tag_id, source) VALUES (?1, ?2, ?3)",
            params![note_id, tag_id, "inline"],
        ).map_err(|e| NodaError::Database(format!("Failed to link note tag (inline): {}", e)))?;
    }
    
    // 5. Clean up any tags that have 0 notes associated with them
    conn.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM note_tags)",
        [],
    ).map_err(|e| NodaError::Database(format!("Failed to clean orphaned tags: {}", e)))?;
    
    Ok(())
}

pub fn list_tags_with_counts(conn: &Connection) -> Result<Vec<(String, i32)>, NodaError> {
    let mut stmt = conn.prepare(
        "SELECT tags.name, COUNT(note_tags.note_id) as note_count
         FROM tags
         JOIN note_tags ON tags.id = note_tags.tag_id
         GROUP BY tags.id
         ORDER BY tags.name ASC"
    ).map_err(|e| NodaError::Database(format!("Prepare list_tags_with_counts failed: {}", e)))?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
    }).map_err(|e| NodaError::Database(format!("Query map list_tags_with_counts failed: {}", e)))?;

    let mut tags = Vec::new();
    for row in rows {
        match row {
            Ok(t) => tags.push(t),
            Err(e) => return Err(NodaError::Database(format!("Row parsing failed: {}", e))),
        }
    }
    Ok(tags)
}

pub fn find_daily_note_id(conn: &Connection, date_str: &str) -> Result<Option<NoteId>, NodaError> {
    let mut stmt = conn.prepare(
        "SELECT id FROM notes WHERE title = ?1 AND file_path LIKE 'Daily Notes/%' LIMIT 1"
    ).map_err(|e| NodaError::Database(format!("Prepare find_daily_note_id failed: {}", e)))?;
    
    let mut rows = stmt.query_map(params![date_str], |row| {
        let id_str: String = row.get(0)?;
        let note_id = parse_ulid(&id_str).map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
        Ok(note_id)
    }).map_err(|e| NodaError::Database(format!("Query map find_daily_note_id failed: {}", e)))?;

    if let Some(row) = rows.next() {
        let id = row.map_err(|e| NodaError::Database(format!("Row parsing failed in find_daily_note_id: {}", e)))?;
        Ok(Some(id))
    } else {
        Ok(None)
    }
}

pub fn load_remote_state(conn: &Connection) -> Result<crate::sync::remote_state::RemoteState, NodaError> {
    use crate::sync::remote_state::{RemoteState, RemoteFileMetadata, DeviceMetadata};

    let mut state = RemoteState::default();

    // 1. Load last_sync_time and devices
    let mut stmt = conn.prepare("SELECT device_name, last_known_etag, last_known_modified FROM sync_device_states")
        .map_err(|e| NodaError::Database(format!("Prepare load_remote_state devices failed: {}", e)))?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    }).map_err(|e| NodaError::Database(format!("Query load_remote_state devices failed: {}", e)))?;

    for row in rows {
        let (device_name, etag, modified_str) = row.map_err(|e| NodaError::Database(format!("Row parsing failed in load_remote_state: {}", e)))?;
        
        let modified = modified_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));

        if device_name == "__last_sync_time__" {
            state.last_sync_time = modified;
        } else {
            state.devices.insert(device_name, DeviceMetadata {
                last_known_etag: etag,
                last_known_modified: modified,
            });
        }
    }

    // 2. Load files
    let mut stmt = conn.prepare("SELECT path, etag, last_modified, size, local_updated_at, hash, is_dirty FROM sync_file_states")
        .map_err(|e| NodaError::Database(format!("Prepare load_remote_state files failed: {}", e)))?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, i32>(6)?,
        ))
    }).map_err(|e| NodaError::Database(format!("Query load_remote_state files failed: {}", e)))?;

    for row in rows {
        let (path, etag, last_mod_str, size, local_up_str, hash, is_dirty_int) = row.map_err(|e| NodaError::Database(format!("Row parsing failed in load_remote_state files: {}", e)))?;

        let last_modified = last_mod_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
        let local_updated_at = local_up_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));

        state.files.insert(path, RemoteFileMetadata {
            etag,
            last_modified,
            size: size as u64,
            local_updated_at,
            hash,
            is_dirty: is_dirty_int != 0,
        });
    }

    Ok(state)
}

pub fn save_remote_state(conn: &Connection, state: &crate::sync::remote_state::RemoteState) -> Result<(), NodaError> {
    // Delete existing
    conn.execute("DELETE FROM sync_device_states", [])
        .map_err(|e| NodaError::Database(format!("Failed to delete sync_device_states: {}", e)))?;
    conn.execute("DELETE FROM sync_file_states", [])
        .map_err(|e| NodaError::Database(format!("Failed to delete sync_file_states: {}", e)))?;

    // Insert last_sync_time
    if let Some(ref lst) = state.last_sync_time {
        conn.execute(
            "INSERT INTO sync_device_states (device_name, last_known_etag, last_known_modified) VALUES (?1, ?2, ?3)",
            params![
                "__last_sync_time__",
                None::<String>,
                lst.to_rfc3339(),
            ],
        ).map_err(|e| NodaError::Database(format!("Failed to insert last_sync_time: {}", e)))?;
    }

    // Insert devices
    let mut stmt = conn.prepare("INSERT INTO sync_device_states (device_name, last_known_etag, last_known_modified) VALUES (?1, ?2, ?3)")
        .map_err(|e| NodaError::Database(format!("Prepare insert device failed: {}", e)))?;
    for (name, meta) in &state.devices {
        let lm_str = meta.last_known_modified.map(|dt| dt.to_rfc3339());
        stmt.execute(params![
            name,
            meta.last_known_etag,
            lm_str,
        ]).map_err(|e| NodaError::Database(format!("Failed to insert device state for {}: {}", name, e)))?;
    }
    drop(stmt);

    // Insert files
    let mut stmt = conn.prepare("INSERT INTO sync_file_states (path, etag, last_modified, size, local_updated_at, hash, is_dirty) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
        .map_err(|e| NodaError::Database(format!("Prepare insert file state failed: {}", e)))?;
    let vault_path = get_vault_path_from_conn(conn);
    for (path, meta) in &state.files {
        let lm_str = meta.last_modified.map(|dt| dt.to_rfc3339());
        let lu_str = meta.local_updated_at.map(|dt| dt.to_rfc3339());
        let is_dirty_int = if meta.is_dirty { 1 } else { 0 };
        
        let mut actual_size = meta.size;
        if let Some(ref vp) = vault_path {
            let full_path = vp.join(path);
            if let Ok(fs_meta) = std::fs::metadata(&full_path) {
                actual_size = fs_meta.len();
            }
        }

        stmt.execute(params![
            path,
            meta.etag,
            lm_str,
            actual_size as i64,
            lu_str,
            meta.hash,
            is_dirty_int,
        ]).map_err(|e| NodaError::Database(format!("Failed to insert file state for {}: {}", path, e)))?;
    }
    drop(stmt);

    Ok(())
}

pub fn get_vault_path_from_conn(conn: &Connection) -> Option<std::path::PathBuf> {
    if let Some(db_path_str) = conn.path() {
        let db_path = std::path::Path::new(db_path_str);
        if let Some(noda_dir) = db_path.parent() {
            if let Some(vault_path) = noda_dir.parent() {
                return Some(vault_path.to_path_buf());
            }
        }
    }
    None
}

pub fn set_file_dirty(conn: &Connection, path: &str, is_dirty: bool) -> Result<(), NodaError> {
    let mut size = 0u64;
    if let Some(vault_path) = get_vault_path_from_conn(conn) {
        let full_path = vault_path.join(path);
        if let Ok(meta) = std::fs::metadata(&full_path) {
            size = meta.len();
        }
    }
    conn.execute(
        "INSERT INTO sync_file_states (path, size, hash, is_dirty, retry_count, sync_error) \
         VALUES (?1, ?2, '', ?3, 0, NULL) \
         ON CONFLICT(path) DO UPDATE SET is_dirty = excluded.is_dirty, size = excluded.size, retry_count = 0, sync_error = NULL",
        params![path, size as i64, if is_dirty { 1 } else { 0 }],
    ).map_err(|e| NodaError::Database(format!("Failed to set_file_dirty for {}: {}", path, e)))?;
    Ok(())
}

pub fn check_file_mismatch(conn: &Connection, relative_path: &str, file_size: u64, file_mtime: DateTime<Utc>) -> Result<bool, NodaError> {
    let mut stmt = conn.prepare("SELECT size, local_updated_at FROM sync_file_states WHERE path = ?1")
        .map_err(|e| NodaError::Database(format!("Prepare check_file_mismatch failed: {}", e)))?;
    let res = stmt.query_row(params![relative_path], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
        ))
    }).optional().map_err(|e| NodaError::Database(format!("Query check_file_mismatch failed: {}", e)))?;

    if let Some((stored_size, stored_mtime_str)) = res {
        if file_size != stored_size as u64 {
            return Ok(true);
        }
        if let Some(stored_mtime_str) = stored_mtime_str {
            if let Ok(stored_mtime) = DateTime::parse_from_rfc3339(&stored_mtime_str) {
                let diff = (file_mtime - stored_mtime.with_timezone(&Utc)).num_seconds().abs();
                if diff > 1 {
                    return Ok(true);
                }
            } else {
                return Ok(true);
            }
        } else {
            return Ok(true);
        }
        Ok(false)
    } else {
        Ok(true)
    }
}

pub fn run_cold_boot_scan(conn: &Connection, vault_path: &Path) -> Result<(), NodaError> {
    info!("Running Cold Boot Light Scan on vault: {:?}", vault_path);
    let mut scanned_paths = std::collections::HashSet::new();

    let mut check_and_update_file = |rel_path: String, full_path: &Path| -> Result<(), NodaError> {
        scanned_paths.insert(rel_path.clone());
        if let Ok(meta) = std::fs::metadata(full_path) {
            let size = meta.len();
            let mtime = meta.modified()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                .unwrap_or_else(|_| chrono::Utc::now());
            
            let is_mismatch = check_file_mismatch(conn, &rel_path, size, mtime)?;
            if is_mismatch {
                tracing::info!("Cold Boot Scan: Mismatch detected for {}, syncing with DB", rel_path);
                if (rel_path.ends_with(".md") || rel_path.ends_with(".markdown")) && !rel_path.starts_with(".noda/") {
                    match crate::vault::scan::parse_or_create_note_from_file_sync(full_path, vault_path) {
                        Ok(note) => {
                            upsert_note(conn, &note, &note.file_path, true)?;
                        }
                        Err(e) => {
                            tracing::error!("Cold Boot Scan: Failed to parse mismatch file {}: {}", rel_path, e);
                            set_file_dirty(conn, &rel_path, true)?;
                        }
                    }
                } else {
                    set_file_dirty(conn, &rel_path, true)?;
                }
            }
        }
        Ok(())
    };

    // 1. Scan notes recursively
    let mut stack = vec![vault_path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let ftype = match entry.file_type() {
                    Ok(t) => t,
                    _ => continue,
                };
                let path = entry.path();
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                
                if name_str.starts_with('.') && name_str != ".templates" {
                    continue;
                }
                
                if ftype.is_dir() {
                    stack.push(path);
                } else if ftype.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "md" || ext == "markdown" {
                            if let Ok(rel) = path.strip_prefix(vault_path) {
                                let rel_str = rel.to_string_lossy().to_string();
                                check_and_update_file(rel_str, &path)?;
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Scan attachments
    let attachments_dir = vault_path.join(".noda").join("attachments");
    if attachments_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&attachments_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.') {
                        continue;
                    }
                    let path = entry.path();
                    if let Ok(rel) = path.strip_prefix(vault_path) {
                        let rel_str = rel.to_string_lossy().to_string();
                        check_and_update_file(rel_str, &path)?;
                    }
                }
            }
        }
    }

    // 3. Scan history
    let history_dir = vault_path.join(".noda").join("history");
    if history_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&history_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.') {
                        continue;
                    }
                    let path = entry.path();
                    if let Ok(rel) = path.strip_prefix(vault_path) {
                        let rel_str = rel.to_string_lossy().to_string();
                        check_and_update_file(rel_str, &path)?;
                    }
                }
            }
        }
    }

    // 4. Identify files in state that no longer exist (deleted locally)
    let mut stmt = conn.prepare("SELECT path FROM sync_file_states")
        .map_err(|e| NodaError::Database(format!("Prepare check deleted files failed: {}", e)))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| NodaError::Database(format!("Query check deleted files failed: {}", e)))?;

    for path_res in rows {
        if let Ok(path) = path_res {
            let path_lower = path.to_lowercase();
            if path_lower.ends_with(".ds_store") || path_lower.contains("/.") {
                let _ = conn.execute("DELETE FROM sync_file_states WHERE path = ?1", [&path]);
                continue;
            }
            if path.starts_with(".noda/sync/") {
                continue;
            }
            if path == ".noda/vault_config.json" {
                if vault_path.join(&path).exists() {
                    continue;
                }
            }
            if !scanned_paths.contains(&path) {
                tracing::info!("Cold Boot Scan: File {} no longer exists on disk, deleting from notes and marking dirty", path);
                if path.ends_with(".md") || path.ends_with(".markdown") {
                    let _ = delete_note_by_path(conn, &path, true);
                } else {
                    set_file_dirty(conn, &path, true)?;
                }
            }
        }
    }

    Ok(())
}

pub fn list_full_notes(conn: &Connection) -> Result<Vec<Note>, NodaError> {
    let mut stmt = conn.prepare(r#"
        SELECT 
            id, 
            parent_id, 
            title, 
            body, 
            color, 
            pinned, 
            (
                SELECT COALESCE(json_group_array(t.name), '[]')
                FROM note_tags nt
                JOIN tags t ON nt.tag_id = t.id
                WHERE nt.note_id = notes.id AND nt.source = 'yaml'
            ) as yaml_tags, 
            (
                SELECT COALESCE(json_group_array(t.name), '[]')
                FROM note_tags nt
                JOIN tags t ON nt.tag_id = t.id
                WHERE nt.note_id = notes.id AND nt.source = 'inline'
            ) as inline_tags, 
            status, 
            created, 
            updated, 
            file_path,
            is_encrypted,
            dek_encrypted,
            dek_nonce,
            outline
        FROM notes
    "#).map_err(|e| NodaError::Database(format!("Prepare list_full_notes failed: {}", e)))?;

    let rows = stmt.query_map([], row_to_note)
        .map_err(|e| NodaError::Database(format!("Query map list_full_notes failed: {}", e)))?;

    let mut notes = Vec::new();
    for row in rows {
        match row {
            Ok(n) => notes.push(n),
            Err(e) => return Err(NodaError::Database(format!("Row parsing failed in list_full_notes: {}", e))),
        }
    }
    Ok(notes)
}

pub fn clear_sync_tables(conn: &Connection) -> Result<(), NodaError> {
    conn.execute("DELETE FROM sync_device_states", [])
        .map_err(|e| NodaError::Database(format!("Failed to clear sync_device_states: {}", e)))?;
    conn.execute("DELETE FROM sync_file_states", [])
        .map_err(|e| NodaError::Database(format!("Failed to clear sync_file_states: {}", e)))?;
    Ok(())
}

pub fn list_active_graph_nodes(conn: &Connection) -> Result<Vec<crate::models::note::GraphNodeDb>, NodaError> {
    let mut stmt = conn.prepare(r#"
        SELECT 
            note_id,
            relative_path,
            CASE 
                WHEN LENGTH(title) > 15 THEN SUBSTR(title, 1, 12) || '...'
                ELSE title 
            END AS title,
            last_modified,
            char_size,
            outline,
            is_encrypted
        FROM mcp_vault_view
    "#).map_err(|e| NodaError::Database(format!("Prepare list_active_graph_nodes failed: {}", e)))?;

    let rows = stmt.query_map([], |row| {
        let last_modified_str: Option<String> = row.get("last_modified")?;
        let last_modified = last_modified_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.timestamp())
                .ok()
        });
        Ok(crate::models::note::GraphNodeDb {
            note_id: row.get("note_id")?,
            relative_path: row.get("relative_path")?,
            title: row.get("title")?,
            last_modified,
            char_size: row.get::<_, Option<u64>>("char_size")?.unwrap_or(0),
            outline: row.get("outline")?,
            is_encrypted: row.get("is_encrypted")?,
        })
    }).map_err(|e| NodaError::Database(format!("Query map list_active_graph_nodes failed: {}", e)))?;

    let mut nodes = Vec::new();
    for r in rows {
        nodes.push(r.map_err(|e| NodaError::Database(format!("Row parsing failed: {}", e)))?);
    }
    Ok(nodes)
}

#[cfg(test)]


mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::tempdir;

    #[test]
    fn test_crud_queries() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("test.db")).unwrap();
        let conn = db.conn.lock();

        let mut note = Note::new();
        note.title = "Test Note".to_string();
        note.tags = vec!["rust".to_string(), "noda".to_string()];
        note.pinned = true;

        // Insert
        insert_note(&conn, &note, "test.md").unwrap();

        // Get
        let fetched = get_note(&conn, note.id).unwrap().unwrap();
        assert_eq!(fetched.title, "Test Note");
        assert_eq!(fetched.tags.len(), 2);
        assert_eq!(fetched.pinned, true);

        // List
        let list = list_notes(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Test Note");

        // Update
        note.title = "Updated Note".to_string();
        update_note(&conn, &note, "test.md").unwrap();
        
        let fetched2 = get_note(&conn, note.id).unwrap().unwrap();
        assert_eq!(fetched2.title, "Updated Note");

        // Delete
        delete_note(&conn, note.id, false).unwrap();
        let list2 = list_notes(&conn).unwrap();
        assert_eq!(list2.len(), 0);
    }
}
