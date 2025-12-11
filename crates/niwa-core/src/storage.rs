//! Storage operations for Expertise CRUD

use crate::{Error, Expertise, Result, Scope};
use async_trait::async_trait;
use sqlx::SqlitePool;
use tracing::{debug, info};

/// Storage operations interface
#[async_trait]
pub trait StorageOperations {
    /// Create a new expertise
    async fn create(&self, expertise: Expertise) -> Result<()>;

    /// Get an expertise by ID and scope
    async fn get(&self, id: &str, scope: Scope) -> Result<Option<Expertise>>;

    /// Update an existing expertise
    async fn update(&self, expertise: Expertise) -> Result<()>;

    /// Delete an expertise
    async fn delete(&self, id: &str, scope: Scope) -> Result<()>;

    /// List all expertises in a scope
    async fn list(&self, scope: Scope) -> Result<Vec<Expertise>>;

    /// List all expertises across all scopes
    async fn list_all(&self) -> Result<Vec<Expertise>>;

    /// Check if an expertise exists
    async fn exists(&self, id: &str, scope: Scope) -> Result<bool>;
}

/// Storage implementation
#[derive(Clone)]
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    /// Create a new Storage instance
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StorageOperations for Storage {
    async fn create(&self, expertise: Expertise) -> Result<()> {
        let id = expertise.id();
        let scope = expertise.metadata.scope;

        info!("Creating expertise: {} (scope: {})", id, scope);

        // Check if already exists
        if self.exists(id, scope).await? {
            return Err(Error::AlreadyExists {
                id: id.to_string(),
                scope: scope.to_string(),
            });
        }

        // Serialize expertise
        let data_json = expertise.to_json()?;
        let description = expertise.description();

        // Insert into expertises table
        sqlx::query(
            r#"
            INSERT INTO expertises (id, version, scope, created_at, updated_at, data_json, description)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(expertise.version())
        .bind(scope.as_str())
        .bind(expertise.metadata.created_at)
        .bind(expertise.metadata.updated_at)
        .bind(&data_json)
        .bind(&description)
        .execute(&self.pool)
        .await?;

        // Insert tags
        for tag in expertise.tags() {
            sqlx::query(
                r#"
                INSERT INTO tags (expertise_id, tag)
                VALUES (?, ?)
                "#,
            )
            .bind(id)
            .bind(tag)
            .execute(&self.pool)
            .await?;
        }

        debug!("Created expertise: {}", id);
        Ok(())
    }

    async fn get(&self, id: &str, scope: Scope) -> Result<Option<Expertise>> {
        debug!("Getting expertise: {} (scope: {})", id, scope);

        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT data_json
            FROM expertises
            WHERE id = ? AND scope = ?
            "#,
        )
        .bind(id)
        .bind(scope.as_str())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((data_json,)) => {
                let expertise = Expertise::from_json(&data_json)?;
                Ok(Some(expertise))
            }
            None => Ok(None),
        }
    }

    async fn update(&self, mut expertise: Expertise) -> Result<()> {
        let id = expertise.id().to_string();
        let scope = expertise.metadata.scope;

        info!("Updating expertise: {} (scope: {})", id, scope);

        // Check if exists
        if !self.exists(&id, scope).await? {
            return Err(Error::NotFound {
                id: id.clone(),
                scope: scope.to_string(),
            });
        }

        // Get existing expertise for versioning
        if let Some(existing) = self.get(&id, scope).await? {
            // Save old version
            self.save_version(&existing).await?;
        }

        // Serialize expertise
        expertise.metadata.touch(); // Update timestamp
        let data_json = expertise.to_json()?;
        let description = expertise.description();
        let version = expertise.version().to_string();

        // Update expertises table
        sqlx::query(
            r#"
            UPDATE expertises
            SET version = ?, updated_at = ?, data_json = ?, description = ?
            WHERE id = ? AND scope = ?
            "#,
        )
        .bind(&version)
        .bind(expertise.metadata.updated_at)
        .bind(&data_json)
        .bind(&description)
        .bind(&id)
        .bind(scope.as_str())
        .execute(&self.pool)
        .await?;

        // Update tags (delete old, insert new)
        sqlx::query("DELETE FROM tags WHERE expertise_id = ?")
            .bind(&id)
            .execute(&self.pool)
            .await?;

        for tag in expertise.tags() {
            sqlx::query("INSERT INTO tags (expertise_id, tag) VALUES (?, ?)")
                .bind(&id)
                .bind(tag)
                .execute(&self.pool)
                .await?;
        }

        debug!("Updated expertise: {}", id);
        Ok(())
    }

    async fn delete(&self, id: &str, scope: Scope) -> Result<()> {
        info!("Deleting expertise: {} (scope: {})", id, scope);

        let result = sqlx::query("DELETE FROM expertises WHERE id = ? AND scope = ?")
            .bind(id)
            .bind(scope.as_str())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound {
                id: id.to_string(),
                scope: scope.to_string(),
            });
        }

        // Tags are automatically deleted by CASCADE
        debug!("Deleted expertise: {}", id);
        Ok(())
    }

    async fn list(&self, scope: Scope) -> Result<Vec<Expertise>> {
        debug!("Listing expertises in scope: {}", scope);

        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT data_json
            FROM expertises
            WHERE scope = ?
            ORDER BY updated_at DESC
            "#,
        )
        .bind(scope.as_str())
        .fetch_all(&self.pool)
        .await?;

        let mut expertises = Vec::with_capacity(rows.len());
        for (data_json,) in rows {
            expertises.push(Expertise::from_json(&data_json)?);
        }

        Ok(expertises)
    }

    async fn list_all(&self) -> Result<Vec<Expertise>> {
        debug!("Listing all expertises");

        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT data_json
            FROM expertises
            ORDER BY scope, updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut expertises = Vec::with_capacity(rows.len());
        for (data_json,) in rows {
            expertises.push(Expertise::from_json(&data_json)?);
        }

        Ok(expertises)
    }

    async fn exists(&self, id: &str, scope: Scope) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM expertises
            WHERE id = ? AND scope = ?
            "#,
        )
        .bind(id)
        .bind(scope.as_str())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0 > 0)
    }
}

