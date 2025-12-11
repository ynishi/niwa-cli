//! Show command

use crate::state::AppState;
use clap::Parser;
use niwa_core::{Scope, StorageOperations};
use sen::{Args, CliResult, State};

/// Show detailed information about an Expertise
///
/// Usage:
///   niwa show rust-expert
///   niwa show rust-expert --scope company
#[derive(Parser, Debug)]
pub struct ShowArgs {
    /// Expertise ID to display
    pub id: String,

    /// Scope (personal, team, company)
    #[arg(short, long, default_value = "personal")]
    pub scope: Scope,
}

#[sen::handler]
pub async fn show(state: State<AppState>, Args(args): Args<ShowArgs>) -> CliResult<String> {
    let app = state.read().await;

    let expertise = app
        .db
        .storage()
        .get(&args.id, args.scope)
        .await
        .map_err(|e| sen::CliError::system(format!("Database error: {}", e)))?
        .ok_or_else(|| {
            sen::CliError::user(format!(
                "Expertise not found: {} (scope: {})",
                args.id, args.scope
            ))
        })?;

    // Format output
    let mut output = String::new();
    output.push_str("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str(&format!("  Expertise: {}\n", expertise.id()));
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    output.push_str(&format!("Version:     {}\n", expertise.version()));
    output.push_str(&format!("Scope:       {}\n", expertise.metadata.scope));
    output.push_str(&format!(
        "Created:     {}\n",
        format_timestamp(expertise.metadata.created_at)
    ));
    output.push_str(&format!(
        "Updated:     {}\n",
        format_timestamp(expertise.metadata.updated_at)
    ));

    if !expertise.tags().is_empty() {
        output.push_str(&format!("\nTags:        {}\n", expertise.tags().join(", ")));
    }

    output.push_str(&format!("\nDescription:\n{}\n", expertise.description()));

    output.push_str(&format!(
        "\nFragments:   {} total\n",
        expertise.inner.content.len()
    ));

    output.push_str("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(output)
}

fn format_timestamp(ts: i64) -> String {
    use chrono::{DateTime, Utc};
    let dt = DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}
