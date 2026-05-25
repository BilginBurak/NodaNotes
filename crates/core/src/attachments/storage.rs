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
    store_attachment_bytes(vault_path, &bytes, &source.to_string_lossy()).await
}

pub async fn store_attachment_bytes<P: AsRef<Path>>(
    vault_path: P,
    bytes: &[u8],
    original_filename: &str,
) -> Result<String, NodaError> {
    let hash = xxhash_rust::xxh3::xxh3_64(bytes);

    let path = Path::new(original_filename);
    let extension = path.extension()
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
        tokio::fs::write(&dest_path, bytes).await.map_err(NodaError::Io)?;
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AttachmentInfo {
    pub name: String,
    pub modified_at: u64, // Epoch millis
    pub size: u64, // Bytes
}

pub async fn list_attachments_with_metadata<P: AsRef<Path>>(
    vault_path: P,
) -> Result<Vec<AttachmentInfo>, NodaError> {
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
                let metadata = file.metadata().await.map_err(NodaError::Io)?;
                let modified = metadata.modified().ok()
                    .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let size = metadata.len();
                
                entries.push(AttachmentInfo {
                    name: name.to_string_lossy().to_string(),
                    modified_at: modified,
                    size,
                });
            }
        }
    }

    // Sort by modified_at descending (latest modified/added first)
    entries.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    Ok(entries)
}

