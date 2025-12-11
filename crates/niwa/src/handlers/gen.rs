//! Generation commands

use crate::state::AppState;
use niwa_core::{Scope, StorageOperations};
use sen::{CliError, CliResult, State};
use std::path::PathBuf;

/// Generate Expertise from log file
///
/// Usage:
///   niwa gen --file session.log --id rust-expert --scope personal
pub async fn generate(state: State<AppState>) -> CliResult<String> {
    // Parse arguments (simplified for now)
    let args: Vec<String> = std::env::args().collect();

    // Extract parameters from args
    let (file_path, id, scope) = parse_gen_args(&args)?;

    // Read log file
    let log_content = std::fs::read_to_string(&file_path)
        .map_err(|e| CliError::user(format!("Failed to read log file: {}", e)))?;

    // Generate expertise
    let app = state.read().await;
    let expertise = app.generator
        .generate_from_log(&log_content, &id, scope)
        .await
        .map_err(|e| CliError::system(format!("Failed to generate expertise: {}", e)))?;

    // Store in database
    app.db.storage()
        .create(expertise.clone())
        .await
        .map_err(|e| CliError::system(format!("Failed to store expertise: {}", e)))?;

    Ok(format!(
        "✓ Generated expertise: {} v{}\n  Scope: {}\n  Description: {}",
        expertise.id(),
        expertise.version(),
        scope,
        expertise.description()
    ))
}

/// Improve existing Expertise
///
/// Usage:
///   niwa improve rust-expert --instruction "Add error handling examples"
pub async fn improve(state: State<AppState>) -> CliResult<String> {
    let args: Vec<String> = std::env::args().collect();

    let (id, instruction) = parse_improve_args(&args)?;

    let app = state.read().await;

    // Get existing expertise
    let expertise = app.db.storage()
        .get(&id, Scope::Personal) // TODO: Support other scopes
        .await
        .map_err(|e| CliError::system(format!("Database error: {}", e)))?
        .ok_or_else(|| CliError::user(format!("Expertise not found: {}", id)))?;

    // Improve it
    let improved = app.generator
        .improve(expertise, &instruction)
        .await
        .map_err(|e| CliError::system(format!("Failed to improve expertise: {}", e)))?;

    // Update in database
    app.db.storage()
        .update(improved.clone())
        .await
        .map_err(|e| CliError::system(format!("Failed to update expertise: {}", e)))?;

    Ok(format!(
        "✓ Improved expertise: {} → v{}",
        improved.id(),
        improved.version()
    ))
}

// Helper functions

fn parse_gen_args(args: &[String]) -> Result<(PathBuf, String, Scope), CliError> {
    let mut file: Option<PathBuf> = None;
    let mut id: Option<String> = None;
    let mut scope = Scope::Personal;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--file" | "-f" => {
                if i + 1 < args.len() {
                    file = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--id" => {
                if i + 1 < args.len() {
                    id = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--scope" | "-s" => {
                if i + 1 < args.len() {
                    scope = Scope::from_str(&args[i + 1])
                        .map_err(|_| CliError::user(format!("Invalid scope: {}", args[i + 1])))?;
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let file = file.ok_or_else(|| CliError::user("Missing required argument: --file"))?;
    let id = id.ok_or_else(|| CliError::user("Missing required argument: --id"))?;

    Ok((file, id, scope))
}

fn parse_improve_args(args: &[String]) -> Result<(String, String), CliError> {
    // Find the ID (first non-flag argument after "improve")
    let id = args.iter()
        .skip_while(|s| s.as_str() != "improve")
        .skip(1)
        .find(|s| !s.starts_with('-'))
        .ok_or_else(|| CliError::user("Missing expertise ID"))?
        .clone();

    // Find instruction
    let instruction = args.iter()
        .skip_while(|s| s.as_str() != "--instruction" && s.as_str() != "-i").nth(1)
        .ok_or_else(|| CliError::user("Missing required argument: --instruction"))?
        .clone();

    Ok((id, instruction))
}
