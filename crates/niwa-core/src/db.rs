//! Database connection management

use crate::{Error, GraphOperations, QueryBuilder, Result, Storage};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::{debug, info};

/// Database handle
///
/// This is the main entry point for all database operations.
/// It manages the SQLite connection pool and provides access to
/// storage, query, and graph operations.
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Open or create a database at the given path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the SQLite database file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use niwa_core::Database;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let db = Database::open("~/.niwa/graph.db").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = Self::expand_path(path)?;
        info!("Opening database at: {}", path.display());

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Configure SQLite connection
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true)
            .foreign_keys(true) // Enable foreign key constraints
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal); // Use WAL mode for better concurrency

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };

        // Run migrations
        db.migrate().await?;

        Ok(db)
    }

    /// Open database at the default location (~/.niwa/graph.db)
    pub async fn open_default() -> Result<Self> {
        let path = Self::default_path()?;
        Self::open(path).await
    }

    /// Get the default database path
    pub fn default_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| Error::Other("HOME environment variable not set".to_string()))?;
        Ok(PathBuf::from(home).join(".niwa").join("graph.db"))
    }

    /// Run database migrations
    async fn migrate(&self) -> Result<()> {
        info!("Running database migrations");

        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| Error::Migration(e.to_string()))?;

        debug!("Migrations completed successfully");
        Ok(())
    }

    /// Get a reference to the storage operations
    pub fn storage(&self) -> Storage {
        Storage::new(self.pool.clone())
    }

    /// Get a query builder
    pub fn query(&self) -> QueryBuilder {
        QueryBuilder::new(self.pool.clone())
    }

    /// Get a reference to the graph operations
    pub fn graph(&self) -> GraphOperations {
        GraphOperations::new(self.pool.clone())
    }

    /// Get the underlying pool (for advanced usage)
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Close the database connection
    pub async fn close(self) {
        self.pool.close().await;
    }

    /// Expand tilde in path
    fn expand_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        let path_str = path
            .to_str()
            .ok_or_else(|| Error::Other(format!("Invalid path: {}", path.display())))?;

        if let Some(stripped) = path_str.strip_prefix("~/") {
            let home = std::env::var("HOME")
                .map_err(|_| Error::Other("HOME environment variable not set".to_string()))?;
            Ok(PathBuf::from(home).join(stripped))
        } else {
            Ok(path.to_path_buf())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_open_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::open(&db_path).await.unwrap();
        assert!(db_path.exists());

        db.close().await;
    }

    #[tokio::test]
    async fn test_migrations_run() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::open(&db_path).await.unwrap();

        // Verify tables exist
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='expertises'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();

        assert_eq!(result.0, 1, "expertises table should exist");

        db.close().await;
    }

    #[test]
    fn test_expand_path() {
        let expanded = Database::expand_path("~/test/path").unwrap();
        assert!(!expanded.to_str().unwrap().starts_with("~"));

        let normal = Database::expand_path("/absolute/path").unwrap();
        assert_eq!(normal.to_str().unwrap(), "/absolute/path");
    }
}
