//! # AppError
//!
//! Serializable error type returned by all Tauri IPC commands.
//! Never exposes internal panic traces or stack frames.

use serde::Serialize;
use std::fmt;

/// The single error type that crosses the Tauri IPC boundary.
///
/// All internal `NodaError` variants are converted into this type via
/// `From<NodaError> for AppError` (implemented in `crates/core`).
#[derive(Debug, Serialize, Clone)]
pub struct AppError {
    /// Machine-readable error code (e.g. `"VAULT_NOT_FOUND"`, `"IO_ERROR"`).
    pub code: String,
    /// Human-readable message safe to display in the frontend UI.
    pub message: String,
}

impl AppError {
    /// Construct an `AppError` directly from a code and message.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Construct an `AppError` for internal/unexpected errors without leaking details.
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "INTERNAL_ERROR".into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}
