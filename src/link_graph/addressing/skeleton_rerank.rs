//! Skeleton Re-ranking for Hybrid Semantic Search (Blueprint Section 2.2).
//!
//! This module implements AST-guidanced validation of vector search results.
//! After vector search returns Top-K candidates, we validate each result against
//! the current AST structure to filter out stale or orphaned fragments.

use crate::link_graph::models::QuantumAnchorHit;
use super::{RegistryIndex, TopologyIndex};

/// Result of skeleton re-ranking with validation metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletonValidatedHit {
    /// Original anchor hit from vector search.
    pub hit: QuantumAnchorHit,
    /// Whether the anchor exists in current AST.
    pub is_valid: bool,
    /// Document ID extracted from anchor_id.
    pub doc_id: String,
    /// Anchor/node ID extracted from anchor_id.
    pub anchor: String,
    /// Optional structural path if found in topology.
    pub structural_path: Option<Vec<String>>,
    /// Boosted score for structurally consistent results.
    pub reranked_score: f64,
}

/// Options for skeleton re-ranking.
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletonRerankOptions {
    /// Score boost for anchors validated against AST (0.0-1.0).
    pub validity_boost: f64,
    /// Score penalty for invalid anchors (0.0-1.0).
    pub invalidity_penalty: f64,
    /// Whether to filter out invalid anchors entirely.
    pub filter_invalid: bool,
}

impl Default for SkeletonRerankOptions {
    fn default() -> Self {
        Self {
            validity_boost: 0.1,
            invalidity_penalty: 0.5,
            filter_invalid: false,
        }
    }
}

impl SkeletonRerankOptions {
    /// Create options that filter out all invalid anchors.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            validity_boost: 0.1,
            invalidity_penalty: 1.0,
            filter_invalid: true,
        }
    }

    /// Create options that keep all anchors but adjust scores.
    #[must_use]
    pub fn lenient() -> Self {
        Self {
            validity_boost: 0.05,
            invalidity_penalty: 0.2,
            filter_invalid: false,
        }
    }
}

