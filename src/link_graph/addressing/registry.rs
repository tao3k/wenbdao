//! O(1) lookup table for stable ID anchoring.
//!
//! The RegistryIndex provides fast direct lookups for nodes with explicit `:ID:` attributes,
//! enabling O(1) resolution regardless of document structure depth.

use std::collections::HashMap;

use crate::link_graph::PageIndexNode;

/// Indexed node with its document context.
#[derive(Debug, Clone)]
pub struct IndexedNode {
    /// Document ID containing this node.
    pub doc_id: String,
    /// The indexed page node.
    pub node: PageIndexNode,
}

/// Record of an ID collision detected during index build.
#[derive(Debug, Clone)]
pub struct IdCollision {
    /// The duplicated ID.
    pub id: String,
    /// All documents containing this ID (doc_id, structural_path).
    pub locations: Vec<(String, Vec<String>)>,
}

/// Result of building a registry index, including any detected issues.
#[derive(Debug, Clone)]
pub struct RegistryBuildResult {
    /// The built registry index.
    pub registry: RegistryIndex,
    /// ID collisions detected during build.
    pub collisions: Vec<IdCollision>,
}

/// O(1) lookup table for stable ID anchoring.
///
/// This index enables direct resolution of nodes by their explicit `:ID:` property
/// without traversing the document tree. Essential for "pinning" references that
/// survive structural changes.
///
/// # Example
///
/// ```ignore
/// let result = RegistryIndex::build_from_trees_with_collisions(&trees);
///
/// // Check for ID collisions
/// for collision in &result.collisions {
///     warn!("ID collision: {} appears in {} documents", collision.id, collision.locations.len());
/// }
///
/// // O(1) lookup by explicit ID
/// if let Some(indexed) = result.registry.get("arch-v1") {
///     println!("Found in doc: {}", indexed.doc_id);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct RegistryIndex {
    /// Flat HashMap: node_id → (doc_id, node)
    by_id: HashMap<String, IndexedNode>,
}

