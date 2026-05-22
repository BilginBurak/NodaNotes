//! Trash storage operations

use crate::errors::NodaError;
use crate::models::note::NoteId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrashEntry {
    pub note_id: NoteId,
    #[serde(default)]
    pub id: Option<NoteId>,
    pub original_path: String,
    pub deleted_at: DateTime<Utc>,
    pub filename: String,
    #[serde(default)]
    pub title: String,
}

pub async fn move_to_trash<P: AsRef<Path>, P2: AsRef<Path>>(
    vault_path: P,
    note_path: P2,
) -> Result<TrashEntry, NodaError> {
    let vault_root = vault_path.as_ref();
    let relative_note_path = note_path.as_ref(); 
    
    let absolute_note_path = if relative_note_path.is_absolute() {
        relative_note_path.to_path_buf()
    } else {
        vault_root.join(relative_note_path)
    };

    if !absolute_note_path.exists() {
        return Err(NodaError::NotFound("Note to delete not found".into()));
    }

    let content = tokio::fs::read_to_string(&absolute_note_path)
        .await
        .map_err(NodaError::Io)?;
        
    use gray_matter::Matter;
    use gray_matter::engine::YAML;
    use crate::models::note::Frontmatter;
    
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);
    
    let frontmatter: Frontmatter = parsed
        .data
        .as_ref()
        .ok_or_else(|| NodaError::Frontmatter("No frontmatter found in file".to_string()))?
        .deserialize()
        .map_err(|e| NodaError::Frontmatter(format!("Failed to parse YAML: {}", e)))?;
        
    let note_id = frontmatter.id;
    
    let trash_dir = vault_root.join(".noda").join("trash");
    tokio::fs::create_dir_all(&trash_dir).await.map_err(NodaError::Io)?;
    
    let md_filename = format!("{}.md", note_id.0.to_string());
    let json_filename = format!("{}.json", note_id.0.to_string());
    let trash_md_path = trash_dir.join(&md_filename);
    let trash_json_path = trash_dir.join(&json_filename);

    let original_path_str = if relative_note_path.is_absolute() {
        relative_note_path.strip_prefix(vault_root).unwrap_or(relative_note_path).to_string_lossy().to_string()
    } else {
        relative_note_path.to_string_lossy().to_string()
    };

    let entry = TrashEntry {
        note_id,
        id: Some(note_id),
        original_path: original_path_str,
        deleted_at: Utc::now(),
        filename: md_filename,
        title: frontmatter.title.clone(),
    };

    // Write sidecar
    let json_content = serde_json::to_string_pretty(&entry).unwrap();
    tokio::fs::write(&trash_json_path, json_content).await.map_err(NodaError::Io)?;
    
    // Move the markdown file
    tokio::fs::rename(&absolute_note_path, &trash_md_path).await.map_err(NodaError::Io)?;

    info!("Moved {} to trash", absolute_note_path.display());

    Ok(entry)
}

pub async fn restore_from_trash<P: AsRef<Path>>(
    vault_path: P,
    trash_entry: &TrashEntry,
) -> Result<(), NodaError> {
    let vault_root = vault_path.as_ref();
    let trash_dir = vault_root.join(".noda").join("trash");
    
    let trash_md_path = trash_dir.join(&trash_entry.filename);
    let trash_json_path = trash_dir.join(format!("{}.json", trash_entry.note_id.0.to_string()));
    
    if !trash_md_path.exists() {
        return Err(NodaError::NotFound("Trashed note missing".into()));
    }
    
    let target_path = vault_root.join(&trash_entry.original_path);
    
    if let Some(parent) = target_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(NodaError::Io)?;
    }
    
    if target_path.exists() {
        return Err(NodaError::Io(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Target file already exists")));
    }
    
    tokio::fs::rename(&trash_md_path, &target_path).await.map_err(NodaError::Io)?;
    
    let _ = tokio::fs::remove_file(&trash_json_path).await;
    
    Ok(())
}

pub async fn permanent_delete<P: AsRef<Path>>(
    vault_path: P,
    trash_entry: &TrashEntry,
) -> Result<(), NodaError> {
    let vault_root = vault_path.as_ref();
    let trash_dir = vault_root.join(".noda").join("trash");
    
    let trash_md_path = trash_dir.join(&trash_entry.filename);
    let trash_json_path = trash_dir.join(format!("{}.json", trash_entry.note_id.0.to_string()));
    
    let _ = tokio::fs::remove_file(&trash_md_path).await;
    let _ = tokio::fs::remove_file(&trash_json_path).await;
    
    Ok(())
}

pub async fn list_trash<P: AsRef<Path>>(vault_path: P) -> Result<Vec<TrashEntry>, NodaError> {
    let vault_root = vault_path.as_ref();
    let trash_dir = vault_root.join(".noda").join("trash");
    
    if !trash_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(&trash_dir).await.map_err(NodaError::Io)?;
    
    while let Some(file) = dir.next_entry().await.map_err(NodaError::Io)? {
        let path = file.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
            let content = tokio::fs::read_to_string(&path).await.map_err(NodaError::Io)?;
            if let Ok(mut entry) = serde_json::from_str::<TrashEntry>(&content) {
                if entry.id.is_none() {
                    entry.id = Some(entry.note_id);
                }
                if entry.title.is_empty() {
                    let md_path = trash_dir.join(&entry.filename);
                    if md_path.exists() {
                        if let Ok(md_content) = tokio::fs::read_to_string(&md_path).await {
                            use gray_matter::Matter;
                            use gray_matter::engine::YAML;
                            use crate::models::note::Frontmatter;
                            let matter = Matter::<YAML>::new();
                            let parsed = matter.parse(&md_content);
                            if let Some(fm_any) = parsed.data {
                                if let Ok(frontmatter) = fm_any.deserialize::<Frontmatter>() {
                                    entry.title = frontmatter.title;
                                }
                            }
                        }
                    }
                    if entry.title.is_empty() {
                        entry.title = "Untitled".to_string();
                    }
                }
                entries.push(entry);
            }
        }
    }
    
    entries.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
    Ok(entries)
}
