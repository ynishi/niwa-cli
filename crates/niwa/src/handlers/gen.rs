//! Generation commands

use crate::state::AppState;
use clap::Parser;
use niwa_core::{Scope, StorageOperations};
use sen::{Args, CliError, CliResult, State};
use std::path::PathBuf;

/// Generate Expertise from log file or text
///
/// Usage:
///   niwa gen --file session.log --id rust-expert --scope personal
///   niwa gen --text "Some knowledge..." --id quick-tip
#[derive(Parser, Debug)]
pub struct GenArgs {
    /// Log file path to generate expertise from
    #[arg(short = 'f', long, conflicts_with = "text")]
    pub file: Option<PathBuf>,

    /// Direct text input (alternative to --file)
    #[arg(short = 't', long, conflicts_with = "file")]
    pub text: Option<String>,

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
    // Get content from file or text
    let log_content = if let Some(file_path) = args.file {
        std::fs::read_to_string(&file_path)
            .map_err(|e| CliError::user(format!("Failed to read log file: {}", e)))?
    } else if let Some(text) = args.text {
        text
    } else {
        return Err(CliError::user(
            "Either --file or --text must be provided".to_string(),
        ));
    };

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
