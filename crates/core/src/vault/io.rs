//! Atomic note file I/O with YAML frontmatter serialization.
//!
//! All write operations use the `write_tmp → rename` pattern to prevent
//! partial writes from corrupting notes.

use crate::errors::NodaError;
use crate::models::{Frontmatter, Note};
use chrono::Utc;
use gray_matter::{engine::YAML, Matter};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::instrument;
use uuid::Uuid;

// ─── Frontmatter serialization ───────────────────────────────────────────────

/// Render a `Frontmatter` struct into its YAML-fenced string representation.
fn render_frontmatter(fm: &Frontmatter) -> String {
    let tags_yaml = fm
        .tags
        .iter()
        .map(|t| format!("  - {}", t))
        .collect::<Vec<_>>()
        .join("\n");

    let tags_section = if fm.tags.is_empty() {
        "tags: []".to_string()
    } else {
        format!("tags:\n{}", tags_yaml)
    };

    format!(
        "---\nid: \"{}\"\ntitle: \"{}\"\ncreated: \"{}\"\nupdated: \"{}\"\n{}\nstatus: \"{}\"\n---\n",
        fm.id,
        fm.title,
        fm.created.to_rfc3339(),
        fm.updated.to_rfc3339(),
        tags_section,
        fm.status,
    )
}

/// Render the full note file content (frontmatter + body).
fn render_note_file(fm: &Frontmatter, body: &str) -> String {
    format!("{}\n{}", render_frontmatter(fm), body)
}

// ─── Parsing ─────────────────────────────────────────────────────────────────

/// Parse a raw file string into a `Frontmatter` and body.
fn parse_note_content(
    raw: &str,
    file_display: &str,
) -> Result<(Frontmatter, String), NodaError> {
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(raw);

    let data: Value = parsed
        .data
        .ok_or_else(|| NodaError::FrontmatterParse {
            file: file_display.to_string(),
            reason: "no YAML frontmatter found".to_string(),
        })?
        .deserialize()
        .map_err(|e| NodaError::FrontmatterParse {
            file: file_display.to_string(),
            reason: e.to_string(),
        })?;

    let get_str = |key: &str| -> Result<String, NodaError> {
        data.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| NodaError::FrontmatterMissingField {
                field: key.to_string(),
                file: file_display.to_string(),
            })
    };

    let id = get_str("id")?;
    let title = get_str("title")?;

    let created_str = get_str("created")?;
    let created = chrono::DateTime::parse_from_rfc3339(&created_str)
        .map_err(|e| NodaError::FrontmatterParse {
            file: file_display.to_string(),
            reason: format!("invalid 'created' timestamp: {}", e),
        })?
        .with_timezone(&Utc);

    let updated_str = get_str("updated")?;
    let updated = chrono::DateTime::parse_from_rfc3339(&updated_str)
        .map_err(|e| NodaError::FrontmatterParse {
            file: file_display.to_string(),
            reason: format!("invalid 'updated' timestamp: {}", e),
        })?
        .with_timezone(&Utc);

    let tags = data
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let status = get_str("status").unwrap_or_else(|_| "active".to_string());

    let frontmatter = Frontmatter {
        id,
        title,
        created,
        updated,
        tags,
        status,
    };

    Ok((frontmatter, parsed.content))
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Read and parse a note from its absolute file path.
#[instrument(skip(file_path), fields(path = %file_path.as_ref().display()))]
pub async fn read_note(file_path: impl AsRef<Path>) -> Result<Note, NodaError> {
    let file_path = file_path.as_ref().to_path_buf();
    let raw = fs::read_to_string(&file_path).await?;
    let display = file_path.display().to_string();

    let (frontmatter, body) = parse_note_content(&raw, &display)?;

    // The relative path cannot be computed here without the vault root.
    // Callers that need it (e.g. scan) set it after the fact.
    // For direct reads we use the filename as a fallback.
    let relative_path = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    Ok(Note {
        frontmatter,
        body: body.trim_start_matches('\n').to_string(),
        file_path,
        relative_path,
    })
}

/// Read and parse a note, computing its relative path from the vault root.
pub async fn read_note_relative(
    vault_root: &Path,
    file_path: &Path,
) -> Result<Note, NodaError> {
    let mut note = read_note(file_path).await?;
    note.relative_path = file_path
        .strip_prefix(vault_root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| note.relative_path.clone());
    Ok(note)
}

/// Create and write a new note to disk.
///
/// Returns the fully constructed `Note` with generated UUID and timestamps.
#[instrument(skip(vault_root, body), fields(title = %title, vault = %vault_root.display()))]
pub async fn write_new_note(
    vault_root: &Path,
    title: String,
    tags: Vec<String>,
    body: String,
) -> Result<Note, NodaError> {
    let now = Utc::now();
    let id = Uuid::new_v4().to_string();

    let frontmatter = Frontmatter {
        id: id.clone(),
        title: title.clone(),
        created: now,
        updated: now,
        tags: tags.clone(),
        status: "active".to_string(),
    };

    let filename = format!("{}.md", title);
    let file_path = vault_root.join(&filename);

    atomic_write(&file_path, &render_note_file(&frontmatter, &body)).await?;

    let relative_path = filename;
    tracing::debug!(id = %id, path = %file_path.display(), "note created");

    Ok(Note {
        frontmatter,
        body,
        file_path,
        relative_path,
    })
}

/// Update a note's body, refreshing the `updated` timestamp.
#[instrument(skip(note, new_body), fields(id = %note.frontmatter.id))]
pub async fn update_note_body(note: &Note, new_body: String) -> Result<Note, NodaError> {
    let mut updated_fm = note.frontmatter.clone();
    updated_fm.updated = Utc::now();

    atomic_write(&note.file_path, &render_note_file(&updated_fm, &new_body)).await?;

    tracing::debug!(id = %note.frontmatter.id, "note body updated");

    Ok(Note {
        frontmatter: updated_fm,
        body: new_body,
        file_path: note.file_path.clone(),
        relative_path: note.relative_path.clone(),
    })
}

/// Atomically rename a note file and update its frontmatter title.
///
/// Steps:
/// 1. Write new content to `new_path` with updated title + timestamp.
/// 2. Remove the old file.
///
/// If step 2 fails the old file may remain; callers should handle this.
/// The new file is fully written before the old one is removed.
#[instrument(skip(old_path, new_path), fields(old = %old_path.display(), new = %new_path.display()))]
pub async fn rename_note(
    old_path: impl AsRef<Path>,
    new_path: impl Into<PathBuf>,
    new_title: String,
) -> Result<Note, NodaError> {
    let old_path = old_path.as_ref();
    let new_path = new_path.into();

    let raw = fs::read_to_string(old_path).await?;
    let display = old_path.display().to_string();
    let (mut frontmatter, body) = parse_note_content(&raw, &display)?;

    frontmatter.title = new_title;
    frontmatter.updated = Utc::now();

    atomic_write(&new_path, &render_note_file(&frontmatter, &body)).await?;

    // Remove old file only after new one is successfully written.
    fs::remove_file(old_path).await?;

    let relative_path = new_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    tracing::info!(
        id = %frontmatter.id,
        old = %old_path.display(),
        new = %new_path.display(),
        "note renamed"
    );

    Ok(Note {
        frontmatter,
        body,
        file_path: new_path,
        relative_path,
    })
}

/// Write `content` to `path` atomically using a sibling `.tmp` file.
pub async fn atomic_write(path: &Path, content: &str) -> Result<(), NodaError> {
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content.as_bytes()).await?;
    fs::rename(&tmp, path).await?;
    Ok(())
}
