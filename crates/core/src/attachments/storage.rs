//! Attachment storage logic

use crate::errors::NodaError;
use std::path::Path;
use ulid::Ulid;

pub async fn store_attachment<P: AsRef<Path>, P2: AsRef<Path>>(
    vault_path: P,
    source_path: P2,
) -> Result<String, NodaError> {
    let source = source_path.as_ref();
    if !source.exists() || !source.is_file() {
        return Err(NodaError::NotFound("Attachment source file not found".into()));
    }

    let file_name = source.file_name()
        .ok_or_else(|| NodaError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file name")))?
        .to_string_lossy()
        .to_string();

    let id = Ulid::new().to_string();
    let dest_name = format!("{}_{}", id, file_name);

    let attachments_dir = vault_path.as_ref().join(".noda").join("attachments");
    tokio::fs::create_dir_all(&attachments_dir).await.map_err(NodaError::Io)?;

    let dest_path = attachments_dir.join(&dest_name);
    tokio::fs::copy(source, &dest_path).await.map_err(NodaError::Io)?;

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
