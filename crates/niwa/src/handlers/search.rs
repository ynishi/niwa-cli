//! Search command

use crate::state::AppState;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use niwa_core::SearchOptions;
use sen::{CliError, CliResult, State};

/// Search expertises
///
/// Usage:
///   niwa search "rust error handling"
///   niwa search "async" --limit 10
pub async fn search(state: State<AppState>) -> CliResult<String> {
    let args: Vec<String> = std::env::args().collect();

    // Get query (first non-flag argument after "search")
    let query = args
        .iter()
        .skip_while(|s| s.as_str() != "search")
        .skip(1)
        .find(|s| !s.starts_with('-'))
        .ok_or_else(|| CliError::user("Missing search query"))?;

    // Parse limit
    let limit = args
        .iter()
        .skip_while(|s| s.as_str() != "--limit" && s.as_str() != "-l")
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok());

    let mut options = SearchOptions::new();
    if let Some(limit) = limit {
        options = options.limit(limit);
    }

    let app = state.read().await;

    let results = app
        .db
        .query()
        .search(query, options)
        .await
        .map_err(|e| CliError::system(format!("Search failed: {}", e)))?;

    if results.is_empty() {
        return Ok(format!("No results found for: {}", query));
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
        query,
        table,
        results.len()
    ))
}
