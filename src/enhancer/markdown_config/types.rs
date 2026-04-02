use serde::{Deserialize, Serialize};

/// Extracted markdown configuration block bound to a tagged heading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownConfigBlock {
    /// Exact identifier from HTML property tag.
    pub id: String,
    /// Configuration kind from HTML property tag.
    pub config_type: String,
    /// Optional logical template target.
    pub target: Option<String>,
    /// Heading title that owns this config block.
    pub heading: String,
    /// Fenced code language (for example `jinja2`).
    pub language: String,
    /// Raw code block content extracted from AST.
    pub content: String,
}

/// One normalized link target extracted under a tagged config heading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownConfigLinkTarget {
    /// Normalized target path or semantic URI.
    pub target: String,
    /// Optional type-hint parsed from a wikilink suffix (for example `#persona`).
    pub reference_type: Option<String>,
}
