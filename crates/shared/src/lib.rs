//! # noda-shared
//!
//! Shared data transfer objects, IPC models, and error types used by both
//! `crates/core` and `crates/tauri-shell`. No business logic lives here.

pub mod dto;
pub mod error;

pub use error::AppError;
