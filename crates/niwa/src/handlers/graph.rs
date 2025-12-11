//! Graph visualization commands

use crate::state::AppState;
use clap::Parser;
use niwa_core::{Scope, StorageOperations};
use sen::{Args, CliError, CliResult, State};
use std::collections::{HashMap, HashSet};

/// Display expertise dependency graph
///
/// Usage:
///   niwa graph                    # Show all expertises and relations
///   niwa graph rust-expert        # Show subgraph centered on rust-expert
///   niwa graph --scope personal   # Filter by scope
#[derive(Parser, Debug)]
pub struct GraphArgs {
    /// Optional expertise ID to center the graph on
    pub id: Option<String>,

    /// Scope filter (if not specified, shows all scopes)
    #[arg(short, long)]
    pub scope: Option<Scope>,

    /// Maximum depth for subgraph (default: 2)
    #[arg(short, long, default_value = "2")]
    pub depth: usize,
}

#[sen::handler]
pub async fn graph(state: State<AppState>, Args(args): Args<GraphArgs>) -> CliResult<String> {
    let app = state.read().await;

    // Get all expertises
    let expertises = if let Some(scope) = args.scope {
        app.db
            .storage()
            .list(scope)
            .await
            .map_err(|e| CliError::system(format!("Failed to list expertises: {}", e)))?
    } else {
        app.db
            .storage()
            .list_all()
            .await
            .map_err(|e| CliError::system(format!("Failed to list expertises: {}", e)))?
    };

    if expertises.is_empty() {
        return Ok("No expertises found.".to_string());
    }

    // Get all relations
    let mut all_relations = Vec::new();
    for exp in &expertises {
        let relations = app
            .db
            .graph()
            .get_outgoing(exp.id())
            .await
            .map_err(|e| CliError::system(format!("Failed to get relations: {}", e)))?;
        all_relations.extend(relations);
    }

    if all_relations.is_empty() {
        return Ok(format!(
            "Found {} expertises but no relations.\nUse 'niwa link' to create relations.",
            expertises.len()
        ));
    }

    // Build graph output
    let output = if let Some(center_id) = args.id {
        // Verify expertise exists
        let scopes_to_check = match args.scope {
            Some(s) => vec![s],
            None => vec![Scope::Personal, Scope::Company, Scope::Project],
        };

        let mut found = false;
        for scope in scopes_to_check {
            if app
                .db
                .storage()
                .exists(&center_id, scope)
                .await
                .map_err(|e| CliError::system(format!("Database error: {}", e)))?
            {
                found = true;
                break;
            }
        }

        if !found {
            return Err(CliError::user(format!(
                "Expertise not found: {}",
                center_id
            )));
        }

        build_subgraph(&center_id, &all_relations, args.depth)
    } else {
        build_full_graph(&expertises, &all_relations)
    };

    Ok(output)
}

/// Build a full graph visualization
fn build_full_graph(
    expertises: &[niwa_core::Expertise],
    relations: &[niwa_core::graph::Relation],
) -> String {
    let mut output = String::new();
    output.push_str("Expertise Dependency Graph\n");
    output.push_str("==========================\n\n");

    // Group relations by source
    let mut relations_by_source: HashMap<String, Vec<&niwa_core::graph::Relation>> = HashMap::new();
    for relation in relations {
        relations_by_source
            .entry(relation.from_id.clone())
            .or_default()
            .push(relation);
    }

    // Find root nodes (no incoming edges)
    let all_targets: HashSet<String> = relations.iter().map(|r| r.to_id.clone()).collect();
    let all_sources: HashSet<String> = relations.iter().map(|r| r.from_id.clone()).collect();
    let roots: Vec<String> = all_sources
        .difference(&all_targets)
        .cloned()
        .collect::<Vec<_>>();

    // Display roots first
    let mut displayed = HashSet::new();
    for root in &roots {
        display_node(root, &relations_by_source, &mut displayed, &mut output, 0);
    }

    // Display remaining nodes (cycles or disconnected)
    for exp in expertises {
        let id = exp.id();
        if !displayed.contains(id) && relations_by_source.contains_key(id) {
            display_node(id, &relations_by_source, &mut displayed, &mut output, 0);
        }
    }

    // Display isolated nodes
    let isolated: Vec<&niwa_core::Expertise> = expertises
        .iter()
        .filter(|e| {
            let id = e.id();
            !all_sources.contains(id) && !all_targets.contains(id)
        })
        .collect();

    if !isolated.is_empty() {
        output.push_str("\nIsolated Expertises (no relations):\n");
        for exp in isolated {
            output.push_str(&format!("  • {}\n", exp.id()));
        }
    }

    output.push_str(&format!(
        "\nTotal: {} expertises, {} relations",
        expertises.len(),
        relations.len()
    ));
    output
}

