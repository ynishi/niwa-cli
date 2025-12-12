//! Agent definitions for niwa-generator
//!
//! This module contains Agent trait implementations using llm-toolkit's Agent derive macro.
//! Agents are kept in a separate module to avoid conflicts with the Result<T> type alias.

use llm_toolkit::{agent, type_marker, ToPrompt};
use serde::{Deserialize, Serialize};

/// Structured response for Expertise generation from LLM
///
/// This structure represents the LLM's output when analyzing conversation logs
/// or other inputs to extract expertise. Each field provides guidance to the LLM
/// about what information to extract and how to structure it.
#[type_marker]
#[derive(Serialize, Deserialize, Debug, Clone, ToPrompt)]
#[prompt(mode = "full")]
pub struct ExpertiseResponse {
    /// Suggested ID for this expertise (lowercase, hyphenated, 3-5 words describing the core topic)
    /// Examples: "rust-async-patterns", "react-state-management", "git-workflow-best-practices"
    pub suggested_id: String,

    /// Brief description of the expertise (1-2 sentences summarizing the core knowledge)
    pub description: String,

    /// Tags for categorization (e.g., "rust", "async", "error-handling")
    pub tags: Vec<String>,

    /// List of key knowledge fragments extracted from the content.
    /// Each fragment should be a self-contained insight, best practice, or important concept.
    pub fragments: Vec<String>,
}

/// Agent for extracting structured expertise from conversation logs
#[agent(
    expertise = r#"You are an expert at extracting DOMAIN-SPECIFIC KNOWLEDGE from development conversation logs.

Your task is to identify and extract knowledge that would be valuable for future development work.

## EXTRACT (High Priority)
- **Domain concepts** unique to this project (e.g., "bi-temporal data model with systemDate and validDate")
- **Project-specific patterns** and their rationale (e.g., "why Authority controls Member visibility")
- **API behaviors** or undocumented quirks discovered during development
- **Bug patterns** and root causes (what failed, why, how it was fixed)
- **Architecture decisions** and trade-offs made
- **Integration patterns** with external services or APIs
- **Data model relationships** and constraints

## DO NOT EXTRACT
- Generic tool usage (how to use grep, git, IDE features)
- System prompt contents or AI operational guidelines (e.g., "I operate in read-only mode")
- Common programming patterns available in public documentation
- Session setup, greetings, or initialization messages
- General best practices that any developer would know

## Output Requirements
1. Generate a meaningful suggested_id (lowercase, hyphenated, 3-5 words) that captures the DOMAIN topic
   - Good: "yesod-bitemporal-member-delta", "google-connector-pagination-handling"
   - Bad: "session-123", "read-only-mode", "code-exploration"
2. Extract a description focusing on the PROJECT-SPECIFIC knowledge
3. Identify 3-5 domain-relevant tags
4. Extract 5-10 knowledge fragments that:
   - Would NOT be in LLM training data (project-specific, recent, internal)
   - Represent decisions/learnings from actual implementation work
   - Help understand "WHY" not just "WHAT"

If the conversation contains only generic tool usage or system prompts without domain knowledge, return minimal fragments focusing on any project context mentioned.

Output a single, valid JSON object with the structure defined by the `ExpertiseResponse` type."#,
    output = "ExpertiseResponse",
    backend = "claude"
)]
pub struct ExpertiseExtractorAgent;

// ============================================================================
// Expertise Improvement
// ============================================================================

/// Response for improving existing Expertise
///
/// This structure represents the LLM's analysis and improvements to an existing
/// Expertise based on user instructions. It identifies what to add, update, or remove.
#[type_marker]
#[derive(Serialize, Deserialize, Debug, Clone, ToPrompt)]
#[prompt(mode = "full")]
pub struct ExpertiseImprovementResponse {
    /// Updated description (enhanced based on improvement instruction)
    /// Should be 1-2 sentences capturing the refined core knowledge
    pub description: String,

    /// Updated or additional tags for better categorization
    /// Use lowercase, hyphenated format (e.g., "rust", "async", "error-handling")
    pub tags: Vec<String>,

    /// New or enhanced knowledge fragments to add
    /// Each should be a self-contained insight that adds value to the expertise
    pub new_fragments: Vec<String>,