impl Storage {
    /// Save a version to the versions table
    async fn save_version(&self, expertise: &Expertise) -> Result<()> {
        let id = expertise.id();
        let version = expertise.version();
        let data_json = expertise.to_json()?;
        let created_at = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO versions (expertise_id, version, created_at, data_json)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(version)
        .bind(created_at)
        .bind(&data_json)
        .execute(&self.pool)
        .await?;

        debug!("Saved version: {} v{}", id, version);
        Ok(())
    }

    /// Get a specific version
    pub async fn get_version(&self, id: &str, version: &str) -> Result<Option<Expertise>> {
        debug!("Getting expertise version: {} v{}", id, version);

        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT data_json
            FROM versions
            WHERE expertise_id = ? AND version = ?
            "#,
        )
        .bind(id)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((data_json,)) => Ok(Some(Expertise::from_json(&data_json)?)),
            None => Ok(None),
        }
    }

    /// List all versions of an expertise
    pub async fn list_versions(&self, id: &str) -> Result<Vec<String>> {
        debug!("Listing versions for expertise: {}", id);

        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT version
            FROM versions
            WHERE expertise_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(v,)| v).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use tempfile::TempDir;

    async fn setup_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).await.unwrap();
        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let (db, _temp) = setup_db().await;
        let storage = db.storage();

        let mut expertise = Expertise::new("test-id", "1.0.0");
        expertise.metadata.scope = Scope::Personal;

        storage.create(expertise.clone()).await.unwrap();

        let retrieved = storage.get("test-id", Scope::Personal).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id(), "test-id");
        assert_eq!(retrieved.version(), "1.0.0");
    }

    #[tokio::test]
    async fn test_create_duplicate_fails() {
        let (db, _temp) = setup_db().await;
        let storage = db.storage();

        let mut expertise = Expertise::new("test-id", "1.0.0");
        expertise.metadata.scope = Scope::Personal;

        storage.create(expertise.clone()).await.unwrap();

        let result = storage.create(expertise).await;
        assert!(matches!(result, Err(Error::AlreadyExists { .. })));
    }

    #[tokio::test]
    async fn test_update() {
        let (db, _temp) = setup_db().await;
        let storage = db.storage();

        let mut expertise = Expertise::new("test-id", "1.0.0");
        expertise.metadata.scope = Scope::Personal;

        storage.create(expertise.clone()).await.unwrap();

        // Update version
        expertise.inner.version = "2.0.0".to_string();
        storage.update(expertise).await.unwrap();

        let retrieved = storage.get("test-id", Scope::Personal).await.unwrap().unwrap();
        assert_eq!(retrieved.version(), "2.0.0");
    }

    #[tokio::test]
    async fn test_delete() {
        let (db, _temp) = setup_db().await;
        let storage = db.storage();

        let mut expertise = Expertise::new("test-id", "1.0.0");
        expertise.metadata.scope = Scope::Personal;

        storage.create(expertise).await.unwrap();
        storage.delete("test-id", Scope::Personal).await.unwrap();

        let retrieved = storage.get("test-id", Scope::Personal).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list() {
        let (db, _temp) = setup_db().await;
        let storage = db.storage();

        let mut exp1 = Expertise::new("test-1", "1.0.0");
        exp1.metadata.scope = Scope::Personal;

        let mut exp2 = Expertise::new("test-2", "1.0.0");
        exp2.metadata.scope = Scope::Personal;

        storage.create(exp1).await.unwrap();
        storage.create(exp2).await.unwrap();

        let list = storage.list(Scope::Personal).await.unwrap();
        assert_eq!(list.len(), 2);
    }
}
