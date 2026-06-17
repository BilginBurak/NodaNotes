use crate::errors::NodaError;
use crate::models::note::{Note, NoteId};
use crate::vault::service::VaultService;
use crate::database::connection::Database;
use rusqlite::OptionalExtension;
use chrono::Utc;
use std::path::Path;

#[derive(serde::Deserialize)]
pub struct ClipperPayload {
    pub title: String,
    pub url: String,
    pub content_markdown: Option<String>,
    pub tags: Vec<String>,
    pub append: Option<bool>,
    pub author: Option<String>,
    pub published_date: Option<String>,
}

pub fn sanitize_title(title: &str) -> String {
    let mut sanitized = String::new();
    for c in title.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            sanitized.push(c);
        } else if c.is_whitespace() {
            sanitized.push('_');
        }
    }
    let mut collapsed = String::new();
    let mut last_was_underscore = false;
    for c in sanitized.chars() {
        if c == '_' {
            if !last_was_underscore {
                collapsed.push(c);
                last_was_underscore = true;
            }
        } else {
            collapsed.push(c);
            last_was_underscore = false;
        }
    }
    let trimmed = collapsed.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "Untitled_Clip".to_string()
    } else {
        trimmed
    }
}


pub async fn check_url_history(db: &Database, url: &str) -> Result<Option<String>, NodaError> {
    let conn = db.conn.lock();
    let query = "SELECT id FROM notes WHERE body LIKE ?1 AND status = 'active' LIMIT 1";
    let like_pattern = format!("%{}%", url);
    
    let existing_id: Option<String> = conn.query_row(
        query,
        rusqlite::params![like_pattern],
        |row| row.get(0)
    ).optional().map_err(|e| NodaError::Database(e.to_string()))?;
    
    Ok(existing_id)
}

