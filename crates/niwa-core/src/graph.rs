//! Graph operations for managing Expertise relations

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use tracing::debug;

/// Relation type between expertises
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationType {
    /// One expertise uses another
    Uses,
    /// One expertise extends another
    Extends,
    /// Two expertises conflict
    Conflicts,
    /// One expertise requires another
    Requires,
}

impl RelationType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationType::Uses => "uses",
            RelationType::Extends => "extends",
            RelationType::Conflicts => "conflicts",
            RelationType::Requires => "requires",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "uses" => Ok(RelationType::Uses),
            "extends" => Ok(RelationType::Extends),
            "conflicts" => Ok(RelationType::Conflicts),
            "requires" => Ok(RelationType::Requires),
            _ => Err(Error::InvalidRelationType(s.to_string())),
        }
    }

    /// Get all relation types
    pub fn all() -> &'static [RelationType] {
        &[
            RelationType::Uses,
            RelationType::Extends,
            RelationType::Conflicts,
            RelationType::Requires,
        ]
    }
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A relation between two expertises
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from_id: String,
    pub to_id: String,
    pub relation_type: RelationType,
    pub metadata: Option<String>,
    pub created_at: i64,
}

/// Graph operations for managing relations
#[derive(Clone)]
pub struct GraphOperations {
    pool: SqlitePool,
}