/// Re-rank vector search results against current AST skeleton.
///
/// # Arguments
///
/// * `hits` - Vector search results to validate
/// * `registry` - Registry index for O(1) ID lookup
/// * `topology` - Topology index for path validation
/// * `options` - Re-ranking options
///
/// # Returns
///
/// Vector of validated hits, sorted by reranked_score descending.
#[must_use]
pub fn skeleton_rerank(
    hits: Vec<QuantumAnchorHit>,
    registry: &RegistryIndex,
    topology: &TopologyIndex,
    options: &SkeletonRerankOptions,
) -> Vec<SkeletonValidatedHit> {
    let mut validated: Vec<SkeletonValidatedHit> = hits
        .into_iter()
        .map(|hit| validate_hit(hit, registry, topology, options))
        .collect();

    // Sort by reranked_score descending
    validated.sort_by(|a, b| {
        b.reranked_score
            .partial_cmp(&a.reranked_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Filter if requested
    if options.filter_invalid {
        validated.retain(|h| h.is_valid);
    }

    validated
}

/// Validate a single anchor hit against the dual indices.
fn validate_hit(
    hit: QuantumAnchorHit,
    registry: &RegistryIndex,
    topology: &TopologyIndex,
    options: &SkeletonRerankOptions,
) -> SkeletonValidatedHit {
    let (doc_id, anchor) = parse_anchor_id(&hit.anchor_id);

    // Try registry lookup first (O(1)) - use just the ID, not full anchor_id
    let registry_match = registry.get(&anchor);

    // Try topology node ID validation
    let path_match = topology.find_by_node_id(&hit.anchor_id);

    let (is_valid, structural_path) = if registry_match.is_some() {
        // Found in registry - exact ID match
        let node = registry_match.unwrap();
        (true, Some(node.node.metadata.structural_path.clone()))
    } else if path_match.is_some() {
        // Found via node_id in topology
        let entry = path_match.unwrap();
        (true, Some(entry.path.clone()))
    } else {
        // Not found in current AST
        (false, None)
    };

    // Calculate reranked score
    let reranked_score = if is_valid {
        (hit.vector_score + options.validity_boost).min(1.0)
    } else {
        (hit.vector_score - options.invalidity_penalty).max(0.0)
    };

    SkeletonValidatedHit {
        hit,
        is_valid,
        doc_id,
        anchor,
        structural_path,
        reranked_score,
    }
}

/// Parse anchor_id into (doc_id, anchor) tuple.
///
/// Anchor IDs are typically formatted as `doc_id#anchor` or `doc_id#slug`.
fn parse_anchor_id(anchor_id: &str) -> (String, String) {
    match anchor_id.split_once('#') {
        Some((doc, anchor)) => (doc.to_string(), anchor.to_string()),
        None => (anchor_id.to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link_graph::{PageIndexMeta, PageIndexNode};
    use std::collections::HashMap;
    use std::sync::Arc;

    fn make_test_node(id: &str, title: &str, path: Vec<&str>) -> PageIndexNode {
        let mut attrs = HashMap::new();
        if !id.is_empty() {
            attrs.insert("ID".to_string(), id.to_string());
        }
        PageIndexNode {
            node_id: format!("doc#{}", title),
            parent_id: None,
            title: title.to_string(),
            level: path.len(),
            text: Arc::from("content"),
            summary: None,
            children: Vec::new(),
            blocks: Vec::new(),
            metadata: PageIndexMeta {
                line_range: (1, 10),
                byte_range: Some((0, 100)),
                structural_path: path.iter().map(|s| s.to_string()).collect(),
                content_hash: Some(format!("hash_{}", title)),
                attributes: attrs,
                token_count: 10,
                is_thinned: false,
            },
        }
    }

    fn build_test_indices() -> (RegistryIndex, TopologyIndex) {
        let mut trees = HashMap::new();

        let intro = make_test_node("intro-id", "Introduction", vec!["Introduction"]);
        let storage =
            make_test_node("storage-id", "Storage", vec!["Architecture", "Storage"]);

        trees.insert("test_doc.md".to_string(), vec![intro, storage]);

        let registry = RegistryIndex::build_from_trees(&trees);
        let topology = TopologyIndex::build_from_trees(&trees);

        (registry, topology)
    }

    #[test]
    fn test_skeleton_rerank_validates_existing_anchors() {
        let (registry, topology) = build_test_indices();

        let hits = vec![
            QuantumAnchorHit {
                anchor_id: "test_doc.md#intro-id".to_string(),
                vector_score: 0.8,
            },
            QuantumAnchorHit {
                anchor_id: "test_doc.md#storage-id".to_string(),
                vector_score: 0.7,
            },
        ];

        let results = skeleton_rerank(hits, &registry, &topology, &SkeletonRerankOptions::default());

        assert_eq!(results.len(), 2);
        assert!(results[0].is_valid);
        assert!(results[1].is_valid);
        // First hit should have boosted score
        assert!(results[0].reranked_score > 0.8);
    }

    #[test]
    fn test_skeleton_rerank_penalizes_missing_anchors() {
        let (registry, topology) = build_test_indices();

        let hits = vec![
            QuantumAnchorHit {
                anchor_id: "test_doc.md#intro-id".to_string(),
                vector_score: 0.8,
            },
            QuantumAnchorHit {
                anchor_id: "test_doc.md#deleted-anchor".to_string(),
                vector_score: 0.75,
            },
        ];

        let results = skeleton_rerank(hits, &registry, &topology, &SkeletonRerankOptions::default());

        assert_eq!(results.len(), 2);
        assert!(results[0].is_valid);
        assert!(!results[1].is_valid);
        // Invalid hit should have penalized score
        assert!(results[1].reranked_score < 0.75);
    }

    #[test]
    fn test_skeleton_rerank_filters_invalid_when_strict() {
        let (registry, topology) = build_test_indices();

        let hits = vec![
            QuantumAnchorHit {
                anchor_id: "test_doc.md#intro-id".to_string(),
                vector_score: 0.8,
            },
            QuantumAnchorHit {
                anchor_id: "test_doc.md#deleted-anchor".to_string(),
                vector_score: 0.75,
            },
        ];

        let results = skeleton_rerank(hits, &registry, &topology, &SkeletonRerankOptions::strict());

        assert_eq!(results.len(), 1);
        assert!(results[0].is_valid);
    }

    #[test]
    fn test_parse_anchor_id() {
        assert_eq!(
            parse_anchor_id("doc.md#intro"),
            ("doc.md".to_string(), "intro".to_string())
        );
        assert_eq!(
            parse_anchor_id("path/to/doc.md#section-1"),
            ("path/to/doc.md".to_string(), "section-1".to_string())
        );
        assert_eq!(
            parse_anchor_id("no-hash"),
            ("no-hash".to_string(), "".to_string())
        );
    }

    #[test]
    fn test_skeleton_rerank_preserves_structural_path() {
        let (registry, topology) = build_test_indices();

        let hits = vec![QuantumAnchorHit {
            anchor_id: "test_doc.md#storage-id".to_string(),
            vector_score: 0.8,
        }];

        let results = skeleton_rerank(hits, &registry, &topology, &SkeletonRerankOptions::default());

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].structural_path,
            Some(vec!["Architecture".to_string(), "Storage".to_string()])
        );
    }
}
