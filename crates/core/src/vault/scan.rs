//! Vault scanning utilities to discover and parse notes

use crate::errors::NodaError;
use crate::models::note::{Frontmatter, Note};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use std::path::Path;
use tokio::fs;
use tracing::{info, warn};
use walkdir::{DirEntry, WalkDir};

fn is_hidden_or_noda(entry: &DirEntry) -> bool {
    // Always scan the root directory itself, even if it happens to be hidden
    // (e.g. system temp directories like .tmpXYZ created by tempfile)
    if entry.depth() == 0 {
        return false;
    }

    // Skip all hidden files and folders, particularly ".noda",
    // but allow the ".templates" directory.
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.') && s != ".templates")
        .unwrap_or(false)
}

/// Helper function to parse a file or convert it to a standard NodaNote (ULID filename + frontmatter).
pub async fn parse_or_create_note_from_file(file_path: &Path, vault_root: &Path) -> Result<Note, NodaError> {
    use crate::models::note::NoteId;
    use chrono::{DateTime, Utc};

    let content = fs::read_to_string(file_path).await.map_err(NodaError::Io)?;
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);

    let filename_stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let is_filename_valid_ulid = filename_stem.len() == 26 && ulid::Ulid::from_string(filename_stem).is_ok();

    // Check if there is valid frontmatter
    let frontmatter_opt = if let Some(data) = &parsed.data {
        data.deserialize::<Frontmatter>().ok()
    } else {
        None
    };

    if is_filename_valid_ulid {
        if let Some(fm) = frontmatter_opt.clone() {
            // Perfect case: filename is valid ULID, and frontmatter matches or has valid ULID
            let expected_id = NoteId(ulid::Ulid::from_string(filename_stem).unwrap());
            if fm.id == expected_id {
                let rel_path = file_path.strip_prefix(vault_root)
                    .unwrap_or(file_path)
                    .to_string_lossy()
                    .to_string();
                return Ok(Note {
                    id: fm.id,
                    parent_id: fm.parent_id,
                    title: fm.title,
                    inline_tags: Note::parse_inline_tags(&parsed.content),
                    body: parsed.content,
                    color: fm.color,
                    pinned: fm.pinned,
                    tags: fm.tags,
                    status: fm.status,
                    created_at: fm.created_at,
                    updated_at: fm.updated_at,
                    file_path: rel_path,
                });
            }
        }
    }

    // If we reach here, we need to convert/rewrite the file!
    // 1. Get/generate the note ID
    let note_id = if let Some(fm) = &frontmatter_opt {
        fm.id
    } else if is_filename_valid_ulid {
        NoteId(ulid::Ulid::from_string(filename_stem).unwrap())
    } else {
        NoteId::new()
    };

    // 2. Determine title
    let title = if let Some(fm) = &frontmatter_opt {
        if fm.title.is_empty() {
            filename_stem.to_string()
        } else {
            fm.title.clone()
        }
    } else {
        filename_stem.to_string()
    };

    // 3. Determine times (from frontmatter or file metadata)
    let meta = fs::metadata(file_path).await.ok();
    let file_created = meta.as_ref().and_then(|m| m.created().ok())
        .map(|t| DateTime::<Utc>::from(t))
        .unwrap_or_else(Utc::now);
    let file_modified = meta.as_ref().and_then(|m| m.modified().ok())
        .map(|t| DateTime::<Utc>::from(t))
        .unwrap_or_else(Utc::now);

    let created_at = frontmatter_opt.as_ref().map(|fm| fm.created_at).unwrap_or(file_created);
    let updated_at = frontmatter_opt.as_ref().map(|fm| fm.updated_at).unwrap_or(file_modified);

    // 5. Serialize and write the converted note
    let parent_dir = file_path.parent().ok_or_else(|| {
        NodaError::Vault("Invalid file path: no parent directory".to_string())
    })?;
    let new_filename = format!("{}.md", note_id.0.to_string());
    let new_path = parent_dir.join(&new_filename);

    let new_rel_path = new_path.strip_prefix(vault_root)
        .unwrap_or(&new_path)
        .to_string_lossy()
        .to_string();

    // 4. Build Note object
    let note = Note {
        id: note_id,
        parent_id: frontmatter_opt.as_ref().and_then(|fm| fm.parent_id),
        title,
        inline_tags: Note::parse_inline_tags(&parsed.content),
        body: parsed.content,
        color: frontmatter_opt.as_ref().and_then(|fm| fm.color.clone()),
        pinned: frontmatter_opt.as_ref().map(|fm| fm.pinned).unwrap_or(false),
        tags: frontmatter_opt.as_ref().map(|fm| fm.tags.clone()).unwrap_or_default(),
        status: frontmatter_opt.as_ref().map(|fm| fm.status.clone()).unwrap_or_else(|| "active".to_string()),
        created_at,
        updated_at,
        file_path: new_rel_path,
    };

    let frontmatter: Frontmatter = (&note).into();
    let yaml_string = serde_yaml::to_string(&frontmatter)
        .map_err(|e| NodaError::Vault(format!("Failed to serialize frontmatter: {}", e)))?;
    let new_content = format!("---\n{}---\n{}", yaml_string, note.body);

    // Write to the new path
    fs::write(&new_path, new_content).await.map_err(NodaError::Io)?;

    // If the path has changed, remove the old file
    if new_path != file_path {
        if let Err(e) = fs::remove_file(file_path).await {
            warn!("Failed to delete old file {:?}: {}", file_path, e);
        }
    }

    Ok(note)
}

