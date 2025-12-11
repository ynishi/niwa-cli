//! Gardener commands - automatic expertise extraction from session logs

use crate::state::AppState;
use clap::{Parser, Subcommand};
use comfy_table::{presets, Table};
use niwa_core::{Scope, StorageOperations};
use sen::{Args, CliError, CliResult, State};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Automatically extract expertise from session logs
#[derive(Parser, Debug)]
pub struct GardenArgs {
    #[command(subcommand)]
    pub command: Option<GardenCommand>,

    /// Directory to scan (if no subcommand specified)
    #[arg(value_name = "DIRECTORY")]
    pub directory: Option<PathBuf>,

    /// Scope for generated expertises (default: personal)
    #[arg(short, long, default_value = "personal")]
    pub scope: Scope,

    /// Dry run - show what would be processed without actually processing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Maximum number of files to process
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Only process files modified in the last N days
    #[arg(long)]
    pub recent_days: Option<u64>,
}

#[derive(Subcommand, Debug)]
pub enum GardenCommand {
    /// Initialize garden with preset paths (claude-code, cursor)
    Init {
        /// Preset name
        preset: String,
    },
    /// Add custom path to monitor
    Add {
        /// Directory path to monitor
        path: PathBuf,
        /// Optional preset name for reference
        #[arg(short, long)]
        name: Option<String>,
    },
    /// List registered monitoring paths
    List,
    /// Remove monitoring path
    Remove {
        /// Path ID to remove
        id: i64,
    },
}

#[derive(Debug)]
pub enum GardenPreset {
    ClaudeCode,
    Cursor,
}

impl GardenPreset {
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "claude-code" | "claude" => Ok(Self::ClaudeCode),
            "cursor" => Ok(Self::Cursor),
            _ => Err(format!("Unknown preset: {}", s)),
        }
    }

    fn get_path(&self) -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or("Could not determine home directory")?;

        match self {
            Self::ClaudeCode => Ok(home.join(".claude/projects")),
            Self::Cursor => {
                #[cfg(target_os = "macos")]
                {
                    Ok(home.join("Library/Application Support/Cursor/User/workspaceStorage"))
                }
                #[cfg(target_os = "linux")]
                {
                    Ok(home.join(".config/Cursor/User/workspaceStorage"))
                }
                #[cfg(target_os = "windows")]
                {
                    // %APPDATA%\Cursor\User\workspaceStorage
                    std::env::var("APPDATA")
                        .map(|appdata| PathBuf::from(appdata).join("Cursor/User/workspaceStorage"))
                        .map_err(|_| "Could not determine APPDATA directory".to_string())
                }
                #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
                {
                    Err("Cursor preset not supported on this platform".to_string())
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Cursor => "cursor",
        }
    }
}

#[sen::handler]
pub async fn garden(state: State<AppState>, Args(args): Args<GardenArgs>) -> CliResult<String> {
    let app = state.read().await;

    match args.command {
        Some(GardenCommand::Init { preset }) => {
            handle_init(&app, &preset).await
        }
        Some(GardenCommand::Add { path, name }) => {
            handle_add(&app, &path, name.as_deref()).await
        }
        Some(GardenCommand::List) => {
            handle_list(&app).await
        }
        Some(GardenCommand::Remove { id }) => {
            handle_remove(&app, id).await
        }
        None => {
            // Scan mode
            if let Some(directory) = args.directory {
                // Explicit directory specified
                handle_scan(&app, &directory, args.scope, args.dry_run, args.limit, args.recent_days).await
            } else {
                // Scan all registered paths
                handle_scan_registered(&app, args.scope, args.dry_run, args.limit, args.recent_days).await
            }
        }
    }
}

async fn handle_init(app: &AppState, preset_name: &str) -> CliResult<String> {
    let preset = GardenPreset::from_str(preset_name)
        .map_err(|e| CliError::user(format!("{}\n\nAvailable presets: claude-code, cursor", e)))?;

    let path = preset.get_path()
        .map_err(|e| CliError::system(e))?;

    // Check if path exists
    if !path.exists() {
        return Err(CliError::user(format!(
            "Preset path does not exist: {}\n\nMake sure {} is installed.",
            path.display(),
            preset.name()
        )));
    }

    // Add to database
    let now = chrono::Utc::now().timestamp();
    let path_str = path.to_string_lossy();

    sqlx::query(
        r#"
        INSERT INTO garden_paths (path, preset_name, enabled, added_at)
        VALUES (?, ?, 1, ?)
        ON CONFLICT(path) DO UPDATE SET enabled = 1
        "#,
    )
    .bind(&*path_str)
    .bind(preset.name())
    .bind(now)
    .execute(app.db.pool())
    .await
    .map_err(|e| CliError::system(format!("Database error: {}", e)))?;

    Ok(format!(
        "✓ Initialized {} garden monitoring\n  Path: {}",
        preset.name(),
        path.display()
    ))
}

