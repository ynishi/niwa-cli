//! Generation commands

use crate::state::AppState;
use clap::Parser;
use niwa_core::{Scope, StorageOperations};
use sen::{Args, CliError, CliResult, State};
use std::path::PathBuf;

/// Generate Expertise from log file
///
/// Usage:
///   niwa gen --file session.log --id rust-expert --scope personal
#[derive(Parser, Debug)]
pub struct GenArgs {
    /// Log file path to generate expertise from
    #[arg(short = 'f', long)]
    pub file: PathBuf,

    /// Expertise ID
    #[arg(long)]
    pub id: String,

    /// Scope (personal, team, company)
    #[arg(short, long, default_value = "personal")]
    pub scope: Scope,
}

#[sen::handler]
pub async fn generate(
    state: State<AppState>,
    Args(args): Args<GenArgs>,
) -> CliResult<String> {
    // Read log file
    let log_content = std::fs::read_to_string(&args.file)
        .map_err(|e| CliError::user(format!("Failed to read log file: {}", e)))?;

    // Generate expertise
    let app = state.read().await;
    let expertise = app
        .generator
        .generate_from_log(&log_content, &args.id, args.scope)
        .await
        .map_err(|e| CliError::system(format!("Failed to generate expertise: {}", e)))?;

    // Store in database
    app.db
        .storage()
        .create(expertise.clone())
        .await
        .map_err(|e| CliError::system(format!("Failed to store expertise: {}", e)))?;

    Ok(format!(
        "✓ Generated expertise: {} v{}\n  Scope: {}\n  Description: {}",
        expertise.id(),
        expertise.version(),
        args.scope,
        expertise.description()
    ))
}

/// Improve existing Expertise
///
/// Usage:
///   niwa improve rust-expert --instruction "Add error handling examples" --scope personal
#[derive(Parser, Debug)]
pub struct ImproveArgs {
    /// Expertise ID to improve
    pub id: String,

    /// Improvement instruction
    #[arg(short, long)]
    pub instruction: String,

    /// Scope (personal, team, company)
    #[arg(short, long, default_value = "personal")]
    pub scope: Scope,
}

#[sen::handler]
pub async fn improve(
    state: State<AppState>,
    Args(args): Args<ImproveArgs>,
) -> CliResult<String> {
    let app = state.read().await;

    // Get existing expertise
    let expertise = app
        .db
        .storage()
        .get(&args.id, args.scope)
        .await
        .map_err(|e| CliError::system(format!("Database error: {}", e)))?
        .ok_or_else(|| {
            CliError::user(format!(
                "Expertise not found: {} (scope: {})",
                args.id, args.scope
            ))
        })?;

    // Improve it
    let improved = app
        .generator
        .improve(expertise, &args.instruction)
        .await
        .map_err(|e| CliError::system(format!("Failed to improve expertise: {}", e)))?;

    // Update in database
    app.db
        .storage()
        .update(improved.clone())
        .await
        .map_err(|e| CliError::system(format!("Failed to update expertise: {}", e)))?;

    Ok(format!(
        "✓ Improved expertise: {} → v{}",
        improved.id(),
        improved.version()
    ))
}
