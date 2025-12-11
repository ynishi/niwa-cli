//! Error types for niwa-generator

use thiserror::Error;

/// Result type for niwa-generator operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for niwa-generator
#[derive(Error, Debug)]
pub enum Error {
    /// LLM error
    #[error("LLM error: {0}")]
    Llm(String),

    /// Invalid log format
    #[error("Invalid log format: {0}")]
    InvalidLogFormat(String),

    /// Schema validation error
    #[error("Schema validation error: {0}")]
    SchemaValidation(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Core library error
    #[error("Core error: {0}")]
    Core(#[from] niwa_core::Error),

    /// Agent error from llm-toolkit
    #[error("Agent error: {0}")]
    Agent(#[from] llm_toolkit::agent::AgentError),

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
