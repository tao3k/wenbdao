use super::ParsedSection;
use crate::link_graph::models::LinkGraphDocument;

/// Parsed note payload + unresolved outgoing link targets.
#[derive(Debug, Clone)]
pub struct ParsedNote {
    /// Canonical document row.
    pub doc: LinkGraphDocument,
    /// Raw link targets extracted from content.
    pub link_targets: Vec<String>,
    /// Raw attachment targets extracted from content.
    pub attachment_targets: Vec<String>,
    /// Parsed markdown sections/headings for section-aware retrieval.
    pub sections: Vec<ParsedSection>,
}
