//! Attachment storage logic

use crate::errors::NodaError;
use std::path::Path;
pub async fn store_attachment<P: AsRef<Path>, P2: AsRef<Path>>(
    vault_path: P,
    source_path: P2,
) -> Result<String, NodaError> {
    let source = source_path.as_ref();
    if !source.exists() || !source.is_file() {
        return Err(NodaError::NotFound("Attachment source file not found".into()));
    }

    let bytes = tokio::fs::read(source).await.map_err(NodaError::Io)?;
    let hash = xxhash_rust::xxh3::xxh3_64(&bytes);

    let extension = source.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    let dest_name = if extension.is_empty() {
        format!("xxh3_{:016x}", hash)
    } else {
        format!("xxh3_{:016x}.{}", hash, extension)
    };

    let attachments_dir = vault_path.as_ref().join(".noda").join("attachments");
    tokio::fs::create_dir_all(&attachments_dir).await.map_err(NodaError::Io)?;

    let dest_path = attachments_dir.join(&dest_name);
    if !dest_path.exists() {
        tokio::fs::write(&dest_path, &bytes).await.map_err(NodaError::Io)?;
    }

    // Return the custom protocol URI
    Ok(format!("noda://attachments/{}", dest_name))
}

pub async fn delete_attachment<P: AsRef<Path>>(
    vault_path: P,
    attachment_name: &str,
) -> Result<(), NodaError> {
    let attachments_dir = vault_path.as_ref().join(".noda").join("attachments");
    let dest_path = attachments_dir.join(attachment_name);

    if dest_path.exists() {
        tokio::fs::remove_file(dest_path).await.map_err(NodaError::Io)?;
    }
    
    Ok(())
}

pub async fn list_attachments<P: AsRef<Path>>(
    vault_path: P,
) -> Result<Vec<String>, NodaError> {
    let attachments_dir = vault_path.as_ref().join(".noda").join("attachments");
    if !attachments_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(attachments_dir).await.map_err(NodaError::Io)?;
    
    while let Some(file) = dir.next_entry().await.map_err(NodaError::Io)? {
        let path = file.path();
        if path.is_file() {
            if let Some(name) = path.file_name() {
                entries.push(name.to_string_lossy().to_string());
            }
        }
    }

    Ok(entries)
}
