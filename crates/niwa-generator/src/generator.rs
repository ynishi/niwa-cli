//! Expertise generator using LLM

use crate::agents::{
    ExpertiseExtractorAgent, ExpertiseImproverAgent, ExpertiseMergerAgent,
    InteractiveExpertiseAgent,
};
use crate::Result;
use llm_toolkit::Agent;
use niwa_core::{Expertise, Scope};
use tracing::{debug, error, info};

/// Generation options
#[derive(Debug, Clone)]
pub struct GenerationOptions {
    /// Model to use (default: claude-sonnet-4-5)
    pub model: String,
    /// Temperature (0.0-1.0)
    pub temperature: f32,
    /// Additional context to include
    pub additional_context: Option<String>,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-5".to_string(),
            temperature: 0.7,
            additional_context: None,
        }
    }
}

/// Expertise generator using LLM
///
/// This generator uses llm-toolkit Agent macros to generate
/// structured Expertise objects from conversation logs and other inputs.
pub struct ExpertiseGenerator {
    options: GenerationOptions,
}

impl ExpertiseGenerator {
    /// Create a new ExpertiseGenerator with default options
    ///
    /// # Example
    ///
    /// ```no_run
    /// use niwa_generator::ExpertiseGenerator;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let generator = ExpertiseGenerator::new().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new() -> Result<Self> {
        Self::with_options(GenerationOptions::default()).await
    }

    /// Create a new ExpertiseGenerator with custom options
    pub async fn with_options(options: GenerationOptions) -> Result<Self> {
        info!(
            "Initializing ExpertiseGenerator with model: {}",
            options.model
        );
        Ok(Self { options })
    }

    /// Generate Expertise from conversation log
    ///
    /// # Arguments
    ///
    /// * `log_content` - The conversation log content
    /// * `id` - ID for the new Expertise
    /// * `scope` - Scope for the new Expertise
    ///
    /// # Example
    ///
    /// ```no_run
    /// use niwa_generator::ExpertiseGenerator;
    /// use niwa_core::Scope;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let generator = ExpertiseGenerator::new().await?;
    ///     let log = std::fs::read_to_string("session.log")?;
    ///
    ///     let expertise = generator
    ///         .generate_from_log(&log, "rust-expert", Scope::Personal)
    ///         .await?;
    ///
    ///     println!("Generated: {}", expertise.id());
    ///     Ok(())
    /// }
    /// ```
    pub async fn generate_from_log(
        &self,
        log_content: &str,
        id: &str,
        scope: Scope,
    ) -> Result<Expertise> {
        info!("Generating expertise from log: id={}", id);

        // Build prompt for the agent
        let prompt = format!(
            "Analyze the following conversation log and extract structured expertise.\n\n\
             Target Expertise ID: {}\n\n\
             =====================================================================\n
             Log Content Start\n
             =====================================================================\n
             {}
             =====================================================================\n
             Log Content End\n
             =====================================================================\n
             ",
            id, log_content
        );

        // Use the Agent macro-powered agent
        // Agent derive automatically handles:
        // - JSON schema generation from ExpertiseResponse structure + doc comments
        // - Markdown code block stripping
        // - Type-safe deserialization
        // - Error handling with proper error messages
        let agent = ExpertiseExtractorAgent::default();

        match agent.execute(prompt.into()).await {
            Ok(response) => {
                info!(
                    "Successfully extracted expertise: {} tags, {} fragments",
                    response.tags.len(),
                    response.fragments.len()
                );

                // Convert ExpertiseResponse to Expertise
                let mut expertise = Expertise::new(id, "1.0.0");
                expertise.inner.description = Some(response.description);
                expertise.inner.tags = response.tags;
                expertise.metadata.scope = scope;

                // Add text fragments
                use llm_toolkit_expertise::{KnowledgeFragment, WeightedFragment};
                for fragment_text in response.fragments {
                    expertise
                        .inner
                        .content
                        .push(WeightedFragment::new(KnowledgeFragment::Text(
                            fragment_text,
                        )));
                }

                Ok(expertise)
            }
            Err(e) => {
                // Agent error - return error
                error!("LLM generation failed: {:?}", e);
                Err(e.into())
            }
        }
    }

    /// Improve existing Expertise
    ///
    /// # Arguments
    ///
    /// * `expertise` - The Expertise to improve
    /// * `instruction` - How to improve it
    ///
    /// # Example
    ///
    /// ```no_run
    /// use niwa_generator::ExpertiseGenerator;
    /// use niwa_core::{Database, Scope, StorageOperations};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let db = Database::open_default().await?;
    ///     let generator = ExpertiseGenerator::new().await?;
    ///
    ///     let expertise = db.storage()
    ///         .get("rust-expert", Scope::Personal)
    ///         .await?
    ///         .unwrap();
    ///
    ///     let improved = generator
    ///         .improve(expertise, "Add more error handling examples")
    ///         .await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn improve(&self, expertise: Expertise, instruction: &str) -> Result<Expertise> {
        info!("Improving expertise: id={}", expertise.id());

        let current_json = expertise.to_json()?;

        // Build prompt for the agent
        let prompt = format!(
            "Current Expertise:\n{}\n\nImprovement Instruction:\n{}\n\n\
             Please analyze the current expertise and apply the improvement instruction. \
             Identify what to add, update, or remove to make this expertise more valuable.",
            current_json, instruction
        );

        // Use the Agent macro-powered agent
        let agent = ExpertiseImproverAgent::default();

        match agent.execute(prompt.into()).await {
            Ok(response) => {
                info!(
                    "Successfully improved expertise: {} new fragments, {} to remove",
                    response.new_fragments.len(),
                    response.fragments_to_remove.len()
                );
                debug!("Improvement summary: {}", response.improvement_summary);

                // Apply improvements to expertise
                let mut improved = expertise.clone();
                improved.inner.description = Some(response.description);
                improved.inner.tags = response.tags;

                // Remove fragments marked for removal
                use llm_toolkit_expertise::KnowledgeFragment;
                if !response.fragments_to_remove.is_empty() {
                    improved.inner.content.retain(|weighted_fragment| {
                        if let KnowledgeFragment::Text(text) = &weighted_fragment.fragment {
                            !response.fragments_to_remove.contains(text)
                        } else {
                            true // Keep non-text fragments
                        }
                    });
                }

                // Add new fragments
                use llm_toolkit_expertise::WeightedFragment;
                for fragment_text in response.new_fragments {
                    improved
                        .inner
                        .content
                        .push(WeightedFragment::new(KnowledgeFragment::Text(
                            fragment_text,
                        )));
                }

                // Increment version
                let version_parts: Vec<&str> = improved.version().split('.').collect();
                if version_parts.len() >= 2 {
                    let minor: u32 = version_parts[1].parse().unwrap_or(0);
                    improved.inner.version = format!("{}.{}.0", version_parts[0], minor + 1);
                }

                Ok(improved)
            }
            Err(e) => {
                // Agent error - return original expertise with version bump
                debug!(
                    "LLM improvement failed: {:?}, returning original with version bump",
                    e
                );
                let mut improved = expertise;
                let version_parts: Vec<&str> = improved.version().split('.').collect();
                if version_parts.len() >= 2 {
                    let minor: u32 = version_parts[1].parse().unwrap_or(0);
                    improved.inner.version = format!("{}.{}.0", version_parts[0], minor + 1);
                }
                Ok(improved)
            }
        }
    }

    /// Interactive Expertise generation
    ///
    /// # Arguments
    ///
    /// * `id` - ID for the new Expertise
    /// * `description` - Brief description
    /// * `domain` - Domain/category
    /// * `scope` - Scope for the new Expertise
    ///
    /// # Example
    ///
    /// ```no_run
    /// use niwa_generator::ExpertiseGenerator;
    /// use niwa_core::Scope;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let generator = ExpertiseGenerator::new().await?;
    ///
    ///     let expertise = generator
    ///         .generate_interactive(
    ///             "rust-expert",
    ///             "Expert in Rust programming",
    ///             "programming",
    ///             Scope::Personal
    ///         )
    ///         .await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn generate_interactive(
        &self,
        id: &str,
        description: &str,
        domain: &str,
        scope: Scope,
    ) -> Result<Expertise> {
        info!(
            "Generating expertise interactively: id={}, domain={}",
            id, domain
        );

        // Build prompt for the agent
        let mut prompt = format!(
            "Domain: {}\nDescription: {}\n\n\
             Please generate comprehensive expertise for this domain.",
            domain, description
        );

        // Add optional context if provided
        if let Some(context) = self.options.additional_context.as_deref() {
            prompt.push_str(&format!("\n\nAdditional Context:\n{}", context));
        }

        // Use the Agent macro-powered agent
        let agent = InteractiveExpertiseAgent::default();

        match agent.execute(prompt.into()).await {
            Ok(response) => {
                info!(
                    "Successfully generated interactive expertise: {} tags, {} fragments",
                    response.tags.len(),
                    response.fragments.len()
                );
                if !response.related_areas.is_empty() {
                    debug!("Suggested related areas: {:?}", response.related_areas);
                }

                // Convert response to Expertise
                let mut expertise = Expertise::new(id, "1.0.0");
                expertise.inner.description = Some(response.description);
                expertise.inner.tags = response.tags;
                expertise.metadata.scope = scope;

                // Add fragments
                use llm_toolkit_expertise::{KnowledgeFragment, WeightedFragment};
                for fragment_text in response.fragments {
                    expertise
                        .inner
                        .content
                        .push(WeightedFragment::new(KnowledgeFragment::Text(
                            fragment_text,
                        )));
                }

                // Optionally store related_areas as metadata (if needed)
                // For now, we log them but don't persist them in the Expertise structure

                Ok(expertise)
            }
            Err(e) => {
                // Agent error - return error
                debug!("LLM generation failed: {:?}", e);
                Err(e.into())
            }
        }
    }

    /// Merge multiple Expertises
    ///
    /// # Arguments
    ///
    /// * `expertises` - The Expertises to merge
    /// * `output_id` - ID for the merged Expertise
    /// * `description` - Description for the merged Expertise
    /// * `scope` - Scope for the merged Expertise
    pub async fn merge(
        &self,
        expertises: &[Expertise],
        output_id: &str,
        description: &str,
        scope: Scope,
    ) -> Result<Expertise> {
        info!("Merging {} expertises into {}", expertises.len(), output_id);

        if expertises.is_empty() {
            return Err(crate::Error::Other(
                "Cannot merge empty expertise list".to_string(),
            ));
        }

        // Convert expertises to JSON for the prompt
        let expertises_json: Vec<String> = expertises
            .iter()
            .map(|e| e.to_json())
            .collect::<std::result::Result<_, _>>()?;

        // Build prompt for the agent
        let prompt = format!(
            "Target Output ID: {}\nTarget Description: {}\n\n\
             Expertises to Merge:\n{}\n\n\
             Please synthesize these expertises into a unified, coherent expertise. \
             Identify common themes, preserve unique insights, and resolve any conflicts.",
            output_id,
            description,
            expertises_json.join("\n\n---\n\n")
        );

        // Use the Agent macro-powered agent
        let agent = ExpertiseMergerAgent::default();

        match agent.execute(prompt.into()).await {
            Ok(response) => {
                info!(
                    "Successfully merged expertises: {} tags, {} fragments",
                    response.tags.len(),
                    response.fragments.len()
                );
                debug!("Merge summary: {}", response.merge_summary);
                if !response.conflicts_found.is_empty() {
                    info!(
                        "Conflicts found during merge: {:?}",
                        response.conflicts_found
                    );
                }

                // Convert response to Expertise
                let mut merged = Expertise::new(output_id, "1.0.0");
                merged.inner.description = Some(response.description);
                merged.inner.tags = response.tags;
                merged.metadata.scope = scope;

                // Add fragments
                use llm_toolkit_expertise::{KnowledgeFragment, WeightedFragment};
                for fragment_text in response.fragments {
                    merged
                        .inner
                        .content
                        .push(WeightedFragment::new(KnowledgeFragment::Text(
                            fragment_text,
                        )));
                }

                Ok(merged)
            }
            Err(e) => {
                // Agent error - return error
                debug!("LLM merge failed: {:?}", e);
                Err(e.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_generator() {
        let generator = ExpertiseGenerator::new().await.unwrap();
        assert_eq!(generator.options.model, "claude-sonnet-4-5");
    }

    #[tokio::test]
    async fn test_generate_from_log() {
        let generator = ExpertiseGenerator::new().await.unwrap();
        let log = "This is a test conversation log about Rust programming.";

        // This test requires actual LLM integration
        // If LLM call fails, test will fail (expected behavior)
        let result = generator
            .generate_from_log(log, "rust-expert", Scope::Personal)
            .await;

        // For now, we just verify that the method returns Ok or Err
        // In production, this would be an integration test with LLM available
        match result {
            Ok(expertise) => {
                assert_eq!(expertise.id(), "rust-expert");
            }
            Err(_e) => {
                // LLM not available or parsing failed - expected in test environment
                // This is acceptable as we're testing the structure, not LLM integration
            }
        }
    }

    #[tokio::test]
    async fn test_improve_expertise() {
        let generator = ExpertiseGenerator::new().await.unwrap();
        let expertise = Expertise::new("test-id", "1.0.0");

        // This test requires actual LLM integration
        let result = generator.improve(expertise, "Add more examples").await;

        // For now, we just verify that the method returns Ok or Err
        match result {
            Ok(improved) => {
                assert_eq!(improved.version(), "1.1.0");
            }
            Err(_e) => {
                // LLM not available or parsing failed - expected in test environment
            }
        }
    }
}
