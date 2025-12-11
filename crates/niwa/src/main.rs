//! NIWA CLI - Expertise Graph Management
//!
//! A command-line tool for managing AI expertise graphs.

mod handlers;
mod state;

use handlers::{gen, list, search, show};
use sen::Router;
use state::AppState;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Initialize application state
    let state = match AppState::new().await {
        Ok(state) => state,
        Err(e) => {
            eprintln!("Failed to initialize NIWA: {}", e);
            std::process::exit(1);
        }
    };

    // Build router
    let router = Router::new()
        // Generation commands
        .route("gen", gen::generate)
        .route("improve", gen::improve)

        // Query commands
        .route("list", list::list)
        .route("show", show::show)
        .route("search", search::search)
        .route("tags", list::tags)

        .with_state(state)
        .with_agent_mode(); // JSON output for LLM integration

    // Execute
    let response = router.execute().await;

    // Output
    if response.agent_mode {
        println!("{}", response.to_agent_json());
    } else {
        if !response.output.is_empty() {
            println!("{}", response.output);
        }
    }

    std::process::exit(response.exit_code);
}
