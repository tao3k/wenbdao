use serde::{Deserialize, Serialize};

/// Parsed YAML frontmatter from a markdown note.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NoteFrontmatter {
    /// Document title from frontmatter.
    pub title: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
    /// Skill name (for SKILL.md files).
    pub name: Option<String>,
    /// Document category (e.g. "pattern", "architecture").
    pub category: Option<String>,
    /// Tags for discovery and filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Routing keywords from `metadata.routing_keywords`.
    #[serde(default)]
    pub routing_keywords: Vec<String>,
    /// Intent descriptions from `metadata.intents`.
    #[serde(default)]
    pub intents: Vec<String>,
}

/// Input for a single note to be enhanced.
#[derive(Debug, Clone)]
pub struct NoteInput {
    /// Relative path to the note (e.g. `docs/architecture/foo.md`).
    pub path: String,
    /// Note title (from backend or frontmatter).
    pub title: String,
    /// Full raw content of the note.
    pub content: String,
}

/// A relation inferred from note structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredRelation {
    /// Source entity name.
    pub source: String,
    /// Target entity name.
    pub target: String,
    /// Relation type string (e.g. `DOCUMENTED_IN`, `CONTAINS`).
    pub relation_type: String,
    /// Human-readable description of the relation.
    pub description: String,
}

/// A note enriched with secondary analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedNote {
    /// Note path.
    pub path: String,
    /// Note title.
    pub title: String,
    /// Parsed YAML frontmatter.
    pub frontmatter: NoteFrontmatter,
    /// Entity references extracted from wikilinks.
    pub entity_refs: Vec<EntityRefData>,
    /// Reference statistics.
    pub ref_stats: RefStatsData,
    /// Relations inferred from note structure.
    pub inferred_relations: Vec<InferredRelation>,
}

/// Serializable entity reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRefData {
    /// Entity name.
    pub name: String,
    /// Optional entity type hint (from `[[Name#type]]`).
    pub entity_type: Option<String>,
    /// Original matched text.
    pub original: String,
}

/// Serializable reference statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefStatsData {
    /// Total entity references found.
    pub total_refs: usize,
    /// Number of unique entities referenced.
    pub unique_entities: usize,
    /// Reference counts grouped by entity type.
    pub by_type: Vec<(String, usize)>,
}
