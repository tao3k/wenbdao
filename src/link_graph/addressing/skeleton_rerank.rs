//! Skeleton Re-ranking for Hybrid Semantic Search (Blueprint Section 2.2).
//!
//! This module implements AST-guidanced validation of vector search results.
//! After vector search returns Top-K candidates, we validate each result against
//! the current AST structure to filter out stale or orphaned fragments.

use super::{RegistryIndex, TopologyIndex};
use crate::link_graph::models::QuantumAnchorHit;

/// Result of skeleton re-ranking with validation metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletonValidatedHit {
    /// Original anchor hit from vector search.
    pub hit: QuantumAnchorHit,
    /// Whether the anchor exists in current AST.
    pub is_valid: bool,
    /// Document ID extracted from `anchor_id`.
    pub doc_id: String,
    /// Anchor/node ID extracted from `anchor_id`.
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
/// Vector of validated hits, sorted by `reranked_score` descending.
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

    let (is_valid, structural_path) = if let Some(node) = registry_match {
        // Found in registry - exact ID match
        (true, Some(node.node.metadata.structural_path.clone()))
    } else if let Some(entry) = path_match {
        // Found via node_id in topology
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

/// Parse `anchor_id` into (`doc_id`, anchor) tuple.
///
/// Anchor IDs are typically formatted as `doc_id#anchor` or `doc_id#slug`.
fn parse_anchor_id(anchor_id: &str) -> (String, String) {
    match anchor_id.split_once('#') {
        Some((doc, anchor)) => (doc.to_string(), anchor.to_string()),
        None => (anchor_id.to_string(), String::new()),
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/link_graph/addressing/skeleton_rerank.rs"]
mod tests;