/// Build a subgraph centered on a specific node
fn build_subgraph(
    center_id: &str,
    relations: &[niwa_core::graph::Relation],
    max_depth: usize,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("Subgraph centered on: {}\n", center_id));
    output.push_str("==========================\n\n");

    // Group relations by source
    let mut relations_by_source: HashMap<String, Vec<&niwa_core::graph::Relation>> = HashMap::new();
    for relation in relations {
        relations_by_source
            .entry(relation.from_id.clone())
            .or_default()
            .push(relation);
    }

    let mut displayed = HashSet::new();
    display_node_with_depth(
        center_id,
        &relations_by_source,
        &mut displayed,
        &mut output,
        0,
        max_depth,
    );

    output
}

/// Display a node and its children recursively
fn display_node(
    id: &str,
    relations_by_source: &HashMap<String, Vec<&niwa_core::graph::Relation>>,
    displayed: &mut HashSet<String>,
    output: &mut String,
    indent: usize,
) {
    if displayed.contains(id) {
        return;
    }

    displayed.insert(id.to_string());

    // Display current node
    let indent_str = "  ".repeat(indent);
    output.push_str(&format!("{}{}\n", indent_str, id));

    // Display children
    if let Some(children) = relations_by_source.get(id) {
        let child_count = children.len();
        for (i, relation) in children.iter().enumerate() {
            let is_last = i == child_count - 1;
            let connector = if is_last { "└─" } else { "├─" };
            let child_indent_str = "  ".repeat(indent + 1);

            output.push_str(&format!(
                "{}{}[{}]→ {}\n",
                child_indent_str, connector, relation.relation_type, relation.to_id
            ));

            // Recursively display child's children
            if !displayed.contains(&relation.to_id) {
                display_node(
                    &relation.to_id,
                    relations_by_source,
                    displayed,
                    output,
                    indent + 2,
                );
            }
        }
    }
}

/// Display a node with depth limit
fn display_node_with_depth(
    id: &str,
    relations_by_source: &HashMap<String, Vec<&niwa_core::graph::Relation>>,
    displayed: &mut HashSet<String>,
    output: &mut String,
    indent: usize,
    max_depth: usize,
) {
    if indent > max_depth || displayed.contains(id) {
        return;
    }

    displayed.insert(id.to_string());

    // Display current node
    let indent_str = "  ".repeat(indent);
    output.push_str(&format!("{}{}\n", indent_str, id));

    // Display children
    if let Some(children) = relations_by_source.get(id) {
        let child_count = children.len();
        for (i, relation) in children.iter().enumerate() {
            let is_last = i == child_count - 1;
            let connector = if is_last { "└─" } else { "├─" };
            let child_indent_str = "  ".repeat(indent + 1);

            output.push_str(&format!(
                "{}{}[{}]→ {}\n",
                child_indent_str, connector, relation.relation_type, relation.to_id
            ));

            // Recursively display child's children with depth limit
            if indent + 2 <= max_depth {
                display_node_with_depth(
                    &relation.to_id,
                    relations_by_source,
                    displayed,
                    output,
                    indent + 2,
                    max_depth,
                );
            }
        }
    }
}
