//! List commands

use crate::state::AppState;
use clap::Parser;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use niwa_core::{Scope, StorageOperations};
use sen::{Args, CliError, CliResult, State};

/// List all expertises
///
/// Usage:
///   niwa list
///   niwa list --scope personal
#[derive(Parser, Debug)]
pub struct ListArgs {
    /// Filter by scope (personal, team, company)
    #[arg(short, long)]
    pub scope: Option<Scope>,
}

#[sen::handler]
pub async fn list(state: State<AppState>, Args(args): Args<ListArgs>) -> CliResult<String> {
    let app = state.read().await;

    let expertises = if let Some(scope) = args.scope {
        app.db.storage().list(scope).await
    } else {
        app.db.storage().list_all().await
    }
    .map_err(|e| CliError::system(format!("Failed to list expertises: {}", e)))?;

    if expertises.is_empty() {
        return Ok("No expertises found.".to_string());
    }

    // Build table
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("ID").fg(Color::Green),
            Cell::new("Version").fg(Color::Green),
            Cell::new("Scope").fg(Color::Green),
            Cell::new("Tags").fg(Color::Green),
            Cell::new("Description").fg(Color::Green),
        ]);

    for exp in &expertises {
        let tags = exp.tags().join(", ");
        let description = exp.description();
        let truncated_desc = if description.len() > 50 {
            format!("{}...", &description[..50])
        } else {
            description
        };

        table.add_row(vec![
            exp.id(),
            exp.version(),
            &exp.metadata.scope.to_string(),
            &tags,
            &truncated_desc,
        ]);
    }

    Ok(format!(
        "\n{}\n\nTotal: {} expertises",
        table,
        expertises.len()
    ))
}

/// List all tags
///
/// Usage:
///   niwa tags
pub async fn tags(state: State<AppState>) -> CliResult<String> {
    let app = state.read().await;

    let tags = app
        .db
        .query()
        .list_tags(None)
        .await
        .map_err(|e| CliError::system(format!("Failed to list tags: {}", e)))?;

    if tags.is_empty() {
        return Ok("No tags found.".to_string());
    }

    // Build table
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Tag").fg(Color::Cyan),
            Cell::new("Count").fg(Color::Cyan),
        ]);

    for (tag, count) in tags {
        table.add_row(vec![tag, count.to_string()]);
    }

    Ok(format!("\n{}", table))
}
