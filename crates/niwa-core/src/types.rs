//! Type definitions and re-exports from llm-toolkit

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// Re-export from llm-toolkit-expertise
// Note: llm-toolkit-expertise v0.2.1 is a separate crate (deprecated but functional)
pub use llm_toolkit_expertise::{Expertise as LlmExpertise, KnowledgeFragment, WeightedFragment};

/// Scope for expertise organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Scope {
    /// Personal expertise (user-specific)
    #[default]
    Personal,
    /// Company expertise (organization-wide)
    Company,
    /// Project expertise (project-specific)
    Project,
}

impl FromStr for Scope {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, crate::Error> {
        match s.to_lowercase().as_str() {
            "personal" => Ok(Scope::Personal),
            "company" => Ok(Scope::Company),
            "project" => Ok(Scope::Project),
            _ => Err(crate::Error::InvalidScope(s.to_string())),
        }
    }
}

impl Scope {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::Personal => "personal",
            Scope::Company => "company",
            Scope::Project => "project",
        }
    }

    /// Get all scopes
    pub fn all() -> &'static [Scope] {
        &[Scope::Personal, Scope::Company, Scope::Project]
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Expertise with NIWA-specific metadata
///
/// This wraps llm-toolkit's Expertise with additional metadata
/// needed for storage and management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expertise {
    /// The underlying llm-toolkit Expertise
    #[serde(flatten)]
    pub inner: LlmExpertise,

    /// NIWA-specific metadata
    #[serde(flatten)]
    pub metadata: ExpertiseMetadata,
}

impl Expertise {
    /// Create a new Expertise
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            inner: LlmExpertise::new(id, version),
            metadata: ExpertiseMetadata::default(),
        }
    }

    /// Get the ID
    pub fn id(&self) -> &str {
        &self.inner.id
    }

    /// Get the version
    pub fn version(&self) -> &str {
        &self.inner.version
    }

    /// Get the description
    pub fn description(&self) -> String {
        self.inner.get_description()
    }

    /// Get tags
    pub fn tags(&self) -> &[String] {
        &self.inner.tags
    }

    /// Convert to JSON for storage
    pub fn to_json(&self) -> Result<String, crate::Error> {
        Ok(serde_json::to_string(self)?)
    }

    /// Parse from JSON
    pub fn from_json(json: &str) -> Result<Self, crate::Error> {
        Ok(serde_json::from_str(json)?)
    }
}

/// NIWA-specific metadata for Expertise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseMetadata {
    /// Scope
    pub scope: Scope,

    /// Created timestamp (Unix timestamp in seconds)
    pub created_at: i64,

    /// Last updated timestamp (Unix timestamp in seconds)
    pub updated_at: i64,
}

impl Default for ExpertiseMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            scope: Scope::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl ExpertiseMetadata {
    /// Create new metadata with given scope
    pub fn new(scope: Scope) -> Self {
        Self {
            scope,
            ..Default::default()
        }
    }

    /// Update the updated_at timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_conversion() {
        assert_eq!(Scope::from_str("personal").unwrap(), Scope::Personal);
        assert_eq!(Scope::from_str("COMPANY").unwrap(), Scope::Company);
        assert_eq!(Scope::from_str("Project").unwrap(), Scope::Project);

        assert!(Scope::from_str("invalid").is_err());
    }

    #[test]
    fn test_scope_display() {
        assert_eq!(Scope::Personal.to_string(), "personal");
        assert_eq!(Scope::Company.to_string(), "company");
        assert_eq!(Scope::Project.to_string(), "project");
    }

    #[test]
    fn test_expertise_creation() {
        let expertise = Expertise::new("test-id", "1.0.0");
        assert_eq!(expertise.id(), "test-id");
        assert_eq!(expertise.version(), "1.0.0");
        assert_eq!(expertise.metadata.scope, Scope::Personal);
    }

    #[test]
    fn test_expertise_json_roundtrip() {
        let expertise = Expertise::new("test-id", "1.0.0");
        let json = expertise.to_json().unwrap();
        let parsed = Expertise::from_json(&json).unwrap();

        assert_eq!(parsed.id(), expertise.id());
        assert_eq!(parsed.version(), expertise.version());
    }
}
