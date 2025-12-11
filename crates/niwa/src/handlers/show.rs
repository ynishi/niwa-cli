//! Show command

use crate::state::AppState;
use niwa_core::{Scope, StorageOperations};
use sen::{CliError, CliResult, State};

/// Show detailed information about an Expertise
///
/// Usage:
///   niwa show rust-expert
///   niwa show rust-expert --scope company
pub async fn show(state: State<AppState>) -> CliResult<String> {
    let args: Vec<String> = std::env::args().collect();

    // Get ID (first non-flag argument after "show")
    let id = args.iter()
        .skip_while(|s| s.as_str() != "show")
        .skip(1)
        .find(|s| !s.starts_with('-'))
        .ok_or_else(|| CliError::user("Missing expertise ID"))?;

    // Get scope
    let scope = args.iter()
        .skip_while(|s| s.as_str() != "--scope" && s.as_str() != "-s").nth(1)
        .and_then(|s| Scope::from_str(s).ok())
        .unwrap_or(Scope::Personal);

    let app = state.read().await;

    let expertise = app.db.storage()
        .get(id, scope)
        .await
        .map_err(|e| CliError::system(format!("Database error: {}", e)))?
        .ok_or_else(|| CliError::user(format!("Expertise not found: {} (scope: {})", id, scope)))?;

    // Format output
    let mut output = String::new();
    output.push_str("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str(&format!("  Expertise: {}\n", expertise.id()));
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    output.push_str(&format!("Version:     {}\n", expertise.version()));
    output.push_str(&format!("Scope:       {}\n", expertise.metadata.scope));
    output.push_str(&format!("Created:     {}\n", format_timestamp(expertise.metadata.created_at)));
    output.push_str(&format!("Updated:     {}\n", format_timestamp(expertise.metadata.updated_at)));

    if !expertise.tags().is_empty() {
        output.push_str(&format!("\nTags:        {}\n", expertise.tags().join(", ")));
    }

    output.push_str(&format!("\nDescription:\n{}\n", expertise.description()));

    output.push_str(&format!("\nFragments:   {} total\n", expertise.inner.content.len()));

    output.push_str("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(output)
}

fn format_timestamp(ts: i64) -> String {
    use chrono::{DateTime, Utc};
    let dt = DateTime::<Utc>::from_timestamp(ts, 0)
        .unwrap_or_else(Utc::now);
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}
