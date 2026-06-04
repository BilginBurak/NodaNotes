//! Database queries for Note CRUD operations

use crate::errors::NodaError;
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
    })
}

fn row_to_note_meta(row: &Row) -> Result<NoteMeta, rusqlite::Error> {
    let id_str: String = row.get("id")?;
    let parent_id_str: Option<String> = row.get("parent_id")?;
    let yaml_tags_json: String = row.get("yaml_tags")?;
    let inline_tags_json: String = row.get("inline_tags")?;
    let updated_str: String = row.get("updated")?;
    let file_path: String = row.get("file_path")?;
    
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
            file_path 
        FROM notes 
        WHERE id = ?1
    "#)
        .map_err(|e| NodaError::Database(format!("Prepare get_note failed: {}", e)))?;
    
    let note = stmt.query_row(params![id.0.to_string()], row_to_note)
        .optional()
        .map_err(|e| NodaError::Database(format!("Query get_note failed: {}", e)))?;
        
    Ok(note)
}

pub fn insert_note(conn: &Connection, note: &Note, file_path: &str, file_hash: &str) -> Result<(), NodaError> {
    let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".to_string());
    let parent_id_str = note.parent_id.map(|id| id.0.to_string());
    
    let created_str = note.created_at.to_rfc3339();
    let updated_str = note.updated_at.to_rfc3339();

    conn.execute(
        "INSERT INTO notes (id, parent_id, title, body, color, pinned, tags, status, created, updated, file_path, file_hash) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            note.id.0.to_string(),
            parent_id_str,
            note.title,
            note.body,
            note.color,
            note.pinned,
            tags_json,
            note.status,
            created_str,
            updated_str,
            file_path,
            file_hash
        ],
    ).map_err(|e| NodaError::Database(format!("Failed to insert note: {}", e)))?;
    
    sync_note_tags(conn, note)?;
    
    Ok(())
}

pub fn update_note(conn: &Connection, note: &Note, file_path: &str, file_hash: &str) -> Result<(), NodaError> {
    let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".to_string());
    let parent_id_str = note.parent_id.map(|id| id.0.to_string());
    
    let created_str = note.created_at.to_rfc3339();
    let updated_str = note.updated_at.to_rfc3339();

    conn.execute(
        "UPDATE notes SET 
            parent_id = ?2, title = ?3, body = ?4, color = ?5, pinned = ?6, tags = ?7, status = ?8, created = ?9, updated = ?10, file_path = ?11, file_hash = ?12
         WHERE id = ?1",
        params![
            note.id.0.to_string(),
            parent_id_str,
            note.title,
            note.body,
            note.color,
            note.pinned,
            tags_json,
            note.status,
            created_str,
            updated_str,
            file_path,
            file_hash
        ],
    ).map_err(|e| NodaError::Database(format!("Failed to update note: {}", e)))?;
    
    sync_note_tags(conn, note)?;
    
    Ok(())
}

pub fn upsert_note(conn: &Connection, note: &Note, file_path: &str, file_hash: &str) -> Result<(), NodaError> {
    let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".to_string());
    let parent_id_str = note.parent_id.map(|id| id.0.to_string());
    
    let created_str = note.created_at.to_rfc3339();
    let updated_str = note.updated_at.to_rfc3339();

    conn.execute(
        "INSERT INTO notes (id, parent_id, title, body, color, pinned, tags, status, created, updated, file_path, file_hash) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
            parent_id = excluded.parent_id,
            title = excluded.title,
            body = excluded.body,
            color = excluded.color,
            pinned = excluded.pinned,
            tags = excluded.tags,
            status = excluded.status,
            created = excluded.created,
            updated = excluded.updated,
            file_path = excluded.file_path,
            file_hash = excluded.file_hash",
        params![
            note.id.0.to_string(),
            parent_id_str,
            note.title,
            note.body,
            note.color,
            note.pinned,
            tags_json,
            note.status,
            created_str,
            updated_str,
            file_path,
            file_hash
        ],
    ).map_err(|e| NodaError::Database(format!("Failed to upsert note: {}", e)))?;
    
    sync_note_tags(conn, note)?;
    
    Ok(())
}

pub fn delete_note(conn: &Connection, id: NoteId) -> Result<(), NodaError> {
    conn.execute("DELETE FROM notes WHERE id = ?1", params![id.0.to_string()])
        .map_err(|e| NodaError::Database(format!("Failed to delete note: {}", e)))?;

    // Clean up orphaned tags
    conn.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM note_tags)",
        [],
    ).ok();

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
            file_path 
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

pub fn delete_note_by_path(conn: &Connection, file_path: &str) -> Result<(), NodaError> {
    conn.execute("DELETE FROM notes WHERE file_path = ?1", params![file_path])
        .map_err(|e| NodaError::Database(format!("Failed to delete note by path: {}", e)))?;

    // Clean up orphaned tags
    conn.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM note_tags)",
        [],
    ).ok();

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
        insert_note(&conn, &note, "test.md", "hash123").unwrap();

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
        update_note(&conn, &note, "test.md", "hash456").unwrap();
        
        let fetched2 = get_note(&conn, note.id).unwrap().unwrap();
        assert_eq!(fetched2.title, "Updated Note");

        // Delete
        delete_note(&conn, note.id).unwrap();
        let list2 = list_notes(&conn).unwrap();
        assert_eq!(list2.len(), 0);
    }
}
