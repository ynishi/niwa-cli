//! Application state

use niwa_core::Database;
use niwa_generator::{ExpertiseGenerator, GenerationOptions, LlmProvider};
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

        // Create generator with provider from environment variable
        let provider = Self::get_llm_provider_from_env();
        let generator = if provider != LlmProvider::Claude {
            tracing::info!("Using LLM provider: {:?}", provider);
            let options = GenerationOptions {
                provider,
                ..Default::default()
            };
            ExpertiseGenerator::with_options(options).await?
        } else {
            ExpertiseGenerator::new().await?
        };

        Ok(Self {
            db: Arc::new(db),
            generator: Arc::new(generator),
        })
    }

    /// Get LLM provider from environment variable NIWA_LLM_PROVIDER
    /// Supported values: claude, gemini, codex
    /// Default: claude
    fn get_llm_provider_from_env() -> LlmProvider {
        match std::env::var("NIWA_LLM_PROVIDER") {
            Ok(val) => match val.to_lowercase().as_str() {
                "gemini" => LlmProvider::Gemini,
                "codex" | "openai" => LlmProvider::Codex,
                "claude" => LlmProvider::Claude,
                _ => {
                    tracing::warn!(
                        "Unknown NIWA_LLM_PROVIDER value: '{}'. Using default (claude)",
                        val
                    );
                    LlmProvider::Claude
                }
            },
            Err(_) => LlmProvider::Claude, // Default
        }
    }
}