impl RegistryIndex {
    /// Create an empty registry index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_id: HashMap::new(),
        }
    }

    /// Build a registry index from page index trees.
    ///
    /// Walks all document trees and indexes nodes with explicit `:ID:` attributes.
    /// Note: This silently uses the last occurrence for duplicate IDs.
    /// Use `build_from_trees_with_collisions` to detect duplicates.
    #[must_use]
    pub fn build_from_trees(trees: &HashMap<String, Vec<PageIndexNode>>) -> Self {
        let mut by_id = HashMap::new();
        for (doc_id, nodes) in trees {
            Self::index_nodes_simple(nodes, doc_id, &mut by_id);
        }
        Self { by_id }
    }

    /// Build a registry index with collision detection.
    ///
    /// Returns both the registry and a list of ID collisions found.
    /// This is the recommended build method for semantic validation.
    #[must_use]
    pub fn build_from_trees_with_collisions(
        trees: &HashMap<String, Vec<PageIndexNode>>,
    ) -> RegistryBuildResult {
        let mut by_id = HashMap::new();
        let mut collision_tracker: HashMap<String, Vec<(String, Vec<String>)>> = HashMap::new();

        for (doc_id, nodes) in trees {
            Self::index_nodes_with_tracking(nodes, doc_id, &mut by_id, &mut collision_tracker);
        }

        // Extract collisions (IDs appearing in more than one location)
        let collisions: Vec<IdCollision> = collision_tracker
            .into_iter()
            .filter(|(_, locations)| locations.len() > 1)
            .map(|(id, locations)| IdCollision { id, locations })
            .collect();

        RegistryBuildResult {
            registry: Self { by_id },
            collisions,
        }
    }

    /// Recursively index nodes with explicit IDs (simple, no tracking).
    fn index_nodes_simple(
        nodes: &[PageIndexNode],
        doc_id: &str,
        index: &mut HashMap<String, IndexedNode>,
    ) {
        for node in nodes {
            // Index if this node has an explicit ID attribute
            if let Some(id) = node.metadata.attributes.get("ID") {
                index.insert(
                    id.clone(),
                    IndexedNode {
                        doc_id: doc_id.to_string(),
                        node: node.clone(),
                    },
                );
            }
            // Recurse into children
            Self::index_nodes_simple(&node.children, doc_id, index);
        }
    }

    /// Recursively index nodes with explicit IDs (with collision tracking).
    fn index_nodes_with_tracking(
        nodes: &[PageIndexNode],
        doc_id: &str,
        index: &mut HashMap<String, IndexedNode>,
        collision_tracker: &mut HashMap<String, Vec<(String, Vec<String>)>>,
    ) {
        for node in nodes {
            // Index if this node has an explicit ID attribute
            if let Some(id) = node.metadata.attributes.get("ID") {
                // Track for collision detection
                collision_tracker
                    .entry(id.clone())
                    .or_default()
                    .push((doc_id.to_string(), node.metadata.structural_path.clone()));

                // Insert (last occurrence wins for the index itself)
                index.insert(
                    id.clone(),
                    IndexedNode {
                        doc_id: doc_id.to_string(),
                        node: node.clone(),
                    },
                );
            }
            // Recurse into children
            Self::index_nodes_with_tracking(&node.children, doc_id, index, collision_tracker);
        }
    }

    /// Look up a node by its explicit ID.
    ///
    /// Returns the indexed node with document context, or `None` if not found.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&IndexedNode> {
        self.by_id.get(id)
    }

    /// Check if an ID exists in the registry.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// Get the total number of indexed nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Iterate over all indexed entries.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &IndexedNode)> {
        self.by_id.iter()
    }

    /// Get all document IDs that have indexed nodes.
    #[must_use]
    pub fn doc_ids(&self) -> Vec<&str> {
        let mut docs: Vec<&str> = self
            .by_id
            .values()
            .map(|indexed| indexed.doc_id.as_str())
            .collect();
        docs.sort_unstable();
        docs.dedup();
        docs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn make_test_node(id: &str, title: &str) -> PageIndexNode {
        let mut attrs = HashMap::new();
        if !id.is_empty() {
            attrs.insert("ID".to_string(), id.to_string());
        }
        PageIndexNode {
            node_id: format!("doc#{}", title),
            parent_id: None,
            title: title.to_string(),
            level: 1,
            text: Arc::from("content"),
            summary: None,
            children: Vec::new(),
            blocks: Vec::new(),
            metadata: crate::link_graph::PageIndexMeta {
                line_range: (1, 10),
                byte_range: Some((0, 100)),
                structural_path: vec![title.to_string()],
                content_hash: Some("abc123".to_string()),
                attributes: attrs,
                token_count: 10,
                is_thinned: false,
            },
        }
    }

    #[test]
    fn test_empty_registry() {
        let registry = RegistryIndex::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(!registry.contains("any-id"));
    }

    #[test]
    fn test_build_from_trees() {
        let mut trees = HashMap::new();
        trees.insert(
            "doc1.md".to_string(),
            vec![
                make_test_node("intro", "Introduction"),
                make_test_node("arch", "Architecture"),
            ],
        );
        trees.insert(
            "doc2.md".to_string(),
            vec![make_test_node("config", "Configuration")],
        );

        let registry = RegistryIndex::build_from_trees(&trees);

        assert_eq!(registry.len(), 3);
        assert!(registry.contains("intro"));
        assert!(registry.contains("arch"));
        assert!(registry.contains("config"));
    }

    #[test]
    fn test_get_returns_correct_doc() {
        let mut trees = HashMap::new();
        trees.insert(
            "doc1.md".to_string(),
            vec![make_test_node("my-id", "Section")],
        );

        let registry = RegistryIndex::build_from_trees(&trees);

        let indexed = registry.get("my-id").expect("should find node");
        assert_eq!(indexed.doc_id, "doc1.md");
        assert_eq!(indexed.node.title, "Section");
    }

    #[test]
    fn test_nested_nodes_indexed() {
        let child = make_test_node("child-id", "Child Section");
        let parent = PageIndexNode {
            node_id: "doc#parent".to_string(),
            parent_id: None,
            title: "Parent".to_string(),
            level: 1,
            text: Arc::from("parent content"),
            summary: None,
            children: vec![child],
            blocks: Vec::new(),
            metadata: crate::link_graph::PageIndexMeta {
                line_range: (1, 20),
                byte_range: Some((0, 200)),
                structural_path: vec!["Parent".to_string()],
                content_hash: None,
                attributes: HashMap::new(),
                token_count: 20,
                is_thinned: false,
            },
        };

        let mut trees = HashMap::new();
        trees.insert("doc.md".to_string(), vec![parent]);

        let registry = RegistryIndex::build_from_trees(&trees);

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("child-id"));
    }

    #[test]
    fn test_nodes_without_id_not_indexed() {
        let mut trees = HashMap::new();
        trees.insert(
            "doc.md".to_string(),
            vec![
                make_test_node("", "NoID1"), // Empty ID = not indexed
                make_test_node("has-id", "HasID"),
                make_test_node("", "NoID2"), // Empty ID = not indexed
            ],
        );

        let registry = RegistryIndex::build_from_trees(&trees);

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("has-id"));
        assert!(!registry.contains("")); // Empty ID not indexed
    }

    #[test]
    fn test_doc_ids() {
        let mut trees = HashMap::new();
        trees.insert(
            "doc1.md".to_string(),
            vec![make_test_node("id1", "Section 1")],
        );
        trees.insert(
            "doc2.md".to_string(),
            vec![make_test_node("id2", "Section 2")],
        );

        let registry = RegistryIndex::build_from_trees(&trees);
        let docs = registry.doc_ids();

        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_collision_detection_no_collisions() {
        let mut trees = HashMap::new();
        trees.insert(
            "doc1.md".to_string(),
            vec![make_test_node("intro", "Introduction")],
        );
        trees.insert(
            "doc2.md".to_string(),
            vec![make_test_node("arch", "Architecture")],
        );

        let result = RegistryIndex::build_from_trees_with_collisions(&trees);

        assert!(result.collisions.is_empty());
        assert_eq!(result.registry.len(), 2);
    }

    #[test]
    fn test_collision_detection_with_collisions() {
        let mut trees = HashMap::new();
        // Same ID "intro" in two different documents
        trees.insert(
            "doc1.md".to_string(),
            vec![make_test_node("intro", "Introduction 1")],
        );
        trees.insert(
            "doc2.md".to_string(),
            vec![make_test_node("intro", "Introduction 2")],
        );

        let result = RegistryIndex::build_from_trees_with_collisions(&trees);

        assert_eq!(result.collisions.len(), 1);
        assert_eq!(result.collisions[0].id, "intro");
        assert_eq!(result.collisions[0].locations.len(), 2);

        // Registry still works (last occurrence wins)
        assert!(result.registry.contains("intro"));
    }

    #[test]
    fn test_collision_detection_multiple_collisions() {
        let mut trees = HashMap::new();
        // Multiple IDs duplicated
        trees.insert(
            "doc1.md".to_string(),
            vec![
                make_test_node("shared-id", "Section A"),
                make_test_node("another-dup", "Section B"),
            ],
        );
        trees.insert(
            "doc2.md".to_string(),
            vec![
                make_test_node("shared-id", "Section C"),
                make_test_node("another-dup", "Section D"),
            ],
        );
        trees.insert(
            "doc3.md".to_string(),
            vec![make_test_node("unique-id", "Section E")],
        );

        let result = RegistryIndex::build_from_trees_with_collisions(&trees);

        assert_eq!(result.collisions.len(), 2);
        let collision_ids: Vec<&str> =
            result.collisions.iter().map(|c| c.id.as_str()).collect();
        assert!(collision_ids.contains(&"shared-id"));
        assert!(collision_ids.contains(&"another-dup"));
    }
}