    /// Fragments to remove by matching content
    /// List exact fragment texts that are outdated, redundant, or incorrect
    pub fragments_to_remove: Vec<String>,

    /// Explanation of what was improved and why
    /// Should summarize the key changes and their rationale
    pub improvement_summary: String,
}

/// Agent for refining and improving existing Expertise
#[agent(
    expertise = r#"You are an expert at refining and improving existing expertise.

Your task is to:
1. Analyze the current Expertise (description, tags, fragments)
2. Apply the user's improvement instruction carefully
3. Enhance the description if needed (keep it concise, 1-2 sentences)
4. Add/update tags for better categorization
5. Add new valuable fragments that address the improvement instruction
6. Identify outdated, redundant, or incorrect fragments to remove
7. Provide a clear summary of improvements made

Guidelines:
- Be conservative: only change what needs improvement
- Maintain consistency with the existing expertise's domain and scope
- Ensure new fragments are concrete, actionable, and valuable
- Remove only fragments that are clearly outdated or redundant
- Explain your reasoning in the improvement_summary

Focus on making the expertise more accurate, comprehensive, and valuable."#,
    output = "ExpertiseImprovementResponse"
)]
pub struct ExpertiseImproverAgent;

// ============================================================================
// Interactive Expertise Generation
// ============================================================================

/// Response for interactive Expertise generation
///
/// This structure represents the LLM's creation of new expertise from high-level
/// user requirements (domain, description, optional context).
#[type_marker]
#[derive(Serialize, Deserialize, Debug, Clone, ToPrompt)]
#[prompt(mode = "full")]
pub struct InteractiveExpertiseResponse {
    /// Detailed description of the expertise (2-3 sentences)
    /// Should clearly explain what this expertise covers and its purpose
    pub description: String,

    /// Domain-specific tags for categorization
    /// Use lowercase, hyphenated format. Include 5-7 relevant tags
    pub tags: Vec<String>,

    /// Core knowledge fragments for this domain
    /// Should include 8-15 diverse fragments covering key concepts, best practices, and common pitfalls
    pub fragments: Vec<String>,

    /// Suggested related expertise areas for future expansion
    /// List 3-5 adjacent or complementary domains that would enhance this expertise
    pub related_areas: Vec<String>,
}

/// Agent for generating structured expertise from high-level requirements
#[agent(
    expertise = r#"You are an expert at generating structured expertise from high-level requirements.

Your task is to:
1. Analyze the provided domain, description, and any additional context
2. Generate a comprehensive description (2-3 sentences) of what this expertise covers
3. Identify 5-7 relevant tags appropriate for the domain
4. Generate 8-15 core knowledge fragments covering:
   - Key concepts and fundamental principles
   - Best practices and common patterns
   - Common pitfalls and how to avoid them
   - Tool/library recommendations if applicable
   - Performance considerations if relevant
5. Suggest 3-5 related areas for future expertise expansion

Guidelines:
- Make fragments concrete and actionable
- Cover breadth first, then depth
- Include both positive guidance (what to do) and negative guidance (what to avoid)
- Ensure fragments are self-contained and understandable independently
- Suggest related areas that are adjacent or complementary

Create well-rounded, practical expertise that would be valuable for someone learning or working in this domain."#,
    output = "InteractiveExpertiseResponse"
)]
pub struct InteractiveExpertiseAgent;

// ============================================================================
// Expertise Merging
// ============================================================================

/// Response for merging multiple Expertises
///
/// This structure represents the LLM's synthesis of multiple expertise sources
/// into a unified, coherent expertise.
#[type_marker]
#[derive(Serialize, Deserialize, Debug, Clone, ToPrompt)]
#[prompt(mode = "full")]
pub struct MergedExpertiseResponse {
    /// Unified description that captures all merged expertise
    /// Should be 2-3 sentences summarizing the combined knowledge domain
    pub description: String,

    /// Consolidated tags from all sources (deduplicated and prioritized)
    /// Use lowercase, hyphenated format. Include 5-10 most relevant tags
    pub tags: Vec<String>,

