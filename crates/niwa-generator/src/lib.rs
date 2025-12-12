//! # niwa-generator
//!
//! LLM-powered Expertise generation for NIWA.
//!
//! ## Features
//!
//! - Generate Expertise from conversation logs using LLM
//! - Improve existing Expertise with LLM assistance
//! - Interactive Expertise creation
//! - Schema-based structured data generation
//!
//! ## Example
//!
//! ```no_run
//! use niwa_generator::ExpertiseGenerator;
//! use niwa_core::Scope;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let generator = ExpertiseGenerator::new().await?;
//!
//!     // Generate from log
//!     let log_content = std::fs::read_to_string("session.log")?;
//!     let expertise = generator
//!         .generate_from_log(&log_content, "rust-expert", Scope::Personal)
//!         .await?;
//!
//!     println!("Generated: {} v{}", expertise.id(), expertise.version());
//!
//!     Ok(())
//! }
//! ```

pub mod agents;
pub mod error;
pub mod generator;
pub mod session_log;

// Re-exports
pub use agents::{
    ExpertiseExtractorAgent, ExpertiseImprovementResponse, ExpertiseImproverAgent,
    ExpertiseLinkerAgent, ExpertiseMergerAgent, ExpertiseResponse, ExpertiseSummary,
    InteractiveExpertiseAgent, InteractiveExpertiseResponse, LinkerResponse,
    MergedExpertiseResponse, SuggestedLink,
};
pub use error::{Error, Result};
pub use generator::{ExpertiseGenerator, GenerationOptions, LlmProvider};
pub use session_log::SessionLogParser;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