pub async fn clip_url(
    service: &VaultService,
    db: &Database,
    base_path: &Path,
    payload: ClipperPayload,
) -> Result<bool, NodaError> {
    let content_markdown = match &payload.content_markdown {
        Some(md) if !md.trim().is_empty() => md.clone(),
        _ => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default();
            let html = client.get(&payload.url).send().await
                .map_err(|e| NodaError::Vault(format!("Failed to fetch webpage content: {}", e)))?
                .text().await
                .map_err(|e| NodaError::Vault(format!("Failed to read webpage content: {}", e)))?;
            
            if html.trim().is_empty() {
                return Err(NodaError::Vault("Fetched HTML content is empty".to_string()));
            }
            
            let _ = url::Url::parse(&payload.url)
                .map_err(|e| NodaError::Vault(format!("Invalid URL: {}", e)))?;
            
            let readability = readabilityrs::Readability::new(&html, Some(payload.url.as_str()), None)
                .map_err(|e| NodaError::Vault(format!("Readability initialization failed: {}", e)))?;
            
            let article = readability.parse()
                .ok_or_else(|| NodaError::Vault("Readability parsing failed".to_string()))?;
            
            html2md::parse_html(&article.content.unwrap_or_default())
        }
    };

    let existing_note_info = {
        let conn = db.conn.lock();
        conn.query_row(
            "SELECT id, file_path FROM notes WHERE body LIKE ?1 AND status = 'active' LIMIT 1",
            rusqlite::params![format!("%{}%", payload.url)],
            |row| {
                let id_str: String = row.get(0)?;
                let file_path: String = row.get(1)?;
                let id = ulid::Ulid::from_string(&id_str)
                    .map(NoteId)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
                Ok((id, file_path))
            }
        ).optional().unwrap_or(None)
    };

    let now = Utc::now();
    let file_path: String;
    let final_note: Note;

    if payload.append.unwrap_or(false) && existing_note_info.is_some() {
        let (_, relative_path) = existing_note_info.unwrap();
        let abs_path = base_path.join(&relative_path);
        let file_content = std::fs::read_to_string(&abs_path)
            .map_err(NodaError::Io)?;

        let boundary = "<!-- noda-webclipper -->";
        let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
        let appended_text = format!(
            "*Appended via Android on {}:*\n{}",
            date_str,
            content_markdown.trim()
        );

        let new_content = if let Some(pos) = file_content.find(boundary) {
            let (before, after) = file_content.split_at(pos);
            format!("{}\n\n{}\n\n{}", before.trim(), appended_text, after.trim())
        } else {
            format!("{}\n\n{}\n\n{}", file_content.trim(), appended_text, boundary)
        };

        std::fs::write(&abs_path, &new_content).map_err(NodaError::Io)?;

        let mut note = VaultService::read_note_from_absolute_path(&abs_path, &relative_path).await?;
        note.updated_at = now;
        service.write_note(&note).await?;

        file_path = relative_path;
        final_note = note;
    } else {
        let id = NoteId::new();
        let sanitized = sanitize_title(&payload.title);
        let relative_path = format!("clipper/{}.md", sanitized);
        let abs_path = base_path.join(&relative_path);

        let final_relative_path = if abs_path.exists() {
            format!("clipper/{}_{}.md", sanitized, id.0.to_string())
        } else {
            relative_path
        };

        let author = payload.author.filter(|a| !a.is_empty()).unwrap_or_else(|| "Unknown".to_string());
        let published = payload.published_date.filter(|p| !p.is_empty()).unwrap_or_else(|| "Unknown".to_string());
        let added_date = chrono::Local::now().format("%Y-%m-%d").to_string();

        let source_link = if payload.url.is_empty() {
            "Unknown".to_string()
        } else {
            format!("[Link]({})", payload.url)
        };

        let compiled_hashtags = if payload.tags.is_empty() {
            "".to_string()
        } else {
            payload.tags.iter()
                .map(|t| {
                    let trimmed = t.trim();
                    if trimmed.starts_with('#') {
                        trimmed.to_string()
                    } else {
                        format!("#{}", trimmed)
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        };

        let footer = format!(
            "<!-- noda-webclipper -->\n—\n*Added via NodaNotes #webclipper*\n\n**Source:** {}\n**Author:** {}\n**Published:** {}\n**Tags:** {}\n**Added:** {}",
            source_link,
            author,
            published,
            compiled_hashtags,
            added_date
        );

        let body = format!("{}\n\n{}", content_markdown, footer);
        let inline_tags = Note::parse_inline_tags(&body);

        let note = Note {
            id,
            parent_id: None,
            title: payload.title,
            body,
            color: None,
            pinned: false,
            tags: payload.tags,
            inline_tags,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
            file_path: final_relative_path.clone(),
        };

        service.write_note(&note).await?;

        file_path = final_relative_path;
        final_note = note;
    }

    let final_abs_path = base_path.join(&file_path);
    let size = std::fs::metadata(&final_abs_path)
        .map_err(NodaError::Io)?
        .len();

    let markdown = final_note.to_markdown()
        .map_err(|e| NodaError::Vault(format!("Failed to serialize note markdown: {}", e)))?;
    let raw_hash = xxhash_rust::xxh3::xxh3_64(markdown.as_bytes());
    let hash_hex = format!("{:016x}", raw_hash);

    {
        let conn = db.conn.lock();
        crate::database::queries::upsert_note(&conn, &final_note, &file_path, false)
            .map_err(|e| NodaError::Database(e.to_string()))?;

        conn.execute(
            "INSERT INTO sync_file_states (path, etag, last_modified, size, local_updated_at, hash, is_dirty, retry_count, sync_error) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 0, NULL) \
             ON CONFLICT(path) DO UPDATE SET \
                etag = excluded.etag, \
                last_modified = excluded.last_modified, \
                size = excluded.size, \
                local_updated_at = excluded.local_updated_at, \
                hash = excluded.hash, \
                is_dirty = 1, \
                retry_count = 0, \
                sync_error = NULL",
            rusqlite::params![
                file_path,
                None::<String>,
                None::<String>,
                size as i64,
                now.to_rfc3339(),
                hash_hex,
            ],
        ).map_err(|e| NodaError::Database(e.to_string()))?;
    }

    Ok(true)
}