async fn handle_add(app: &AppState, path: &Path, name: Option<&str>) -> CliResult<String> {
    // Verify directory exists
    if !path.exists() {
        return Err(CliError::user(format!(
            "Directory not found: {}",
            path.display()
        )));
    }

    if !path.is_dir() {
        return Err(CliError::user(format!(
            "Not a directory: {}",
            path.display()
        )));
    }

    // Add to database
    let now = chrono::Utc::now().timestamp();
    let path_str = path.to_string_lossy();

    sqlx::query(
        r#"
        INSERT INTO garden_paths (path, preset_name, enabled, added_at)
        VALUES (?, ?, 1, ?)
        ON CONFLICT(path) DO UPDATE SET enabled = 1
        "#,
    )
    .bind(&*path_str)
    .bind(name)
    .bind(now)
    .execute(app.db.pool())
    .await
    .map_err(|e| CliError::system(format!("Database error: {}", e)))?;

    Ok(format!("✓ Added monitoring path: {}", path.display()))
}

async fn handle_list(app: &AppState) -> CliResult<String> {
    let rows: Vec<(i64, String, Option<String>, bool, i64)> = sqlx::query_as(
        r#"
        SELECT id, path, preset_name, enabled, added_at
        FROM garden_paths
        ORDER BY added_at DESC
        "#,
    )
    .fetch_all(app.db.pool())
    .await
    .map_err(|e| CliError::system(format!("Database error: {}", e)))?;

    if rows.is_empty() {
        return Ok("No monitoring paths registered.\n\nUse 'niwa garden init <preset>' or 'niwa garden add <path>' to register paths.".to_string());
    }

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL);
    table.set_header(vec!["ID", "Preset", "Path", "Status"]);

    for (id, path, preset_name, enabled, _added_at) in rows {
        table.add_row(vec![
            id.to_string(),
            preset_name.unwrap_or_else(|| "custom".to_string()),
            path,
            if enabled { "✓" } else { "✗" }.to_string(),
        ]);
    }

    Ok(table.to_string())
}

async fn handle_remove(app: &AppState, id: i64) -> CliResult<String> {
    let result = sqlx::query(
        r#"
        DELETE FROM garden_paths
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(app.db.pool())
    .await
    .map_err(|e| CliError::system(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        Err(CliError::user(format!("No monitoring path found with ID: {}", id)))
    } else {
        Ok(format!("✓ Removed monitoring path ID: {}", id))
    }
}

async fn handle_scan_registered(app: &AppState, scope: Scope, dry_run: bool, limit: Option<usize>, recent_days: Option<u64>) -> CliResult<String> {
    // Get all enabled paths
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT path
        FROM garden_paths
        WHERE enabled = 1
        "#,
    )
    .fetch_all(app.db.pool())
    .await
    .map_err(|e| CliError::system(format!("Database error: {}", e)))?;

    if rows.is_empty() {
        return Ok("No monitoring paths registered.\n\nUse 'niwa garden init <preset>' or 'niwa garden add <path>' to register paths.".to_string());
    }

    let mut all_results = Vec::new();

    for (path_str,) in rows {
        let path = PathBuf::from(&path_str);

        if !path.exists() {
            warn!("Skipping non-existent path: {}", path.display());
            continue;
        }

        match handle_scan(app, &path, scope, dry_run, limit, recent_days).await {
            Ok(result) => {
                all_results.push(format!("\n{}: {}\n{}", path.display(), "✓", result));
            }
            Err(e) => {
                warn!("Failed to scan {}: {}", path.display(), e);
                all_results.push(format!("\n{}: ✗ {}", path.display(), e));
            }
        }
    }

    let mut output = String::from("Garden Scan Results\n");
    output.push_str("===================\n");
    for result in all_results {
        output.push_str(&result);
        output.push('\n');
    }

    Ok(output)
}

async fn handle_scan(app: &AppState, directory: &Path, scope: Scope, dry_run: bool, limit: Option<usize>, recent_days: Option<u64>) -> CliResult<String> {
    // Verify directory exists
    if !directory.exists() {
        return Err(CliError::user(format!(
            "Directory not found: {}",
            directory.display()
        )));
    }

    if !directory.is_dir() {
        return Err(CliError::user(format!(
            "Not a directory: {}",
            directory.display()
        )));
    }

    info!("Scanning directory: {}", directory.display());

    // Scan for session log files
    let session_files = scan_session_files(directory)?;
    info!("Found {} potential session files", session_files.len());

    if session_files.is_empty() {
        return Ok("No session files found.".to_string());
    }

    // Filter by recent_days if specified
    let filtered_files = if let Some(days) = recent_days {
        let cutoff_time = std::time::SystemTime::now()
            - std::time::Duration::from_secs(days * 24 * 60 * 60);

        session_files.into_iter().filter(|path| {
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    return modified >= cutoff_time;
                }
            }
            false
        }).collect()
    } else {
        session_files
    };

    info!("After recent_days filter: {} files", filtered_files.len());

    // Filter out already processed files
    let mut unprocessed_files = Vec::new();
    for file_path in filtered_files {
        let hash = calculate_file_hash(&file_path)?;
        let is_processed = is_file_processed(&app.db.pool(), &file_path, &hash).await?;

        if !is_processed {
            unprocessed_files.push((file_path, hash));
        }
    }

    // Apply limit if specified
    if let Some(max_count) = limit {
        unprocessed_files.truncate(max_count);
    }

    info!(
        "Found {} unprocessed files (after filters)",
        unprocessed_files.len()
    );

    if unprocessed_files.is_empty() {
        return Ok("All session files have already been processed.".to_string());
    }

    if dry_run {
        let mut output = String::from("Dry run - would process:\n\n");
        for (file_path, _) in &unprocessed_files {
            output.push_str(&format!("  • {}\n", file_path.display()));
        }
        output.push_str(&format!("\nTotal: {} files", unprocessed_files.len()));
        return Ok(output);
    }

    // Process each unprocessed file
    let mut processed_count = 0;
    let mut failed_count = 0;
    let mut results = Vec::new();

    for (file_path, file_hash) in unprocessed_files {
        info!("Processing: {}", file_path.display());

        match process_session_file(app, &file_path, &file_hash, scope).await {
            Ok(expertise_id) => {
                processed_count += 1;
                results.push(format!("✓ {}: {}", file_path.display(), expertise_id));
            }
            Err(e) => {
                failed_count += 1;
                warn!("Failed to process {}: {}", file_path.display(), e);
                results.push(format!("✗ {}: {}", file_path.display(), e));
            }
        }
    }

    // Build summary
    let mut output = String::new();

    for result in results {
        output.push_str(&format!("{}\n", result));
    }

    output.push_str(&format!(
        "\nSummary: {} processed, {} failed, {} total",
        processed_count,
        failed_count,
        processed_count + failed_count
    ));

    Ok(output)
}

