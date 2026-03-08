use serde::{Deserialize, Serialize};

/// Represents an entity reference extracted from note content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkGraphEntityRef {
    /// Entity name (without type hint)
    pub name: String,
    /// Optional entity type hint (e.g., "rust", "py", "pattern")
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Original matched text
    #[serde(skip)]
    pub original: String,
}

impl LinkGraphEntityRef {
    /// Create a new entity reference.
    #[must_use]
    pub fn new(name: String, entity_type: Option<String>, original: String) -> Self {
        Self {
            name,
            entity_type,
            original,
        }
    }

    /// Get the wikilink format: [[Name]] or [[Name#type]]
    #[must_use]
    pub fn to_wikilink(&self) -> String {
        match &self.entity_type {
            Some(value) => format!("[[{}#{}]]", self.name, value),
            None => format!("[[{}]]", self.name),
        }
    }

    /// Get the tag format: #entity or #entity-type
    #[must_use]
    pub fn to_tag(&self) -> String {
        match &self.entity_type {
            Some(value) => format!("#entity-{}", value.to_lowercase()),
            None => "#entity".to_string(),
        }
    }
}
