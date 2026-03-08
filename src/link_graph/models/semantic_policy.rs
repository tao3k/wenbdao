use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Semantic document scope used when hybrid retrieval escalates into semantic ignition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinkGraphSemanticDocumentScope {
    /// Search across every semantic row visible to the backend.
    #[default]
    All,
    /// Restrict semantic ignition to summary rows only.
    SummaryOnly,
}

impl LinkGraphSemanticDocumentScope {
    /// Parse a semantic-scope alias from user-facing query directives.
    #[must_use]
    pub fn from_alias(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "all" => Some(Self::All),
            "summary" | "summary_only" => Some(Self::SummaryOnly),
            _ => None,
        }
    }
}

/// Semantic ignition policy carried by `LinkGraph` planning surfaces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct LinkGraphSemanticSearchPolicy {
    /// Document-scope constraint to apply when the vector stage runs.
    pub document_scope: LinkGraphSemanticDocumentScope,
    /// Optional minimum vector score required before topology diffusion runs.
    pub min_vector_score: Option<f64>,
}

impl LinkGraphSemanticSearchPolicy {
    /// Return a normalized copy with bounded numeric policy values.
    #[must_use]
    pub fn normalized(&self) -> Self {
        Self {
            document_scope: self.document_scope,
            min_vector_score: self.min_vector_score.map(|score| score.clamp(0.0, 1.0)),
        }
    }

    /// Merge request-local policy with runtime defaults.
    #[must_use]
    pub fn merged_with_defaults(&self, defaults: &Self) -> Self {
        Self {
            document_scope: if matches!(self.document_scope, LinkGraphSemanticDocumentScope::All) {
                defaults.document_scope
            } else {
                self.document_scope
            },
            min_vector_score: self.min_vector_score.or(defaults.min_vector_score),
        }
        .normalized()
    }

    /// Return whether the policy restricts semantic ignition to summary rows.
    #[must_use]
    pub const fn summary_only(&self) -> bool {
        matches!(
            self.document_scope,
            LinkGraphSemanticDocumentScope::SummaryOnly
        )
    }
}