/// Scan directory recursively for session log files
fn scan_session_files(dir: &Path) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();

            // Filter by extension
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if matches!(ext_str.as_str(), "log" | "md" | "txt" | "jsonl") {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}

/// Calculate SHA256 hash of file content
fn calculate_file_hash(path: &Path) -> Result<String, CliError> {
    let content = std::fs::read(path)
        .map_err(|e| CliError::system(format!("Failed to read file: {}", e)))?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let hash = hasher.finalize();

    Ok(format!("{:x}", hash))
}

/// Check if file has already been processed
async fn is_file_processed(
    pool: &sqlx::SqlitePool,
    file_path: &Path,
    file_hash: &str,
) -> Result<bool, CliError> {
    let path_str = file_path.to_string_lossy();

    let row: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT file_hash
        FROM processed_sessions
        WHERE file_path = ?
        "#,
    )
    .bind(&*path_str)
    .fetch_optional(pool)
    .await
    .map_err(|e| CliError::system(format!("Database error: {}", e)))?;

    match row {
        Some((existing_hash,)) => {
            // Check if hash matches (file not modified)
            Ok(existing_hash == file_hash)
        }
        None => Ok(false),
    }
}

/// Process a session file and generate expertise
async fn process_session_file(
    app: &AppState,
    file_path: &Path,
    file_hash: &str,
    scope: Scope,
) -> Result<String, String> {
    // Read file content
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Generate expertise ID from file name
    let expertise_id = generate_expertise_id(file_path);

    debug!("Generated expertise ID: {}", expertise_id);

    // Generate expertise using LLM
    let expertise = app
        .generator
        .generate_from_log(&content, &expertise_id, scope)
        .await
        .map_err(|e| format!("Failed to generate expertise: {}", e))?;

    // Store in database
    app.db
        .storage()
        .create(expertise)
        .await
        .map_err(|e| format!("Failed to store expertise: {}", e))?;

    // Record as processed
    let path_str = file_path.to_string_lossy();
    let processed_at = chrono::Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT OR REPLACE INTO processed_sessions (file_path, file_hash, expertise_id, processed_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&*path_str)
    .bind(file_hash)
    .bind(&expertise_id)
    .bind(processed_at)
    .execute(app.db.pool())
    .await
    .map_err(|e| format!("Failed to record processed session: {}", e))?;

    Ok(expertise_id)
}

/// Generate expertise ID from file path
fn generate_expertise_id(path: &Path) -> String {
    // Use file stem (name without extension) as base
    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("session");

    // Sanitize: replace spaces and special chars with hyphens
    let sanitized = file_stem
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();

    // Remove consecutive hyphens
    let cleaned = sanitized
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    // Limit length
    if cleaned.len() > 50 {
        cleaned[..50].to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_expertise_id() {
        assert_eq!(
            generate_expertise_id(Path::new("session-2024-01-15.log")),
            "session-2024-01-15"
        );
        assert_eq!(
            generate_expertise_id(Path::new("My Session Log.txt")),
            "my-session-log"
        );
        assert_eq!(
            generate_expertise_id(Path::new("rust_async_patterns.md")),
            "rust-async-patterns"
        );
    }
}
