//! Query and search operations

use crate::{Expertise, Result, Scope};
use sqlx::SqlitePool;
use tracing::debug;

/// Search options
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
    /// Filter by scope
    pub scope: Option<Scope>,
    /// Filter by tags (AND condition)
    pub tags: Vec<String>,
}

impl SearchOptions {
    /// Create a new SearchOptions
    pub fn new() -> Self {
        Self::default()
    }

    /// Set limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Set scope filter
    pub fn scope(mut self, scope: Scope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Add tag filter
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set tags filter
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Query builder for searching expertises
#[derive(Clone)]
pub struct QueryBuilder {
    pool: SqlitePool,
}

impl QueryBuilder {
    /// Create a new QueryBuilder
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Full-text search using FTS5
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `options` - Search options (limit, offset, filters)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use niwa_core::{Database, SearchOptions};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let db = Database::open_default().await?;
    ///
    ///     let options = SearchOptions::new().limit(10);
    ///     let results = db.query().search("rust error handling", options).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<Expertise>> {
        debug!("Searching for: {}", query);

        let mut sql = String::from(
            r#"
            SELECT e.data_json
            FROM expertises e
            WHERE e.id IN (SELECT id FROM expertises_fts WHERE expertises_fts MATCH ?)
            "#,
        );

        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Sqlite> + Send>> = vec![];
        params.push(Box::new(query.to_string()));

        // Add scope filter
        if let Some(scope) = options.scope {
            sql.push_str(" AND e.scope = ?");
            params.push(Box::new(scope.as_str().to_string()));
        }

        // Add tag filters
        if !options.tags.is_empty() {
            for tag in &options.tags {
                sql.push_str(
                    " AND e.id IN (SELECT expertise_id FROM tags WHERE tag = ?)",
                );
                params.push(Box::new(tag.clone()));
            }
        }

        sql.push_str(" ORDER BY e.updated_at DESC");

        // Add limit and offset
        if options.limit.is_some() {
            sql.push_str(" LIMIT ?");
        }
        if options.offset.is_some() {
            sql.push_str(" OFFSET ?");
        }

        // Execute query (note: this is simplified, real implementation would use proper binding)
        let mut query_builder = sqlx::query_as::<_, (String,)>(&sql);

        // Bind parameters
        query_builder = query_builder.bind(query);
        if let Some(scope) = &options.scope {
            query_builder = query_builder.bind(scope.as_str());
        }
        for tag in &options.tags {
            query_builder = query_builder.bind(tag);
        }
        if let Some(limit) = options.limit {
            query_builder = query_builder.bind(limit as i64);
        }
        if let Some(offset) = options.offset {
            query_builder = query_builder.bind(offset as i64);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut expertises = Vec::with_capacity(rows.len());
        for (data_json,) in rows {
            expertises.push(Expertise::from_json(&data_json)?);
        }

        debug!("Found {} results", expertises.len());
        Ok(expertises)
    }

    /// Filter expertises by tags
    pub async fn filter_by_tags(&self, tags: Vec<String>, options: SearchOptions) -> Result<Vec<Expertise>> {
        debug!("Filtering by tags: {:?}", tags);

        if tags.is_empty() {
            return Ok(vec![]);
        }

        let mut sql = String::from(
            r#"
            SELECT DISTINCT e.data_json
            FROM expertises e
            INNER JOIN tags t ON e.id = t.expertise_id
            WHERE t.tag IN (
            "#,
        );

        // Add placeholders for tags
        for (i, _) in tags.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push('?');
        }
        sql.push(')');

        // Add scope filter
        if options.scope.is_some() {
            sql.push_str(" AND e.scope = ?");
        }

        // Group by to ensure all tags match (AND condition)
        sql.push_str(&format!(" GROUP BY e.id HAVING COUNT(DISTINCT t.tag) = {}", tags.len()));
        sql.push_str(" ORDER BY e.updated_at DESC");

        // Add limit and offset
        if let Some(limit) = options.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = options.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let mut query_builder = sqlx::query_as::<_, (String,)>(&sql);

        // Bind tags
        for tag in &tags {
            query_builder = query_builder.bind(tag);
        }

        // Bind scope
        if let Some(scope) = &options.scope {
            query_builder = query_builder.bind(scope.as_str());
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut expertises = Vec::with_capacity(rows.len());
        for (data_json,) in rows {
            expertises.push(Expertise::from_json(&data_json)?);
        }

        debug!("Found {} results with tags {:?}", expertises.len(), tags);
        Ok(expertises)
    }

    /// List all tags with counts
    pub async fn list_tags(&self, scope: Option<Scope>) -> Result<Vec<(String, usize)>> {
        debug!("Listing tags");

        let mut sql = String::from(
            r#"
            SELECT t.tag, COUNT(*) as count
            FROM tags t
            "#,
        );

        if scope.is_some() {
            sql.push_str(
                r#"
                INNER JOIN expertises e ON t.expertise_id = e.id
                WHERE e.scope = ?
                "#,
            );
        }

        sql.push_str(" GROUP BY t.tag ORDER BY count DESC, t.tag");

        let mut query_builder = sqlx::query_as::<_, (String, i64)>(&sql);

        if let Some(scope) = scope {
            query_builder = query_builder.bind(scope.as_str());
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(|(tag, count)| (tag, count as usize)).collect())
    }

    /// Count total expertises
    pub async fn count(&self, scope: Option<Scope>) -> Result<usize> {
        let sql = if scope.is_some() {
            "SELECT COUNT(*) FROM expertises WHERE scope = ?"
        } else {
            "SELECT COUNT(*) FROM expertises"
        };

        let mut query_builder = sqlx::query_as::<_, (i64,)>(sql);

        if let Some(scope) = scope {
            query_builder = query_builder.bind(scope.as_str());
        }

        let (count,) = query_builder.fetch_one(&self.pool).await?;
        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Database, StorageOperations};
    use tempfile::TempDir;

    async fn setup_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).await.unwrap();
        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_search() {
        let (db, _temp) = setup_db().await;

        // Create test expertise
        let mut exp = Expertise::new("rust-expert", "1.0.0");
        exp.inner.description = Some("Expert in Rust error handling".to_string());
        exp.metadata.scope = Scope::Personal;

        db.storage().create(exp).await.unwrap();

        // Search
        let options = SearchOptions::new();
        let results = db.query().search("rust", options).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id(), "rust-expert");
    }

    #[tokio::test]
    async fn test_filter_by_tags() {
        let (db, _temp) = setup_db().await;

        // Create expertises with tags
        let mut exp1 = Expertise::new("exp-1", "1.0.0");
        exp1.inner.tags = vec!["rust".to_string(), "async".to_string()];
        exp1.metadata.scope = Scope::Personal;

        let mut exp2 = Expertise::new("exp-2", "1.0.0");
        exp2.inner.tags = vec!["rust".to_string()];
        exp2.metadata.scope = Scope::Personal;

        db.storage().create(exp1).await.unwrap();
        db.storage().create(exp2).await.unwrap();

        // Filter by single tag
        let options = SearchOptions::new();
        let results = db.query().filter_by_tags(vec!["rust".to_string()], options).await.unwrap();
        assert_eq!(results.len(), 2);

        // Filter by multiple tags (AND condition)
        let options = SearchOptions::new();
        let results = db.query().filter_by_tags(vec!["rust".to_string(), "async".to_string()], options).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id(), "exp-1");
    }

