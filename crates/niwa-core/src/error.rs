//! Error types for niwa-core

use thiserror::Error;

/// Result type for niwa-core operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for niwa-core
#[derive(Error, Debug)]
pub enum Error {
    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Expertise not found
    #[error("Expertise not found: {id} (scope: {scope})")]
    NotFound { id: String, scope: String },

    /// Expertise already exists
    #[error("Expertise already exists: {id} (scope: {scope})")]
    AlreadyExists { id: String, scope: String },

    /// Invalid scope
    #[error("Invalid scope: {0}")]
    InvalidScope(String),

    /// Invalid relation type
    #[error("Invalid relation type: {0}")]
    InvalidRelationType(String),

    /// Circular dependency detected
    #[error("Circular dependency detected: {from} -> {to}")]
    CircularDependency { from: String, to: String },

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Other(s.to_string())
    }
}
