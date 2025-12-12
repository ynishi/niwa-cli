//! Show command

use crate::state::AppState;
use clap::Parser;
use niwa_core::{KnowledgeFragment, Scope, StorageOperations};
use sen::{Args, CliResult, State};

/// Show detailed information about an Expertise
///
/// Usage:
///   niwa show rust-expert
///   niwa show rust-expert --scope company
///   niwa show rust-expert --fragments
#[derive(Parser, Debug)]
pub struct ShowArgs {
    /// Expertise ID to display
    pub id: String,

    /// Scope (personal, team, company). If not specified, searches all scopes.
    #[arg(short, long)]
    pub scope: Option<Scope>,

    /// Show fragment contents
    #[arg(short, long)]
    pub fragments: bool,
}

#[sen::handler]
pub async fn show(state: State<AppState>, Args(args): Args<ShowArgs>) -> CliResult<String> {
    let app = state.read().await;

    // If scope is specified, search only that scope
    // Otherwise, search all scopes in order: personal, team, company
    let expertise = if let Some(scope) = args.scope {
        app.db
            .storage()
            .get(&args.id, scope)
            .await
            .map_err(|e| sen::CliError::system(format!("Database error: {}", e)))?
    } else {
        // Search all scopes
        let mut found = None;
        for scope in [Scope::Personal, Scope::Project, Scope::Company] {
            if let Some(exp) = app
                .db
                .storage()
                .get(&args.id, scope)
                .await
                .map_err(|e| sen::CliError::system(format!("Database error: {}", e)))?
            {
                found = Some(exp);
                break;
            }
        }
        found
    };

    let expertise = expertise.ok_or_else(|| {
        if let Some(scope) = args.scope {
            sen::CliError::user(format!(
                "Expertise not found: {} (scope: {})",
                args.id, scope
            ))
        } else {
            sen::CliError::user(format!("Expertise not found: {} (in any scope)", args.id))
        }
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

    // Show fragments if requested
    if args.fragments && !expertise.inner.content.is_empty() {
        output.push_str("\n────────────────────────────────────────\n");
        output.push_str("  Fragments\n");
        output.push_str("────────────────────────────────────────\n\n");

        for (i, weighted_fragment) in expertise.inner.content.iter().enumerate() {
            let content = match &weighted_fragment.fragment {
                KnowledgeFragment::Text(text) => text.clone(),
                KnowledgeFragment::Logic { instruction, steps } => {
                    let mut s = format!("[Logic] {}", instruction);
                    if !steps.is_empty() {
                        s.push_str("\nSteps: ");
                        s.push_str(&steps.join(" → "));
                    }
                    s
                }
                KnowledgeFragment::Guideline { rule, anchors: _ } => {
                    format!("[Guideline] {}", rule)
                }
                KnowledgeFragment::QualityStandard {
                    criteria,
                    passing_grade,
                } => {
                    format!(
                        "[QualityStandard] Pass: {} | Criteria: {}",
                        passing_grade,
                        criteria.join(", ")
                    )
                }
                KnowledgeFragment::ToolDefinition(value) => {
                    format!(
                        "[ToolDefinition] {}",
                        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
                    )
                }
            };

            output.push_str(&format!("#{} ", i + 1));

            // Truncate long content for display
            let display_content = if content.len() > 500 {
                format!("{}...", &content[..500])
            } else {
                content
            };
            output.push_str(&display_content);
            output.push_str("\n\n");
        }
    }

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(output)
}

fn format_timestamp(ts: i64) -> String {
    use chrono::{DateTime, Utc};
    let dt = DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}