impl GraphOperations {
    /// Create a new GraphOperations instance
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a relation between two expertises
    ///
    /// # Arguments
    ///
    /// * `from_id` - Source expertise ID
    /// * `to_id` - Target expertise ID
    /// * `relation_type` - Type of relation
    /// * `metadata` - Optional JSON metadata
    ///
    /// # Example
    ///
    /// ```no_run
    /// use niwa_core::{Database, RelationType};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let db = Database::open_default().await?;
    ///
    ///     db.graph().create_relation(
    ///         "rust-expert",
    ///         "error-handling",
    ///         RelationType::Uses,
    ///         None
    ///     ).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_relation(
        &self,
        from_id: &str,
        to_id: &str,
        relation_type: RelationType,
        metadata: Option<String>,
    ) -> Result<()> {
        debug!("Creating relation: {} -[{}]-> {}", from_id, relation_type, to_id);

        // Check for circular dependency
        if self.would_create_cycle(from_id, to_id).await? {
            return Err(Error::CircularDependency {
                from: from_id.to_string(),
                to: to_id.to_string(),
            });
        }

        let created_at = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO relations (from_id, to_id, relation_type, metadata, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(from_id)
        .bind(to_id)
        .bind(relation_type.as_str())
        .bind(&metadata)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        debug!("Created relation successfully");
        Ok(())
    }

    /// Delete a relation
    pub async fn delete_relation(
        &self,
        from_id: &str,
        to_id: &str,
        relation_type: RelationType,
    ) -> Result<()> {
        debug!("Deleting relation: {} -[{}]-> {}", from_id, relation_type, to_id);

        sqlx::query(
            r#"
            DELETE FROM relations
            WHERE from_id = ? AND to_id = ? AND relation_type = ?
            "#,
        )
        .bind(from_id)
        .bind(to_id)
        .bind(relation_type.as_str())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get outgoing relations from an expertise
    pub async fn get_outgoing(&self, from_id: &str) -> Result<Vec<Relation>> {
        debug!("Getting outgoing relations for: {}", from_id);

        let rows: Vec<(String, String, String, Option<String>, i64)> = sqlx::query_as(
            r#"
            SELECT from_id, to_id, relation_type, metadata, created_at
            FROM relations
            WHERE from_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(from_id)
        .fetch_all(&self.pool)
        .await?;

        let mut relations = Vec::with_capacity(rows.len());
        for (from_id, to_id, relation_type, metadata, created_at) in rows {
            relations.push(Relation {
                from_id,
                to_id,
                relation_type: RelationType::from_str(&relation_type)?,
                metadata,
                created_at,
            });
        }

        Ok(relations)
    }

    /// Get incoming relations to an expertise
    pub async fn get_incoming(&self, to_id: &str) -> Result<Vec<Relation>> {
        debug!("Getting incoming relations for: {}", to_id);

        let rows: Vec<(String, String, String, Option<String>, i64)> = sqlx::query_as(
            r#"
            SELECT from_id, to_id, relation_type, metadata, created_at
            FROM relations
            WHERE to_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(to_id)
        .fetch_all(&self.pool)
        .await?;

        let mut relations = Vec::with_capacity(rows.len());
        for (from_id, to_id, relation_type, metadata, created_at) in rows {
            relations.push(Relation {
                from_id,
                to_id,
                relation_type: RelationType::from_str(&relation_type)?,
                metadata,
                created_at,
            });
        }

        Ok(relations)
    }

    /// Get all relations for an expertise (both incoming and outgoing)
    pub async fn get_all_relations(&self, id: &str) -> Result<Vec<Relation>> {
        debug!("Getting all relations for: {}", id);

        let rows: Vec<(String, String, String, Option<String>, i64)> = sqlx::query_as(
            r#"
            SELECT from_id, to_id, relation_type, metadata, created_at
            FROM relations
            WHERE from_id = ? OR to_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(id)
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let mut relations = Vec::with_capacity(rows.len());
        for (from_id, to_id, relation_type, metadata, created_at) in rows {
            relations.push(Relation {
                from_id,
                to_id,
                relation_type: RelationType::from_str(&relation_type)?,
                metadata,
                created_at,
            });
        }

        Ok(relations)
    }

    /// Get dependencies (expertises that this expertise depends on)
    pub async fn get_dependencies(&self, id: &str) -> Result<Vec<String>> {
        debug!("Getting dependencies for: {}", id);

        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT to_id
            FROM relations
            WHERE from_id = ? AND relation_type IN ('uses', 'requires', 'extends')
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Get dependents (expertises that depend on this expertise)
    pub async fn get_dependents(&self, id: &str) -> Result<Vec<String>> {
        debug!("Getting dependents for: {}", id);

        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT from_id
            FROM relations
            WHERE to_id = ? AND relation_type IN ('uses', 'requires', 'extends')
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Check if adding a relation would create a cycle
    async fn would_create_cycle(&self, from_id: &str, to_id: &str) -> Result<bool> {
        // If we're creating from -> to, check if there's already a path from to -> from
        // This would create a cycle

        let reachable = self.get_reachable_nodes(to_id).await?;
        Ok(reachable.contains(from_id))
    }

    /// Get all nodes reachable from a given node (DFS)
    async fn get_reachable_nodes(&self, start_id: &str) -> Result<HashSet<String>> {
        let mut reachable = HashSet::new();
        let mut to_visit = vec![start_id.to_string()];

        while let Some(current) = to_visit.pop() {
            if reachable.contains(&current) {
                continue;
            }

            reachable.insert(current.clone());

            let deps = self.get_dependencies(&current).await?;
            for dep in deps {
                if !reachable.contains(&dep) {
                    to_visit.push(dep);
                }
            }
        }

        Ok(reachable)
    }

    /// Build a full dependency graph
    pub async fn build_graph(&self) -> Result<HashMap<String, Vec<String>>> {
        debug!("Building full dependency graph");

        let rows: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT DISTINCT from_id, to_id
            FROM relations
            WHERE relation_type IN ('uses', 'requires', 'extends')
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        for (from_id, to_id) in rows {
            graph.entry(from_id).or_default().push(to_id);
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Database, Expertise, Scope, StorageOperations};
    use tempfile::TempDir;

    async fn setup_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).await.unwrap();
        (db, temp_dir)
    }

    async fn create_test_expertise(db: &Database, id: &str) {
        let mut exp = Expertise::new(id, "1.0.0");
        exp.metadata.scope = Scope::Personal;
        db.storage().create(exp).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_relation() {
        let (db, _temp) = setup_db().await;

        create_test_expertise(&db, "exp-1").await;
        create_test_expertise(&db, "exp-2").await;

        db.graph()
            .create_relation("exp-1", "exp-2", RelationType::Uses, None)
            .await
            .unwrap();

        let outgoing = db.graph().get_outgoing("exp-1").await.unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].to_id, "exp-2");
        assert_eq!(outgoing[0].relation_type, RelationType::Uses);
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let (db, _temp) = setup_db().await;

        create_test_expertise(&db, "exp-1").await;
        create_test_expertise(&db, "exp-2").await;
        create_test_expertise(&db, "exp-3").await;

        // Create chain: 1 -> 2 -> 3
        db.graph()
            .create_relation("exp-1", "exp-2", RelationType::Uses, None)
            .await
            .unwrap();
        db.graph()
            .create_relation("exp-2", "exp-3", RelationType::Uses, None)
            .await
            .unwrap();

        // Try to create cycle: 3 -> 1 (should fail)
        let result = db.graph()
            .create_relation("exp-3", "exp-1", RelationType::Uses, None)
            .await;

        assert!(matches!(result, Err(Error::CircularDependency { .. })));
    }

    #[tokio::test]
    async fn test_get_dependencies() {
        let (db, _temp) = setup_db().await;

        create_test_expertise(&db, "exp-1").await;
        create_test_expertise(&db, "exp-2").await;
        create_test_expertise(&db, "exp-3").await;

        db.graph()
            .create_relation("exp-1", "exp-2", RelationType::Uses, None)
            .await
            .unwrap();
        db.graph()
            .create_relation("exp-1", "exp-3", RelationType::Requires, None)
            .await
            .unwrap();

        let deps = db.graph().get_dependencies("exp-1").await.unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"exp-2".to_string()));
        assert!(deps.contains(&"exp-3".to_string()));
    }

    #[tokio::test]
    async fn test_get_dependents() {
        let (db, _temp) = setup_db().await;

        create_test_expertise(&db, "exp-1").await;
        create_test_expertise(&db, "exp-2").await;
        create_test_expertise(&db, "exp-3").await;

        db.graph()
            .create_relation("exp-2", "exp-1", RelationType::Uses, None)
            .await
            .unwrap();
        db.graph()
            .create_relation("exp-3", "exp-1", RelationType::Requires, None)
            .await
            .unwrap();

        let dependents = db.graph().get_dependents("exp-1").await.unwrap();
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&"exp-2".to_string()));
        assert!(dependents.contains(&"exp-3".to_string()));
    }

    #[tokio::test]
    async fn test_delete_relation() {
        let (db, _temp) = setup_db().await;

        create_test_expertise(&db, "exp-1").await;
        create_test_expertise(&db, "exp-2").await;

        db.graph()
            .create_relation("exp-1", "exp-2", RelationType::Uses, None)
            .await
            .unwrap();

        db.graph()
            .delete_relation("exp-1", "exp-2", RelationType::Uses)
            .await
            .unwrap();

        let outgoing = db.graph().get_outgoing("exp-1").await.unwrap();
        assert_eq!(outgoing.len(), 0);
    }
}
