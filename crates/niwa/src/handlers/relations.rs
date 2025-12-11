//! Relations commands

use crate::state::AppState;
use clap::Parser;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use niwa_core::{RelationType, Scope, StorageOperations};
use sen::{Args, CliError, CliResult, State};

/// Create a relation between two expertises
///
/// Usage:
///   niwa link rust-expert --to error-handling --type uses
///   niwa link rust-expert --to error-handling --scope personal
#[derive(Parser, Debug)]
pub struct LinkArgs {
    /// Source expertise ID
    pub from_id: String,

    /// Target expertise ID
    #[arg(short, long)]
    pub to: String,

    /// Relation type (uses, extends, conflicts, requires)
    #[arg(short = 't', long, default_value = "uses")]
    pub relation_type: RelationType,

    /// Scope (if not specified, searches all scopes)
    #[arg(short, long)]
    pub scope: Option<Scope>,

    /// Optional metadata (JSON)
    #[arg(short, long)]
    pub metadata: Option<String>,
}

#[sen::handler]
pub async fn link(state: State<AppState>, Args(args): Args<LinkArgs>) -> CliResult<String> {
    let app = state.read().await;

    // Verify source expertise exists
    let scopes_to_check = match args.scope {
        Some(s) => vec![s],
        None => vec![Scope::Personal, Scope::Company, Scope::Project],
    };

    let mut from_found = false;
    for scope in &scopes_to_check {
        if app
            .db
            .storage()
            .exists(&args.from_id, *scope)
            .await
            .map_err(|e| CliError::system(format!("Database error: {}", e)))?
        {
            from_found = true;
            break;
        }
    }

    if !from_found {
        return Err(CliError::user(format!(
            "Source expertise not found: {}",
            args.from_id
        )));
    }

    // Verify target expertise exists
    let mut to_found = false;
    for scope in &scopes_to_check {
        if app
            .db
            .storage()
            .exists(&args.to, *scope)
            .await
            .map_err(|e| CliError::system(format!("Database error: {}", e)))?
        {
            to_found = true;
            break;
        }
    }

    if !to_found {
        return Err(CliError::user(format!(
            "Target expertise not found: {}",
            args.to
        )));
    }

    // Create relation
    app.db
        .graph()
        .create_relation(&args.from_id, &args.to, args.relation_type, args.metadata)
        .await
        .map_err(|e| CliError::system(format!("Failed to create relation: {}", e)))?;

    Ok(format!(
        "✓ Created relation: {} -[{}]-> {}",
        args.from_id, args.relation_type, args.to
    ))
}

/// Show dependencies and relations
///
/// Usage:
///   niwa deps rust-expert
///   niwa deps rust-expert --incoming
///   niwa deps rust-expert --all
///   niwa deps rust-expert --scope personal
#[derive(Parser, Debug)]
pub struct DepsArgs {
    /// Expertise ID
    pub id: String,

    /// Show incoming relations (dependents)
    #[arg(short, long)]
    pub incoming: bool,

    /// Show all relations (both incoming and outgoing)
    #[arg(short, long)]
    pub all: bool,

    /// Scope (if not specified, searches all scopes)
    #[arg(short, long)]
    pub scope: Option<Scope>,
}

#[sen::handler]
pub async fn deps(state: State<AppState>, Args(args): Args<DepsArgs>) -> CliResult<String> {
    let app = state.read().await;

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
            .exists(&args.id, scope)
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
            args.id
        )));
    }

    // Get relations based on flags
    let relations = if args.all {
        app.db
            .graph()
            .get_all_relations(&args.id)
            .await
            .map_err(|e| CliError::system(format!("Failed to get relations: {}", e)))?
    } else if args.incoming {
        app.db
            .graph()
            .get_incoming(&args.id)
            .await
            .map_err(|e| CliError::system(format!("Failed to get incoming relations: {}", e)))?
    } else {
        app.db
            .graph()
            .get_outgoing(&args.id)
            .await
            .map_err(|e| CliError::system(format!("Failed to get outgoing relations: {}", e)))?
    };

    if relations.is_empty() {
        let direction = if args.all {
            "any"
        } else if args.incoming {
            "incoming"
        } else {
            "outgoing"
        };
        return Ok(format!("No {} relations found for: {}", direction, args.id));
    }

    // Build table
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    // Header
    table.set_header(vec![
        Cell::new("Direction").fg(Color::Cyan),
        Cell::new("Expertise").fg(Color::Cyan),
        Cell::new("Type").fg(Color::Cyan),
        Cell::new("Metadata").fg(Color::Cyan),
    ]);

    // Rows
    for relation in &relations {
        let (direction, expertise_id) = if relation.from_id == args.id {
            ("→", relation.to_id.as_str())
        } else {
            ("←", relation.from_id.as_str())
        };

        let metadata = relation
            .metadata
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("-");

        table.add_row(vec![
            Cell::new(direction),
            Cell::new(expertise_id),
            Cell::new(relation.relation_type.to_string()),
            Cell::new(metadata),
        ]);
    }

    let title = if args.all {
        "All Relations"
    } else if args.incoming {
        "Incoming Relations (Dependents)"
    } else {
        "Outgoing Relations (Dependencies)"
    };

    Ok(format!(
        "\n{}: {}\n\n{}\n\nTotal: {} relations",
        title,
        args.id,
        table,
        relations.len()
    ))
}
