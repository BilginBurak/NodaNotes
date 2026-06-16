//! Attachments module

pub mod storage;

pub use storage::{delete_attachment, list_attachments, store_attachment, store_attachment_bytes, AttachmentInfo, list_attachments_with_metadata};

use crate::errors::NodaError;
use std::path::{Path, PathBuf};

/// Resolves a noda:// URI to an absolute file path on disk
pub fn resolve_path<P: AsRef<Path>>(vault_path: P, uri: &str) -> Result<PathBuf, NodaError> {
    let target = "/attachments/";
    let file_name = if let Some(idx) = uri.find(target) {
        &uri[idx + target.len()..]
    } else {
        return Err(NodaError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Invalid attachment URI: {}", uri),
        )));
    };

    // Path traversal protection
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err(NodaError::PathTraversal("Path traversal detected in URI".into()));
    }

    let attachments_dir = vault_path.as_ref().join(".noda").join("attachments");
    Ok(attachments_dir.join(file_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_attachment_flow() {
        let vault_dir = tempdir().unwrap();
        let source_dir = tempdir().unwrap();
        
        let source_file = source_dir.path().join("test_image.png");
        fs::write(&source_file, b"fake image data").await.unwrap();

        // Store
        let uri = store_attachment(vault_dir.path(), &source_file).await.unwrap();
        assert!(uri.starts_with("noda://attachments/xxh3_"));
        assert!(uri.ends_with(".png"));

        // Store same file again (deduplication)
        let uri2 = store_attachment(vault_dir.path(), &source_file).await.unwrap();
        assert_eq!(uri, uri2);

        // Resolve
        let resolved = resolve_path(vault_dir.path(), &uri).unwrap();
        assert!(resolved.exists());
        let content = fs::read(&resolved).await.unwrap();
        assert_eq!(content, b"fake image data");

        // List (should still be only 1 since it's deduplicated)
        let list = list_attachments(vault_dir.path()).await.unwrap();
        assert_eq!(list.len(), 1);

        // Delete
        let file_name = uri.trim_start_matches("noda://attachments/");
        delete_attachment(vault_dir.path(), file_name).await.unwrap();
        assert!(!resolved.exists());
    }
}
