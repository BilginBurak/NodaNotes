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
            created_at: note.created_at.to_rfc3339(),
            updated_at: note.updated_at.to_rfc3339(),
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
            updated_at: meta.updated_at.to_rfc3339(),
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