/// Recursively scans the vault path for .md files and parses them into Note structs.
/// Corrupt or invalid files are logged and skipped.
pub async fn scan_vault<P: AsRef<Path>>(vault_path: P) -> Result<Vec<Note>, NodaError> {
    let path = vault_path.as_ref();
    if !path.exists() {
        return Err(NodaError::Vault("Vault path does not exist for scanning".to_string()));
    }

    let mut notes = Vec::new();

    // WalkDir is synchronous, but file reading can be asynchronous.
    // For scanning hundreds of files, we collect paths synchronously first.
    let mut md_files = Vec::new();
    
    for entry in WalkDir::new(path).into_iter().filter_entry(|e| !is_hidden_or_noda(e)) {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                warn!("Error reading directory entry: {}", err);
                continue;
            }
        };

        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ext == "md" || ext == "markdown" {
                    md_files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    info!("Found {} markdown files during vault scan.", md_files.len());

    // Process each markdown file
    for file_path in md_files {
        match parse_or_create_note_from_file(&file_path, path).await {
            Ok(note) => {
                notes.push(note);
            }
            Err(e) => {
                warn!("Failed to parse or convert file {:?}: {}", file_path, e);
            }
        }
    }

    Ok(notes)
}

/// Helper function to parse raw markdown content, handle frontmatter, and create a Note struct.
pub async fn parse_or_create_note_from_content(
    title: &str,
    content: &str,
    relative_path: &str,
) -> Result<Note, NodaError> {
    use crate::models::note::NoteId;
    use chrono::Utc;

    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(content);

    // Check if there is valid frontmatter
    let frontmatter_opt = if let Some(data) = &parsed.data {
        data.deserialize::<Frontmatter>().ok()
    } else {
        None
    };

    let note_id = NoteId::new(); // Always generate a new ULID
    let final_title = if let Some(fm) = &frontmatter_opt {
        if fm.title.is_empty() {
            title.to_string()
        } else {
            fm.title.clone()
        }
    } else {
        title.to_string()
    };

    let now = Utc::now();
    let created_at = frontmatter_opt.as_ref().map(|fm| fm.created_at).unwrap_or(now);
    let updated_at = frontmatter_opt.as_ref().map(|fm| fm.updated_at).unwrap_or(now);

    let filename = format!("{}.md", note_id.0.to_string());
    let new_rel_path = if relative_path.is_empty() {
        filename
    } else {
        format!("{}/{}", relative_path.trim_end_matches('/'), filename)
    };

    let note = Note {
        id: note_id,
        parent_id: None,
        title: final_title,
        inline_tags: Note::parse_inline_tags(&parsed.content),
        body: parsed.content,
        color: frontmatter_opt.as_ref().and_then(|fm| fm.color.clone()),
        pinned: frontmatter_opt.as_ref().map(|fm| fm.pinned).unwrap_or(false),
        tags: frontmatter_opt.as_ref().map(|fm| fm.tags.clone()).unwrap_or_default(),
        status: frontmatter_opt.as_ref().map(|fm| fm.status.clone()).unwrap_or_else(|| "active".to_string()),
        created_at,
        updated_at,
        file_path: new_rel_path,
    };

    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::service::VaultService;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_scan_vault() {
        let dir = tempdir().unwrap();
        let service = VaultService::new(dir.path()).unwrap();

        // Write a valid note
        let note1 = Note::new();
        service.write_note(&note1).await.unwrap();

        // Write a second valid note
        let note2 = Note::new();
        service.write_note(&note2).await.unwrap();

        // Create an invalid markdown file manually
        let invalid_path = dir.path().join("invalid.md");
        fs::write(&invalid_path, "No frontmatter here!").await.unwrap();

        // Create a file in a hidden directory (should be ignored)
        let hidden_dir = dir.path().join(".noda");
        fs::create_dir_all(&hidden_dir).await.unwrap();
        fs::write(hidden_dir.join("should_be_ignored.md"), "---\nid: 123\n---\nIgnored").await.unwrap();

        let scanned_notes = scan_vault(dir.path()).await.expect("Failed to scan vault");
        
        // It should pick up note1, note2, and the imported/converted invalid.md file.
        assert_eq!(scanned_notes.len(), 3);
        assert!(!dir.path().join("invalid.md").exists());
        
        // Ensure both notes are present in the results
        let contains_note1 = scanned_notes.iter().any(|n| n.id == note1.id);
        let contains_note2 = scanned_notes.iter().any(|n| n.id == note2.id);
        let contains_invalid_imported = scanned_notes.iter().any(|n| n.title == "invalid" && n.body == "No frontmatter here!");
        
        assert!(contains_note1);
        assert!(contains_note2);
        assert!(contains_invalid_imported);
    }
}
