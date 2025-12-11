//! Application state

use niwa_core::Database;
use niwa_generator::ExpertiseGenerator;
use std::sync::Arc;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Database connection
    pub db: Arc<Database>,
    /// LLM-powered generator
    pub generator: Arc<ExpertiseGenerator>,
}

impl AppState {
    /// Create a new AppState
    pub async fn new() -> anyhow::Result<Self> {
        // Open database
        let db = Database::open_default().await?;

        // Create generator
        let generator = ExpertiseGenerator::new().await?;

        Ok(Self {
            db: Arc::new(db),
            generator: Arc::new(generator),
        })
    }
}
