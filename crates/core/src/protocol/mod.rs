//! Custom protocol implementation (`noda://`)

pub mod security;

use crate::errors::NodaError;
use mime_guess::from_path;
use std::path::Path;

/// Serves a file securely for the custom `noda://` protocol.
/// Returns a tuple of `(file_bytes, mime_type)`
pub async fn serve_file<P: AsRef<Path>>(
    vault_path: P,
    uri: &str,
) -> Result<(Vec<u8>, String), NodaError> {
    // 1. Resolve URI to raw path
    let raw_path = crate::attachments::resolve_path(vault_path.as_ref(), uri)?;

    // 2. Security: Canonicalize and validate it's strictly inside `.noda/attachments/`
    let secure_path = security::validate_path(vault_path.as_ref(), &raw_path)?;

    // 3. Read file
    let bytes = tokio::fs::read(&secure_path).await.map_err(NodaError::Io)?;

    // 4. Determine MIME type
    let mime_type = from_path(&secure_path)
        .first_or_octet_stream()
        .to_string();

    Ok((bytes, mime_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_protocol_serve_file() {
        let vault_dir = tempdir().unwrap();
        let attach_dir = vault_dir.path().join(".noda").join("attachments");
        fs::create_dir_all(&attach_dir).await.unwrap();

        let image_path = attach_dir.join("test_image.png");
        fs::write(&image_path, b"fake png data").await.unwrap();

        let (bytes, mime) = serve_file(vault_dir.path(), "noda://attachments/test_image.png")
            .await
            .unwrap();

        assert_eq!(bytes, b"fake png data");
        assert_eq!(mime, "image/png");
    }

    #[tokio::test]
    async fn test_protocol_path_traversal() {
        let vault_dir = tempdir().unwrap();
        let attach_dir = vault_dir.path().join(".noda").join("attachments");
        fs::create_dir_all(&attach_dir).await.unwrap();
        
        let secret_path = vault_dir.path().join("secret.txt");
        fs::write(&secret_path, b"secret").await.unwrap();

        let result = serve_file(vault_dir.path(), "noda://attachments/../secret.txt").await;
        
        assert!(matches!(result, Err(NodaError::PathTraversal(_))));
    }
}
