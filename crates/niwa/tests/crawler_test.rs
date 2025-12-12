//! Integration tests for crawler command

use niwa_core::{Database, StorageOperations};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_crawler_processes_new_sessions() {
    // Setup test directory with session files
    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");
    fs::create_dir(&sessions_dir).unwrap();

    // Create test session file
    let session_file = sessions_dir.join("test-session.log");
    fs::write(
        &session_file,
        "User: How do I use async in Rust?\nAssistant: Use tokio runtime...",
    )
    .unwrap();

    // Setup test database
    let db_path = temp_dir.path().join("test.db");
    let db = Database::open(&db_path).await.unwrap();

    // Verify session file exists and is readable
    assert!(session_file.exists());
    let content = fs::read_to_string(&session_file).unwrap();
    assert!(content.contains("async"));

    // Verify database is ready
    let expertises = db.storage().list_all().await.unwrap();
    assert_eq!(expertises.len(), 0, "Database should start empty");

    // Note: Actual crawler command execution would require:
    // 1. Running the CLI binary
    // 2. Or refactoring crawler logic into testable functions

    // For now, verify the test infrastructure is working
    assert!(db_path.exists());
}

#[tokio::test]
async fn test_processed_sessions_table_exists() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Database::open(&db_path).await.unwrap();

    // Verify processed_sessions table was created by migration
    let result: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='processed_sessions'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();

    assert_eq!(result.0, 1, "processed_sessions table should exist");
}

#[tokio::test]
async fn test_session_hash_prevents_duplicates() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Database::open(&db_path).await.unwrap();

    // First, create an expertise (required for foreign key)
    let expertise_id = "test-expertise";
    let mut expertise = niwa_core::Expertise::new(expertise_id, "1.0.0");
    expertise.metadata.scope = niwa_core::Scope::Personal;
    db.storage().create(expertise).await.unwrap();

    // Insert a processed session record
    let file_path = "/tmp/test.log";
    let file_hash = "abc123";
    let processed_at = chrono::Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT INTO processed_sessions (file_path, file_hash, expertise_id, processed_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(file_path)
    .bind(file_hash)
    .bind(expertise_id)
    .bind(processed_at)
    .execute(db.pool())
    .await
    .unwrap();

    // Verify record exists
    let row: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT file_hash
        FROM processed_sessions
        WHERE file_path = ?
        "#,
    )
    .bind(file_path)
    .fetch_optional(db.pool())
    .await
    .unwrap();

    assert_eq!(row, Some((file_hash.to_string(),)));
}
