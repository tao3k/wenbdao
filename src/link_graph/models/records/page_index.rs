use std::sync::Arc;

/// One hierarchical `PageIndex` node derived from a markdown heading section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageIndexNode {
    /// Stable semantic node identifier scoped to a single document.
    pub node_id: String,
    /// Stable parent anchor identifier when this node is nested under another heading.
    pub parent_id: Option<String>,
    /// Display title for this node.
    pub title: String,
    /// Heading depth normalized to the range `1..=6`.
    pub level: usize,
    /// Content chunk owned by this node after optional thinning.
    pub text: Arc<str>,
    /// Optional downstream summary payload.
    pub summary: Option<String>,
    /// Child headings nested under this node.
    pub children: Vec<PageIndexNode>,
    /// Structural metadata collected during build/thinning.
    pub metadata: PageIndexMeta,
}

/// Metadata describing a `PageIndex` node's source span and pruning state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageIndexMeta {
    /// Inclusive 1-based source line range within the markdown body.
    pub line_range: (usize, usize),
    /// Best-effort whitespace token count for the current node text.
    pub token_count: usize,
    /// Whether descendant content was folded into this node.
    pub is_thinned: bool,
}
