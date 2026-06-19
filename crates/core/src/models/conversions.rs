//! Conversions between Domain Models and Shared DTOs

use crate::models::note::{Note, NoteMeta};
use crate::models::vault::Vault;
use shared::dtos::{NoteDto, NoteListItemDto, VaultInfoDto};

impl From<Note> for NoteDto {
    fn from(note: Note) -> Self {
        Self {
            id: note.id.0.to_string(),
            parent_id: note.parent_id.map(|id| id.0.to_string()),
            title: note.title,
            body: note.body,
            color: note.color,
            pinned: note.pinned,
            tags: note.tags,
            inline_tags: note.inline_tags,
            created_at: note.created_at.to_rfc3339(),
            updated_at: note.updated_at.to_rfc3339(),
            file_path: note.file_path,
            is_encrypted: note.is_encrypted,
        }
    }
}

impl From<NoteMeta> for NoteListItemDto {
    fn from(meta: NoteMeta) -> Self {
        Self {
            id: meta.id.0.to_string(),
            parent_id: meta.parent_id.map(|id| id.0.to_string()),
            title: meta.title,
            color: meta.color,
            pinned: meta.pinned,
            tags: meta.tags,
            inline_tags: meta.inline_tags,
            updated_at: meta.updated_at.to_rfc3339(),
            file_path: meta.file_path,
            is_encrypted: meta.is_encrypted,
        }
    }
}

impl From<Vault> for VaultInfoDto {
    fn from(vault: Vault) -> Self {
        Self {
            name: vault.name,
            path: vault.path.to_string_lossy().into_owned(),
        }
    }
}