    #[tokio::test]
    async fn test_list_tags() {
        let (db, _temp) = setup_db().await;

        let mut exp1 = Expertise::new("exp-1", "1.0.0");
        exp1.inner.tags = vec!["rust".to_string(), "async".to_string()];
        exp1.metadata.scope = Scope::Personal;

        let mut exp2 = Expertise::new("exp-2", "1.0.0");
        exp2.inner.tags = vec!["rust".to_string()];
        exp2.metadata.scope = Scope::Personal;

        db.storage().create(exp1).await.unwrap();
        db.storage().create(exp2).await.unwrap();

        let tags = db.query().list_tags(None).await.unwrap();

        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].0, "rust");
        assert_eq!(tags[0].1, 2);
        assert_eq!(tags[1].0, "async");
        assert_eq!(tags[1].1, 1);
    }

    #[tokio::test]
    async fn test_count() {
        let (db, _temp) = setup_db().await;

        let mut exp1 = Expertise::new("exp-1", "1.0.0");
        exp1.metadata.scope = Scope::Personal;

        let mut exp2 = Expertise::new("exp-2", "1.0.0");
        exp2.metadata.scope = Scope::Company;

        db.storage().create(exp1).await.unwrap();
        db.storage().create(exp2).await.unwrap();

        let total = db.query().count(None).await.unwrap();
        assert_eq!(total, 2);

        let personal = db.query().count(Some(Scope::Personal)).await.unwrap();
        assert_eq!(personal, 1);
    }
}
