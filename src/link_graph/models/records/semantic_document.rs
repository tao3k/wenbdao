use std::sync::Arc;

/// Typed semantic document exported from `LinkGraphIndex` for downstream retrieval runtimes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkGraphSemanticDocument {
    /// Stable anchor identifier used to recover semantic paths.
    pub anchor_id: String,
    /// Canonical document identifier owning this semantic document.
    pub doc_id: String,
    /// Relative markdown path for traceability.
    pub path: String,
    /// Semantic document kind used by downstream document-scope filters.
    pub kind: LinkGraphSemanticDocumentKind,
    /// Complete logical ancestry path recovered from `PageIndex`.
    pub semantic_path: Vec<String>,
    /// Text payload exported for semantic indexing.
    pub content: Arc<str>,
    /// Optional source line range when the document maps to one concrete section.
    pub line_range: Option<(usize, usize)>,
}

/// Semantic document kind exported from `LinkGraphIndex`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkGraphSemanticDocumentKind {
    /// One document-level summary row.
    Summary,
    /// One section-level semantic row derived from `PageIndex`.
    Section,
}

impl LinkGraphSemanticDocumentKind {
    /// Return the canonical metadata label used by vector retrieval adapters.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Section => "section",
        }
    }
}
