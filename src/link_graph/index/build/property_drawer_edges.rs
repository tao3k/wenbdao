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

use std::collections::{HashMap, HashSet};

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
    /// Source node ID (doc_id#anchor or just doc_id).
    pub from: String,
    /// Target node ID.
    pub to: String,
    /// Edge type (always PropertyDrawer).
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
fn parse_id_references(value: &str) -> Vec<String> {
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

/// Add property drawer edges to the graph edge maps.
///
/// This function takes extracted edges and adds them to the outgoing/incoming
/// edge maps. It returns the count of new edges added.
pub fn add_edges_to_graph(
    edges: &[PropertyDrawerEdge],
    outgoing: &mut HashMap<String, HashSet<String>>,
    incoming: &mut HashMap<String, HashSet<String>>,
) -> usize {
    let mut added = 0;

    for edge in edges {
        let inserted = outgoing
            .entry(edge.from.clone())
            .or_default()
            .insert(edge.to.clone());
        if inserted {
            incoming
                .entry(edge.to.clone())
                .or_default()
                .insert(edge.from.clone());
            added += 1;
        }
    }

    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_id_references_single() {
        let refs = parse_id_references("#arch-v1");
        assert_eq!(refs, vec!["arch-v1"]);
    }

    #[test]
    fn test_parse_id_references_comma_separated() {
        let refs = parse_id_references("#id1, #id2, #id3");
        assert_eq!(refs, vec!["id1", "id2", "id3"]);
    }

    #[test]
    fn test_parse_id_references_space_separated() {
        let refs = parse_id_references("#id1 #id2 #id3");
        assert_eq!(refs, vec!["id1", "id2", "id3"]);
    }

    #[test]
    fn test_parse_id_references_mixed() {
        let refs = parse_id_references("#id1, #id2 #id3,\n#id4");
        assert_eq!(refs, vec!["id1", "id2", "id3", "id4"]);
    }

    #[test]
    fn test_parse_id_references_empty() {
        let refs = parse_id_references("");
        assert!(refs.is_empty());
    }

    #[test]
    fn test_parse_id_references_no_hash() {
        let refs = parse_id_references("id1, id2");
        assert!(refs.is_empty());
    }

    #[test]
    fn test_extract_property_drawer_edges_related() {
        let mut attrs = HashMap::new();
        attrs.insert("RELATED".to_string(), "#arch-v1, #impl-v2".to_string());

        let edges = extract_property_drawer_edges("doc.md#intro", &attrs);

        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].from, "doc.md#intro");
        assert_eq!(edges[0].to, "arch-v1");
        assert_eq!(edges[0].edge_type, LinkGraphEdgeType::PropertyDrawer);
        assert_eq!(edges[0].attribute_key, "RELATED");
        assert_eq!(edges[1].to, "impl-v2");
    }

    #[test]
    fn test_extract_property_drawer_edges_multiple_attrs() {
        let mut attrs = HashMap::new();
        attrs.insert("RELATED".to_string(), "#id1".to_string());
        attrs.insert("DEPENDS_ON".to_string(), "#id2".to_string());

        let edges = extract_property_drawer_edges("doc.md#section", &attrs);

        assert_eq!(edges.len(), 2);
        assert!(edges.iter().any(|e| e.attribute_key == "RELATED"));
        assert!(edges.iter().any(|e| e.attribute_key == "DEPENDS_ON"));
    }

    #[test]
    fn test_add_edges_to_graph() {
        let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
        let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();

        let edges = vec![
            PropertyDrawerEdge {
                from: "doc1.md".to_string(),
                to: "doc2.md".to_string(),
                edge_type: LinkGraphEdgeType::PropertyDrawer,
                attribute_key: "RELATED".to_string(),
            },
            PropertyDrawerEdge {
                from: "doc1.md".to_string(),
                to: "doc3.md".to_string(),
                edge_type: LinkGraphEdgeType::PropertyDrawer,
                attribute_key: "RELATED".to_string(),
            },
        ];

        let added = add_edges_to_graph(&edges, &mut outgoing, &mut incoming);

        assert_eq!(added, 2);
        assert_eq!(outgoing.get("doc1.md").map(|s| s.len()), Some(2));
        assert!(incoming.contains_key("doc2.md"));
        assert!(incoming.contains_key("doc3.md"));
    }

    #[test]
    fn test_add_edges_to_graph_no_duplicates() {
        let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
        let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();

        // Add same edge twice
        let edges = vec![
            PropertyDrawerEdge {
                from: "doc1.md".to_string(),
                to: "doc2.md".to_string(),
                edge_type: LinkGraphEdgeType::PropertyDrawer,
                attribute_key: "RELATED".to_string(),
            },
            PropertyDrawerEdge {
                from: "doc1.md".to_string(),
                to: "doc2.md".to_string(),
                edge_type: LinkGraphEdgeType::PropertyDrawer,
                attribute_key: "RELATED".to_string(),
            },
        ];

        let added = add_edges_to_graph(&edges, &mut outgoing, &mut incoming);

        // Only one should be added (duplicate detected)
        assert_eq!(added, 1);
    }
}
