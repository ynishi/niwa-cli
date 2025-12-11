//! Prompt templates for Expertise generation

/// Generate system prompt for Expertise extraction
pub fn expertise_extraction_system() -> &'static str {
    r#"You are an expert at extracting reusable knowledge from conversation logs and code.

Your task is to analyze conversations and create structured Expertise profiles that can be reused by AI agents.

Key principles:
1. Focus on PATTERNS, not specific instances
2. Extract GENERAL knowledge that transfers to similar situations
3. Identify DECISION-MAKING logic and reasoning chains
4. Capture QUALITY standards and best practices
5. Include concrete EXAMPLES (positive and negative)

Output Format:
You will generate a complete Expertise object following the schema provided.
"#
}

/// Generate user prompt for log analysis
pub fn expertise_from_log_prompt(log_content: &str, id: &str, description: &str) -> String {
    format!(
        r#"Analyze the following conversation log and extract reusable knowledge as an Expertise profile.

# Conversation Log
```
{}
```

# Task
Create an Expertise profile with:

**ID**: {}
**Description**: {}
**Version**: "1.0.0"

## Guidelines

### 1. Tags
Generate 3-5 relevant tags that categorize this expertise.
Examples: "rust", "error-handling", "async", "testing", "code-review"

### 2. Knowledge Fragments
Organize knowledge into typed fragments:

**Logic Fragments**: Thinking procedures and decision-making patterns
- instruction: High-level reasoning principle
- steps: Concrete chain-of-thought steps

**Guideline Fragments**: Behavioral rules with anchoring examples
- rule: The guideline statement
- anchors: Positive/negative example pairs with context and reasoning

**QualityStandard Fragments**: Evaluation criteria
- criteria: List of quality checks
- passing_grade: Description of what "good" looks like

**Text Fragments**: General knowledge or context

### 3. Priorities
Assign priorities based on importance:
- **Critical**: Core principles that must never be violated
- **High**: Important patterns that should be followed
- **Normal**: Standard practices (default)
- **Low**: Nice-to-have suggestions

### 4. Context Profiles
By default, fragments are "Always" active.
For specialized knowledge, use "Conditional" with task_types or user_states.

Generate the complete Expertise object following the schema.
"#,
        log_content, id, description
    )
}

/// Generate prompt for improving existing Expertise
pub fn improve_expertise_prompt(current_json: &str, instruction: &str) -> String {
    format!(
        r#"Here is an existing Expertise profile:

```json
{}
```

# User Request
{}

# Task
Improve the Expertise according to the user's request while maintaining its core structure and purpose.

Guidelines:
1. Preserve the ID and scope
2. Increment the version appropriately (e.g., 1.0.0 -> 1.1.0 for minor changes, 2.0.0 for major changes)
3. Update or add fragments as needed
4. Maintain or improve the organization and clarity
5. Keep existing valuable content unless explicitly asked to remove it

Return the complete updated Expertise object.
"#,
        current_json, instruction
    )
}

/// Generate prompt for interactive Expertise creation
pub fn interactive_expertise_prompt(
    id: &str,
    description: &str,
    domain: &str,
    additional_context: Option<&str>,
) -> String {
    let context_section = additional_context
        .map(|ctx| format!("\n\n**Additional Context**:\n{}", ctx))
        .unwrap_or_default();

    format!(
        r#"Create a comprehensive Expertise profile for:

**ID**: {}
**Description**: {}
**Domain**: {}{}

# Task
Generate a well-structured Expertise with:

1. **Appropriate tags** (3-5) that reflect the domain and capabilities
2. **Knowledge fragments** organized by type:
   - Logic: Core reasoning and decision-making patterns
   - Guideline: Behavioral rules with concrete examples
   - QualityStandard: Evaluation criteria
   - Text: Background knowledge and context

3. **Priorities** assigned based on importance
4. **Version**: Start with "1.0.0"

Make it practical and immediately usable by an AI agent working in this domain.

Return the complete Expertise object.
"#,
        id, description, domain, context_section
    )
}

/// Generate prompt for merging multiple Expertises
pub fn merge_expertises_prompt(
    expertises_json: &[String],
    output_id: &str,
    description: &str,
) -> String {
    let expertises_section = expertises_json
        .iter()
        .enumerate()
        .map(|(i, json)| format!("## Expertise {}\n```json\n{}\n```", i + 1, json))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r#"Merge the following Expertises into a single, unified profile:

{}

# Task
Create a new Expertise with:

**ID**: {}
**Description**: {}
**Version**: "1.0.0"

## Merging Guidelines

1. **Combine knowledge** from all sources, eliminating redundancy
2. **Resolve conflicts** by choosing the most robust/recent approach
3. **Preserve valuable details** from all sources
4. **Reorganize** for clarity and coherence
5. **Maintain all tags** that are relevant (deduplicated)
6. **Set appropriate priorities** based on consensus importance

The merged Expertise should be more comprehensive and valuable than any individual source.

Return the complete merged Expertise object.
"#,
        expertises_section, output_id, description
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_not_empty() {
        assert!(!expertise_extraction_system().is_empty());
    }

    #[test]
    fn test_log_prompt_contains_id() {
        let prompt = expertise_from_log_prompt("test log", "test-id", "Test description");
        assert!(prompt.contains("test-id"));
    }

    #[test]
    fn test_improve_prompt_contains_instruction() {
        let prompt = improve_expertise_prompt("{}", "Add error handling");
        assert!(prompt.contains("Add error handling"));
    }

    #[test]
    fn test_interactive_prompt_contains_domain() {
        let prompt = interactive_expertise_prompt("id", "desc", "rust-programming", None);
        assert!(prompt.contains("rust-programming"));
    }

    #[test]
    fn test_merge_prompt_multiple_expertises() {
        let expertises = vec![
            r#"{"id":"exp1"}"#.to_string(),
            r#"{"id":"exp2"}"#.to_string(),
        ];
        let prompt = merge_expertises_prompt(&expertises, "merged", "Merged expertise");
        assert!(prompt.contains("exp1"));
        assert!(prompt.contains("exp2"));
    }
}
