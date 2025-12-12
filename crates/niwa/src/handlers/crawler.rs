//! Crawler commands - automatic expertise extraction from session logs

use crate::state::AppState;
use clap::{Parser, Subcommand};
use comfy_table::{presets, Table};
use niwa_core::{RelationType, Scope, StorageOperations};
use sen::{Args, CliError, CliResult, State};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Automatically extract expertise from session logs
#[derive(Parser, Debug)]
pub struct CrawlerArgs {
    #[command(subcommand)]
    pub command: Option<CrawlerCommand>,
}

#[derive(Subcommand, Debug)]
pub enum CrawlerCommand {
    /// Scan and extract expertise from session logs
    Run {
        /// Directory to scan
        #[arg(value_name = "DIRECTORY")]
        directory: Option<PathBuf>,

        /// Scope for generated expertises (default: personal)
        #[arg(short, long, default_value = "personal")]
        scope: Scope,

        /// Dry run - show what would be processed without actually processing
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Maximum number of files to process
        #[arg(short, long)]
        limit: Option<usize>,

        /// Only process files modified in the last N days
        #[arg(long)]
        recent_days: Option<u64>,

        /// Automatically link new expertises to existing ones based on shared tags
        #[arg(long)]
        auto_link: bool,

        /// Automatically detect scope from file path using scope mappings
        /// (overrides --scope when a matching pattern is found)
        #[arg(long)]
        auto_scope: bool,
    },
    /// Initialize crawler with preset paths (claude-code, cursor)
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
    /// Manage scope mappings for automatic scope detection
    Scope {
        #[command(subcommand)]
        command: ScopeCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum ScopeCommand {
    /// Add a scope mapping pattern
    Add {
        /// Pattern to match (e.g., "projects/company-*", "work/*")
        pattern: String,
        /// Scope to assign (personal, company, project)
        #[arg(short, long)]
        scope: Scope,
        /// Priority (higher = checked first, default: 10)
        #[arg(short, long, default_value = "10")]
        priority: i32,
    },
    /// List all scope mappings
    List,
    /// Remove a scope mapping
    Remove {
        /// Mapping ID to remove
        id: i64,
    },
}

#[derive(Debug)]
pub enum CrawlerPreset {
    ClaudeCode,
    Cursor,
}

impl CrawlerPreset {
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
pub async fn crawler(
    state: State<AppState>,
    Args(args): Args<CrawlerArgs>,
) -> CliResult<String> {
    let app = state.read().await;

    match args.command {
        Some(CrawlerCommand::Run {
            directory,
            scope,
            dry_run,
            limit,
            recent_days,
            auto_link,
            auto_scope,
        }) => {
            // Scan mode
            if let Some(dir) = directory {
                // Explicit directory specified
                handle_scan(
                    &app, &dir, scope, dry_run, limit, recent_days, auto_link, auto_scope,
                )
                .await
            } else {
                // Scan all registered paths
                handle_scan_registered(
                    &app, scope, dry_run, limit, recent_days, auto_link, auto_scope,
                )
                .await
            }
        }
        Some(CrawlerCommand::Init { preset }) => handle_init(&app, &preset).await,
        Some(CrawlerCommand::Add { path, name }) => {
            handle_add(&app, &path, name.as_deref()).await
        }
        Some(CrawlerCommand::List) => handle_list(&app).await,
        Some(CrawlerCommand::Remove { id }) => handle_remove(&app, id).await,
        Some(CrawlerCommand::Scope { command }) => handle_scope(&app, command).await,
        None => {
            // Show help when no subcommand is provided
            Err(CliError::user(
                "No subcommand specified. Use 'crawler --help' to see available commands.",
            ))
        }
    }
}

async fn handle_init(app: &AppState, preset_name: &str) -> CliResult<String> {
    let preset = CrawlerPreset::from_str(preset_name)
        .map_err(|e| CliError::user(format!("{}\n\nAvailable presets: claude-code, cursor", e)))?;

    let path = preset.get_path().map_err(CliError::system)?;

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
        "✓ Initialized {} crawler monitoring\n  Path: {}",
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
        return Ok("No monitoring paths registered.\n\nUse 'niwa crawler init <preset>' or 'niwa crawler add <path>' to register paths.".to_string());
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
        Err(CliError::user(format!(
            "No monitoring path found with ID: {}",
            id
        )))
    } else {
        Ok(format!("✓ Removed monitoring path ID: {}", id))
    }
}

async fn handle_scan_registered(
    app: &AppState,
    default_scope: Scope,
    dry_run: bool,
    limit: Option<usize>,
    recent_days: Option<u64>,
    auto_link: bool,
    auto_scope: bool,
) -> CliResult<String> {
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
        return Ok("No monitoring paths registered.\n\nUse 'niwa crawler init <preset>' or 'niwa crawler add <path>' to register paths.".to_string());
    }

    let mut all_results = Vec::new();

    for (path_str,) in rows {
        let path = PathBuf::from(&path_str);

        if !path.exists() {
            warn!("Skipping non-existent path: {}", path.display());
            continue;
        }

        match handle_scan(
            app,
            &path,
            default_scope,
            dry_run,
            limit,
            recent_days,
            auto_link,
            auto_scope,
        )
        .await
        {
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

async fn handle_scan(
    app: &AppState,
    directory: &Path,
    default_scope: Scope,
    dry_run: bool,
    limit: Option<usize>,
    recent_days: Option<u64>,
    auto_link: bool,
    auto_scope: bool,
) -> CliResult<String> {
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
        let cutoff_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(days * 24 * 60 * 60);

        session_files
            .into_iter()
            .filter(|path| {
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(modified) = metadata.modified() {
                        return modified >= cutoff_time;
                    }
                }
                false
            })
            .collect()
    } else {
        session_files
    };

    info!("After recent_days filter: {} files", filtered_files.len());

    // Filter out already processed files and files without meaningful content
    const MIN_MESSAGES: usize = 3;
    const MIN_CHARS: usize = 200;

    let mut unprocessed_files = Vec::new();
    let mut skipped_trivial = 0;

    for file_path in filtered_files {
        // First check if the file has meaningful content (fast filter)
        if !has_meaningful_content(&file_path, MIN_MESSAGES, MIN_CHARS) {
            skipped_trivial += 1;
            continue;
        }

        let hash = calculate_file_hash(&file_path)?;
        let is_processed = is_file_processed(app.db.pool(), &file_path, &hash).await?;

        if !is_processed {
            unprocessed_files.push((file_path, hash));
        }
    }

    if skipped_trivial > 0 {
        info!(
            "Skipped {} trivial sessions (< {} messages or < {} chars)",
            skipped_trivial, MIN_MESSAGES, MIN_CHARS
        );
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
    let mut new_expertise_ids = Vec::new();
    let mut scopes_used: std::collections::HashSet<Scope> = std::collections::HashSet::new();

    for (file_path, file_hash) in unprocessed_files {
        info!("Processing: {}", file_path.display());

        // Determine scope for this file
        let file_scope = if auto_scope {
            resolve_scope_from_path(app.db.pool(), &file_path)
                .await
                .unwrap_or(default_scope)
        } else {
            default_scope
        };
        scopes_used.insert(file_scope);

        match process_session_file(app, &file_path, &file_hash, file_scope).await {
            Ok(expertise_id) => {
                processed_count += 1;
                let scope_indicator = if auto_scope && file_scope != default_scope {
                    format!(" [{}]", file_scope)
                } else {
                    String::new()
                };
                results.push(format!(
                    "✓ {}: {}{}",
                    file_path.display(),
                    expertise_id,
                    scope_indicator
                ));
                new_expertise_ids.push((expertise_id, file_scope));
            }
            Err(e) => {
                failed_count += 1;
                warn!("Failed to process {}: {}", file_path.display(), e);
                results.push(format!("✗ {}: {}", file_path.display(), e));
            }
        }
    }

    // Auto-link new expertises based on shared tags (per scope)
    let mut link_count = 0;
    if auto_link && !new_expertise_ids.is_empty() {
        info!("Auto-linking {} new expertises", new_expertise_ids.len());

        // Group by scope and link within each scope
        for scope in scopes_used {
            let scope_ids: Vec<String> = new_expertise_ids
                .iter()
                .filter(|(_, s)| *s == scope)
                .map(|(id, _)| id.clone())
                .collect();

            if scope_ids.is_empty() {
                continue;
            }

            match auto_link_expertises(app, &scope_ids, scope).await {
                Ok(count) => {
                    link_count += count;
                    if count > 0 {
                        results.push(format!(
                            "\n🔗 Auto-linked: {} relations created (scope: {})",
                            count, scope
                        ));
                    }
                }
                Err(e) => {
                    warn!("Auto-link failed for scope {}: {}", scope, e);
                    results.push(format!("\n⚠ Auto-link failed ({}): {}", scope, e));
                }
            }
        }
    }

    // Build summary
    let mut output = String::new();

    for result in results {
        output.push_str(&format!("{}\n", result));
    }

    let mut summary = format!(
        "\nSummary: {} processed, {} failed, {} total",
        processed_count,
        failed_count,
        processed_count + failed_count
    );
    if auto_link && link_count > 0 {
        summary.push_str(&format!(", {} links", link_count));
    }
    output.push_str(&summary);

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
    let content =
        std::fs::read(path).map_err(|e| CliError::system(format!("Failed to read file: {}", e)))?;

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
    let content =
        std::fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Generate fallback expertise ID from file name (used if LLM doesn't provide a good one)
    let fallback_id = generate_expertise_id(file_path);

    debug!("Fallback expertise ID: {}", fallback_id);

    // Generate expertise using LLM (LLM may suggest a better ID based on content)
    let expertise = app
        .generator
        .generate_from_log(&content, &fallback_id, scope)
        .await
        .map_err(|e| format!("Failed to generate expertise: {}", e))?;

    // Get the actual ID (may be LLM-suggested or fallback)
    let expertise_id = expertise.id().to_string();

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

/// Auto-link new expertises to existing ones using LLM-powered LinkerAgent
async fn auto_link_expertises(
    app: &AppState,
    new_ids: &[String],
    scope: Scope,
) -> Result<usize, String> {
    let storage = app.db.storage();
    let graph = app.db.graph();
    let mut link_count = 0;

    // Get all existing expertises for comparison
    let all_expertises = storage
        .list(scope)
        .await
        .map_err(|e| format!("Failed to list expertises: {}", e))?;

    if all_expertises.len() <= 1 {
        return Ok(0); // Need at least 2 expertises to link
    }

    // For each new expertise, use LinkerAgent to suggest links
    for new_id in new_ids {
        // Get the new expertise
        let new_expertise = match storage.get(new_id, scope).await {
            Ok(Some(e)) => e,
            _ => continue,
        };

        // Use LinkerAgent to analyze and suggest links
        let suggested_links = app
            .generator
            .suggest_links(&new_expertise, &all_expertises)
            .await
            .unwrap_or_default();

        // Create suggested relations
        for link in suggested_links {
            // Parse relation type
            let relation_type = match link.relation_type.to_lowercase().as_str() {
                "uses" => RelationType::Uses,
                "extends" => RelationType::Extends,
                "requires" => RelationType::Requires,
                "conflicts" => RelationType::Conflicts,
                _ => RelationType::Uses, // Default to Uses
            };

            // Check if relation already exists
            let existing_relations = graph
                .get_all_relations(&link.from_id)
                .await
                .unwrap_or_default();

            let already_linked = existing_relations
                .iter()
                .any(|r| r.to_id == link.to_id || r.from_id == link.to_id);

            if !already_linked {
                // Create relation with reason as metadata
                if let Ok(()) = graph
                    .create_relation(
                        &link.from_id,
                        &link.to_id,
                        relation_type,
                        Some(link.reason.clone()),
                    )
                    .await
                {
                    info!(
                        "Auto-linked {} -[{}]-> {} (confidence: {:.2}, reason: {})",
                        link.from_id, relation_type, link.to_id, link.confidence, link.reason
                    );
                    link_count += 1;
                }
            }
        }
    }

    Ok(link_count)
}

// ============================================================================
// Scope Mapping Handlers
// ============================================================================

async fn handle_scope(app: &AppState, command: ScopeCommand) -> CliResult<String> {
    match command {
        ScopeCommand::Add {
            pattern,
            scope,
            priority,
        } => handle_scope_add(app, &pattern, scope, priority).await,
        ScopeCommand::List => handle_scope_list(app).await,
        ScopeCommand::Remove { id } => handle_scope_remove(app, id).await,
    }
}

async fn handle_scope_add(
    app: &AppState,
    pattern: &str,
    scope: Scope,
    priority: i32,
) -> CliResult<String> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT INTO scope_mappings (pattern, scope, priority, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(pattern) DO UPDATE SET
            scope = excluded.scope,
            priority = excluded.priority,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(pattern)
    .bind(scope.as_str())
    .bind(priority)
    .bind(now)
    .bind(now)
    .execute(app.db.pool())
    .await
    .map_err(|e| CliError::system(format!("Failed to add scope mapping: {}", e)))?;

    Ok(format!(
        "✓ Added scope mapping: '{}' → {} (priority: {})",
        pattern, scope, priority
    ))
}

async fn handle_scope_list(app: &AppState) -> CliResult<String> {
    let rows: Vec<(i64, String, String, i32)> = sqlx::query_as(
        r#"
        SELECT id, pattern, scope, priority
        FROM scope_mappings
        ORDER BY priority DESC, id ASC
        "#,
    )
    .fetch_all(app.db.pool())
    .await
    .map_err(|e| CliError::system(format!("Failed to list scope mappings: {}", e)))?;

    if rows.is_empty() {
        return Ok("No scope mappings configured.\n\nUse 'niwa crawler scope add <pattern> --scope <scope>' to add mappings.".to_string());
    }

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);
    table.set_header(vec!["ID", "Pattern", "Scope", "Priority"]);

    for (id, pattern, scope, priority) in rows {
        table.add_row(vec![
            id.to_string(),
            pattern,
            scope,
            priority.to_string(),
        ]);
    }

    Ok(format!("Scope Mappings\n{}", table))
}

async fn handle_scope_remove(app: &AppState, id: i64) -> CliResult<String> {
    let result = sqlx::query("DELETE FROM scope_mappings WHERE id = ?")
        .bind(id)
        .execute(app.db.pool())
        .await
        .map_err(|e| CliError::system(format!("Failed to remove scope mapping: {}", e)))?;

    if result.rows_affected() == 0 {
        Err(CliError::user(format!(
            "No scope mapping found with ID: {}",
            id
        )))
    } else {
        Ok(format!("✓ Removed scope mapping ID: {}", id))
    }
}

/// Resolve scope from a file path using scope mappings
pub async fn resolve_scope_from_path(pool: &sqlx::SqlitePool, path: &Path) -> Option<Scope> {
    let path_str = path.to_string_lossy();

    // Get all mappings ordered by priority (highest first)
    let rows: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT pattern, scope
        FROM scope_mappings
        ORDER BY priority DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .ok()?;

    for (pattern, scope_str) in rows {
        if matches_pattern(&path_str, &pattern) {
            return scope_str.parse().ok();
        }
    }

    None // No match found
}

/// Check if a Claude JSONL session file has meaningful content
///
/// Returns true if the session has:
/// - At least `min_messages` user/assistant messages combined
/// - At least `min_chars` total characters in message content
///
/// This filters out empty agent initialization logs and trivial sessions.
fn has_meaningful_content(path: &Path, min_messages: usize, min_chars: usize) -> bool {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let reader = std::io::BufReader::new(file);
    let mut message_count = 0;
    let mut total_chars = 0;

    for line in std::io::BufRead::lines(reader) {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Parse JSON line
        let json: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Check message type (user or assistant)
        let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if msg_type == "user" || msg_type == "assistant" {
            message_count += 1;

            // Extract content from message
            if let Some(message) = json.get("message") {
                // Handle Claude API format: message.content array
                if let Some(content_array) = message.get("content").and_then(|c| c.as_array()) {
                    for content_item in content_array {
                        if let Some(text) = content_item.get("text").and_then(|t| t.as_str()) {
                            total_chars += text.len();
                        }
                    }
                }
                // Handle simple string content
                else if let Some(content_str) = message.get("content").and_then(|c| c.as_str()) {
                    total_chars += content_str.len();
                }
                // Handle direct message as string (user messages)
                else if let Some(msg_str) = message.as_str() {
                    total_chars += msg_str.len();
                }
            }
        }

        // Early exit if we've already met the criteria
        if message_count >= min_messages && total_chars >= min_chars {
            return true;
        }
    }

    message_count >= min_messages && total_chars >= min_chars
}

/// Match a path against a glob-like pattern
/// Supports:
/// - `*` matches any sequence of characters (except /)
/// - `**` matches any sequence including /
/// - `[...]` character classes (e.g., `[0-9]`, `[a-z]`)
/// - Literal text matches exactly
fn matches_pattern(path: &str, pattern: &str) -> bool {
    // Extract and preserve character classes [...] before escaping
    let mut result = String::new();
    let mut chars = pattern.chars().peekable();
    let mut char_classes: Vec<String> = Vec::new();

    while let Some(c) = chars.next() {
        if c == '[' {
            // Collect the entire character class
            let mut class = String::from("[");
            while let Some(&next) = chars.peek() {
                chars.next();
                class.push(next);
                if next == ']' {
                    break;
                }
            }
            // Replace with placeholder (use unique marker)
            result.push_str(&format!("__CHARCLASS{}__", char_classes.len()));
            char_classes.push(class);
        } else {
            result.push(c);
        }
    }

    // Simple glob matching
    let pattern = result.replace("**", "__DOUBLESTAR__");
    let pattern = pattern.replace('*', "[^/]*");
    let pattern = pattern.replace("__DOUBLESTAR__", ".*");

    // Escape other regex chars
    let mut pattern = regex::escape(&pattern)
        .replace(r"\[\^/\]\*", "[^/]*")
        .replace(r"\.\*", ".*");

    // Restore character classes (after escaping, the placeholder becomes escaped)
    for (i, class) in char_classes.iter().enumerate() {
        pattern = pattern.replace(&format!("__CHARCLASS{}__", i), class);
    }

    // Match anywhere in the path
    let regex_pattern = format!("(?i){}", pattern); // Case-insensitive

    regex::Regex::new(&regex_pattern)
        .map(|re| re.is_match(path))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern() {
        // Simple wildcard
        assert!(matches_pattern("/Users/test/projects/company-foo/file", "company-*"));
        assert!(matches_pattern("/Users/test/projects/niwa-cli/src", "niwa-*"));

        // Double wildcard
        assert!(matches_pattern("/Users/test/work/client/project/file", "work/**"));

        // Exact match
        assert!(matches_pattern("/Users/test/projects/niwa", "niwa"));

        // Character classes
        assert!(matches_pattern("/Users/test/projects/y1/file", "y[0-9]*"));
        assert!(matches_pattern("/Users/test/projects/y23/file", "y[0-9]*"));
        assert!(matches_pattern("/Users/test/projects/y100/file", "y[0-9]*"));
        assert!(!matches_pattern("/Users/test/projects/yui/file", "y[0-9]*"));
        assert!(!matches_pattern("/Users/test/projects/ya/file", "y[0-9]*"));

        // No match
        assert!(!matches_pattern("/Users/test/personal/stuff", "company-*"));
    }

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