    /// Synthesized knowledge fragments (merged, deduplicated, organized)
    /// Should preserve unique insights while removing redundancy. Aim for 10-20 fragments
    pub fragments: Vec<String>,

    /// Summary of how the expertises were merged and what themes emerged
    /// Explain the synthesis process and key patterns identified
    pub merge_summary: String,

    /// Conflicts or contradictions found during merge (if any)
    /// List any cases where sources provided conflicting information
    pub conflicts_found: Vec<String>,
}

/// Agent for synthesizing multiple knowledge sources into unified expertise
#[agent(
    expertise = r#"You are an expert at synthesizing multiple knowledge sources into unified expertise.

Your task is to:
1. Analyze all provided Expertises (descriptions, tags, fragments)
2. Identify common themes, overlapping concepts, and unique insights
3. Create a unified description that captures the essence of all inputs (2-3 sentences)
4. Consolidate tags by:
   - Deduplicating similar tags
   - Prioritizing most relevant tags
   - Including 5-10 tags total
5. Synthesize knowledge fragments by:
   - Merging similar or overlapping fragments
   - Preserving unique insights from each source
   - Organizing by logical themes or categories
   - Removing redundancy while maintaining completeness
   - Aim for 10-20 high-quality fragments
6. Identify any contradictions or conflicts between sources
7. Provide a clear summary of the merge process

Guidelines:
- The result should be coherent and well-organized
- Preserve the most valuable insights from each source
- Resolve conflicts when possible, or note them explicitly
- Organize fragments logically (e.g., by topic, by abstraction level)
- Ensure the merged expertise is greater than the sum of its parts

Focus on creating a comprehensive, unified knowledge base that synthesizes all inputs effectively."#,
    output = "MergedExpertiseResponse"
)]
pub struct ExpertiseMergerAgent;

// ============================================================================
// Expertise Linking
// ============================================================================

/// Summary of an expertise for linking analysis
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExpertiseSummary {
    pub id: String,
    pub description: String,
    pub tags: Vec<String>,
}

/// A suggested link between two expertises
#[derive(Serialize, Deserialize, Debug, Clone, ToPrompt)]
#[prompt(mode = "full")]
pub struct SuggestedLink {
    /// Source expertise ID
    pub from_id: String,
    /// Target expertise ID
    pub to_id: String,
    /// Relation type: "uses", "extends", "requires", or "conflicts"
    pub relation_type: String,
    /// Brief explanation of why this link makes sense
    pub reason: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
}

/// Response for expertise linking analysis
///
/// This structure represents the LLM's analysis of potential relationships
/// between a new expertise and existing expertises in the knowledge graph.
#[type_marker]
#[derive(Serialize, Deserialize, Debug, Clone, ToPrompt)]
#[prompt(mode = "full")]
pub struct LinkerResponse {
    /// List of suggested links to create
    /// Only include links with high confidence (>= 0.7)
    pub suggested_links: Vec<SuggestedLink>,
}

/// Agent for analyzing and suggesting links between expertises
#[agent(
    expertise = r#"You are an expert at analyzing knowledge relationships and suggesting meaningful links between expertise items.

Your task is to:
1. Analyze the NEW expertise (id, description, tags)
2. Compare it with EXISTING expertises in the knowledge graph
3. Identify meaningful relationships based on:
   - Semantic similarity in descriptions
   - Overlapping or related domains
   - Complementary knowledge areas
   - Dependency relationships (one builds on another)

Relation types to use:
- "uses": The new expertise uses/applies concepts from the existing one
- "extends": The new expertise extends/expands on the existing one
- "requires": The new expertise requires understanding of the existing one
- "conflicts": The expertises have conflicting information (use sparingly)

Guidelines:
- Only suggest links with HIGH confidence (>= 0.7)
- Prefer quality over quantity - fewer strong links are better than many weak ones
- Consider both directions: new→existing and existing→new
- Provide clear, concise reasons for each suggested link
- Don't link expertises that are merely tangentially related
- Focus on actionable, meaningful relationships

Output a JSON object with suggested_links array. If no strong links exist, return an empty array."#,
    output = "LinkerResponse",
    backend = "claude"
)]
pub struct ExpertiseLinkerAgent;
