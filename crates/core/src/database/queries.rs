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
    let tags_json: String = row.get("tags")?;
    let created_str: String = row.get("created")?;
    let updated_str: String = row.get("updated")?;
    
    let created_at = DateTime::parse_from_rfc3339(&created_str)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
        
    let updated_at = DateTime::parse_from_rfc3339(&updated_str)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
        
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

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
        status: row.get("status")?,
        created_at,
        updated_at,
    })
}

fn row_to_note_meta(row: &Row) -> Result<NoteMeta, rusqlite::Error> {
    let id_str: String = row.get("id")?;
    let parent_id_str: Option<String> = row.get("parent_id")?;
    let tags_json: String = row.get("tags")?;
    let updated_str: String = row.get("updated")?;
    
    let updated_at = DateTime::parse_from_rfc3339(&updated_str)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
        
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

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
        status: row.get("status")?,
        updated_at,
    })
}

pub fn get_note(conn: &Connection, id: NoteId) -> Result<Option<Note>, NodaError> {
    let mut stmt = conn.prepare("SELECT id, parent_id, title, body, color, pinned, tags, status, created, updated FROM notes WHERE id = ?1")
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
    
    Ok(())
}

pub fn delete_note(conn: &Connection, id: NoteId) -> Result<(), NodaError> {
    conn.execute("DELETE FROM notes WHERE id = ?1", params![id.0.to_string()])
        .map_err(|e| NodaError::Database(format!("Failed to delete note: {}", e)))?;
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

pub fn delete_note_by_path(conn: &Connection, file_path: &str) -> Result<(), NodaError> {
    conn.execute("DELETE FROM notes WHERE file_path = ?1", params![file_path])
        .map_err(|e| NodaError::Database(format!("Failed to delete note by path: {}", e)))?;
    Ok(())
}

pub fn list_notes(conn: &Connection) -> Result<Vec<NoteMeta>, NodaError> {
    let mut stmt = conn.prepare("SELECT id, parent_id, title, color, pinned, tags, status, updated FROM notes ORDER BY pinned DESC, updated DESC")
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
