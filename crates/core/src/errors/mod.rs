// Defines core error types for the application.

use shared::AppError;
use thiserror::Error;

/// The central error type for the core domain.
#[derive(Debug, Error)]
pub enum NodaError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Vault error: {0}")]
    Vault(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Frontmatter error: {0}")]
    Frontmatter(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Watch error: {0}")]
    Watch(String),

    #[error("Path traversal detected: {0}")]
    PathTraversal(String),

    #[error("Duplicate filename: {0}")]
    DuplicateFilename(String),

    #[error("Resource not found: {0}")]
    NotFound(String),
}

impl From<NodaError> for AppError {
    fn from(error: NodaError) -> Self {
        AppError {
            code: match &error {
                NodaError::Io(_) => "IO_ERROR".to_string(),
                NodaError::Database(_) => "DATABASE_ERROR".to_string(),
                NodaError::Vault(_) => "VAULT_ERROR".to_string(),
                NodaError::Sync(_) => "SYNC_ERROR".to_string(),
                NodaError::Frontmatter(_) => "FRONTMATTER_ERROR".to_string(),
                NodaError::Search(_) => "SEARCH_ERROR".to_string(),
                NodaError::Watch(_) => "WATCH_ERROR".to_string(),
                NodaError::PathTraversal(_) => "PATH_TRAVERSAL".to_string(),
                NodaError::DuplicateFilename(_) => "DUPLICATE_FILENAME".to_string(),
                NodaError::NotFound(_) => "NOT_FOUND".to_string(),
            },
            message: error.to_string(),
        }
    }
}
