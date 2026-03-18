//! Property Drawer Edge Extraction (Blueprint v2.0 Section 5.3).
//!
//! This module extracts structured edges from Org-style property drawer attributes.
//! Property drawers can contain explicit references to other nodes, which are
//! converted to typed edges in the link graph.
//!
//! ## Supported Attribute Keys
//!
//! - `:RELATED:` - References to related nodes (e.g., `:RELATED: #arch-v1, #impl-v2`)
//! - `:DEPENDS_ON:` - Dependency relationships
//! - `:EXTENDS:` - Extension/inheritance relationships
//!
//! ## Edge Type
//!
//! All property drawer edges are typed as `LinkGraphEdgeType::PropertyDrawer`,
//! enabling filtered traversal and distinct ranking behavior.

use std::collections::HashMap;

use crate::link_graph::models::LinkGraphEdgeType;

/// Standard property drawer attribute keys that contain node references.
pub mod ref_attrs {
    /// References to related nodes (comma-separated IDs).
    pub const RELATED: &str = "RELATED";
    /// Dependencies (this node depends on the referenced nodes).
    pub const DEPENDS_ON: &str = "DEPENDS_ON";
    /// Extension/inheritance (this node extends the referenced nodes).
    pub const EXTENDS: &str = "EXTENDS";
    /// See also references.
    pub const SEE_ALSO: &str = "SEE_ALSO";
}

/// All property drawer attribute keys that contain node references.
pub const REF_ATTRIBUTE_KEYS: &[&str] = &[
    ref_attrs::RELATED,
    ref_attrs::DEPENDS_ON,
    ref_attrs::EXTENDS,
    ref_attrs::SEE_ALSO,
];

/// A property drawer edge extracted from attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDrawerEdge {
    /// Source node ID (`doc_id#anchor` or just `doc_id`).
    pub from: String,
    /// Target node ID.
    pub to: String,
    /// Edge type (always `PropertyDrawer`).
    pub edge_type: LinkGraphEdgeType,
    /// The attribute key that defined this edge (e.g., "RELATED").
    pub attribute_key: String,
}

/// Extract property drawer edges from a section's attributes.
///
/// Looks for reference attributes in the format:
/// - `:RELATED: #id1, #id2`
/// - `:DEPENDS_ON: #arch-v1`
///
/// Returns a list of edges from the source node to the referenced targets.
pub fn extract_property_drawer_edges(
    source_node_id: &str,
    attributes: &HashMap<String, String>,
) -> Vec<PropertyDrawerEdge> {
    let mut edges = Vec::new();

    for &attr_key in REF_ATTRIBUTE_KEYS {
        if let Some(value) = attributes.get(attr_key) {
            let refs = parse_id_references(value);
            for target_id in refs {
                edges.push(PropertyDrawerEdge {
                    from: source_node_id.to_string(),
                    to: target_id,
                    edge_type: LinkGraphEdgeType::PropertyDrawer,
                    attribute_key: attr_key.to_string(),
                });
            }
        }
    }

    edges
}

/// Parse ID references from a property drawer value.
///
/// Supports formats:
/// - `#id1` - Single ID reference
/// - `#id1, #id2, #id3` - Comma-separated references
/// - `#id1 #id2` - Space-separated references
pub fn parse_id_references(value: &str) -> Vec<String> {
    value
        .split([',', ' ', '\n', '\t'])
        .filter_map(|s| {
            let trimmed = s.trim();
            if let Some(id) = trimmed.strip_prefix('#') {
                let id = id.trim();
                if !id.is_empty() {
                    return Some(id.to_string());
                }
            }
            None
        })
        .collect()
}

#[cfg(test)]
#[path = "../../../../tests/unit/link_graph/index/build/property_drawer_edges.rs"]
mod tests;
