//! # niwa-core
//!
//! Core library for NIWA Expertise Graph management.
//!
//! ## Features
//!
//! - SQLite-based Expertise storage with versioning
//! - Full-text search with FTS5
//! - Dependency graph (Relations)
//! - Type-safe operations with llm-toolkit Expertise types
//!
//! ## Example
//!
//! ```no_run
//! use niwa_core::{Database, Expertise, Scope, SearchOptions, StorageOperations};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize database
//!     let db = Database::open("~/.niwa/graph.db").await?;
//!
//!     // Create expertise
//!     let mut expertise = Expertise::new("rust-expert", "1.0.0");
//!     expertise.metadata.scope = Scope::Personal;
//!
//!     // Store
//!     db.storage().create(expertise).await?;
//!
//!     // Query
//!     let results = db.query().search("rust error handling", SearchOptions::default()).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod db;
pub mod error;
pub mod graph;
pub mod query;
pub mod storage;
pub mod types;

// Re-exports for convenience
pub use db::Database;
pub use error::{Error, Result};
pub use graph::{GraphOperations, RelationType};
pub use query::{QueryBuilder, SearchOptions};
pub use storage::{Storage, StorageOperations};
pub use types::{Expertise, ExpertiseMetadata, KnowledgeFragment, Scope, WeightedFragment};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
