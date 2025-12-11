//! Search command

use crate::state::AppState;
use clap::Parser;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use niwa_core::SearchOptions;
use sen::{Args, CliResult, State};

/// Search expertises
///
/// Usage:
///   niwa search "rust error handling"
///   niwa search "async" --limit 10
#[derive(Parser, Debug)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Maximum number of results
    #[arg(short, long)]
    pub limit: Option<usize>,
}

#[sen::handler]
pub async fn search(state: State<AppState>, Args(args): Args<SearchArgs>) -> CliResult<String> {
    let mut options = SearchOptions::new();
    if let Some(limit) = args.limit {
        options = options.limit(limit);
    }

    let app = state.read().await;

    let results = app
        .db
        .query()
        .search(&args.query, options)
        .await
        .map_err(|e| sen::CliError::system(format!("Search failed: {}", e)))?;

    if results.is_empty() {
        return Ok(format!("No results found for: {}", args.query));
    }

    // Build table
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("ID").fg(Color::Yellow),
            Cell::new("Version").fg(Color::Yellow),
            Cell::new("Tags").fg(Color::Yellow),
            Cell::new("Description").fg(Color::Yellow),
        ]);

    for exp in &results {
        let tags = exp.tags().join(", ");
        let description = exp.description();
        let truncated_desc = if description.len() > 60 {
            format!("{}...", &description[..60])
        } else {
            description
        };

        table.add_row(vec![exp.id(), exp.version(), &tags, &truncated_desc]);
    }

    Ok(format!(
        "\nSearch: \"{}\"\n\n{}\n\nFound: {} results",
        args.query,
        table,
        results.len()
    ))
}
