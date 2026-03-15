//! Triple-A Addressing Protocol for semantic node resolution.
//!
//! Implements the three-layer addressing protocol from the Project Anchor blueprint:
//! 1. **Anchor** (explicit ID): Resolve by `:ID:` property drawer attribute
//! 2. **AST Path** (structural path): Resolve by heading hierarchy path
//! 3. **Alias** (content hash): Resolve by Blake3 content fingerprint
//!
//! ## Dual-Index Architecture
//!
//! This module provides two complementary index types:
//! - **RegistryIndex**: O(1) lookup for stable ID anchoring (used by agents/systems)
//! - **TopologyIndex**: Fuzzy path discovery for human-friendly navigation
//!
//! ## Usage
//!
//! ```ignore
//! use crate::link_graph::addressing::{Address, resolve_node, ResolveMode};
//!
//! // Resolve by explicit ID
//! let addr = Address::Id("arch-v1".to_string());
//! let node = resolve_node(&index, &addr, "doc.md");
//!
//! // Resolve by structural path
//! let addr = Address::Path(vec!["Architecture".to_string(), "Storage".to_string()]);
//! let node = resolve_node(&index, &addr, "doc.md");
//!
//! // Resolve by content hash (self-healing)
//! let addr = Address::Hash("a1b2c3d4e5f6".to_string());
//! let node = resolve_node(&index, &addr, "doc.md");
//!
//! // Resolve block within section
//! let addr = Address::Block {
//!     section_path: vec!["Architecture".to_string()],
//!     block_addr: BlockAddress::new(BlockKindSpecifier::Paragraph, 2),
//! };
//!
//! // Use dual indices for enhanced resolution
//! let registry = RegistryIndex::build_from_trees(trees);
//! let topology = TopologyIndex::build_from_trees(trees);
//! let result = resolve_with_indices(&registry, &topology, &addr, doc_id, ResolveMode::Anchor);
//! ```

mod registry;
mod skeleton_rerank;
mod structural_transaction;
mod topology;

use crate::link_graph::models::{BlockAddress, PageIndexNode};

pub use registry::{IdCollision, IndexedNode, RegistryBuildResult, RegistryIndex};
pub use skeleton_rerank::{SkeletonRerankOptions, SkeletonValidatedHit, skeleton_rerank};
pub use structural_transaction::{
    StructureUpdateSignal, StructuralTransaction, StructuralTransactionCoordinator,
};
pub use topology::{MatchType, PathEntry, PathMatch, TopologyIndex};

/// Semantic address for node resolution following the Triple-A protocol.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Address {
    /// Explicit anchor ID from `:ID:` property drawer.
    /// Highest priority - directly identifies a node.
    Id(String),
    /// Structural path through heading hierarchy.
    /// Medium priority - e.g., `["Architecture", "Storage"]`.
    Path(Vec<String>),
    /// Content fingerprint (Blake3 hash).
    /// Lowest priority - used for self-healing when content moved.
    Hash(String),
    /// Block-level address within a section.
    ///
    /// Enables fine-grained addressing like `/Section/Paragraph[2]`.
    Block {
        /// Path to the containing section.
        section_path: Vec<String>,
        /// Block address within the section.
        block_addr: BlockAddress,
    },
}

impl Address {
    /// Create an ID-based address.
    #[must_use]
    pub fn id(id: impl Into<String>) -> Self {
        Self::Id(id.into())
    }

    /// Create a path-based address.
    #[must_use]
    pub fn path(components: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Path(components.into_iter().map(Into::into).collect())
    }

    /// Create a hash-based address.
    #[must_use]
    pub fn hash(hash: impl Into<String>) -> Self {
        Self::Hash(hash.into())
    }

    /// Parse an address string.
    ///
    /// Formats:
    /// - `#id` - explicit ID (e.g., `#arch-v1`)
    /// - `/path/to/heading` - structural path
    /// - `@hash` - content hash
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some(id) = trimmed.strip_prefix('#') {
            if id.is_empty() {
                return None;
            }
            return Some(Self::Id(id.to_string()));
        }

        if let Some(hash) = trimmed.strip_prefix('@') {
            if hash.is_empty() {
                return None;
            }
            return Some(Self::Hash(hash.to_string()));
        }

        // Path format: /Heading1/Heading2 or Heading1/Heading2
        let path_str = trimmed.strip_prefix('/').unwrap_or(trimmed);
        let components: Vec<String> = path_str
            .split('/')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if components.is_empty() {
            return None;
        }

        Some(Self::Path(components))
    }

    /// Format address as a human-readable string.
    #[must_use]
    pub fn to_display_string(&self) -> String {
        match self {
            Self::Id(id) => format!("#{id}"),
            Self::Path(components) => format!("/{}", components.join("/")),
            Self::Hash(hash) => format!("@{hash}"),
            Self::Block {
                section_path,
                block_addr,
            } => {
                format!(
                    "/{}{}",
                    section_path.join("/"),
                    block_addr.to_path_component()
                )
            }
        }
    }
}

/// Resolution result containing the found node and any path migration info.
#[derive(Debug, Clone)]
pub struct ResolvedNode {
    /// The resolved node.
    pub node: PageIndexNode,
    /// Document ID containing the node.
    pub doc_id: String,
    /// Whether the address was found via a different addressing mode.
    /// E.g., ID not found, but found via path.
    pub migrated_from: Option<Address>,
}

/// Enhanced resolution result with detailed match information.
#[derive(Debug, Clone)]
pub struct EnhancedResolvedNode {
    /// The resolved node.
    pub node: PageIndexNode,
    /// Document ID containing the node.
    pub doc_id: String,
    /// Actual path matched (may differ from request if fuzzy).
    pub resolved_path: Vec<String>,
    /// Stable ID for future anchoring.
    pub resolved_id: Option<String>,
    /// How the match was found.
    pub match_type: MatchType,
    /// Fuzzy match score (1.0 = exact).
    pub similarity: f32,
}

/// Resolution mode for dual-index addressing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveMode {
    /// Exact ID lookup only (O(1)).
    Anchor,
    /// Fuzzy path discovery with path drift tolerance.
    Discover {
        /// Enable fuzzy matching.
        fuzzy: bool,
        /// Maximum number of results to return.
        max_results: usize,
    },
}

/// Resolve error types for dual-index addressing.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ResolveError {
    /// Address not found.
    #[error("address '{address}' not found in document '{doc_id}'")]
    NotFound {
        /// Requested address.
        address: String,
        /// Document ID.
        doc_id: String,
    },
    /// Unsupported address type for the given mode.
    #[error("unsupported address type for mode")]
    UnsupportedAddress,
}

/// Resolve a node using dual indices with enhanced match information.
///
/// This is the preferred resolution function for new code, providing:
/// - O(1) ID lookup via RegistryIndex
/// - Fuzzy path discovery via TopologyIndex
/// - Detailed match type and similarity scoring
///
/// # Arguments
///
/// * `registry` - The registry index for O(1) ID lookups
/// * `topology` - The topology index for path-based resolution
/// * `address` - The address to resolve
/// * `doc_id` - The document ID context
/// * `mode` - Resolution mode (Anchor or Discover)
///
/// # Returns
///
/// Enhanced resolution result with match metadata, or an error.
pub fn resolve_with_indices(
    registry: &RegistryIndex,
    topology: &TopologyIndex,
    address: &Address,
    doc_id: &str,
    mode: ResolveMode,
) -> Result<EnhancedResolvedNode, ResolveError> {
    match address {
        Address::Id(id) => {
            // Direct O(1) lookup from registry
            if let Some(indexed) = registry.get(id) {
                return Ok(EnhancedResolvedNode {
                    node: indexed.node.clone(),
                    doc_id: indexed.doc_id.clone(),
                    resolved_path: indexed.node.metadata.structural_path.clone(),
                    resolved_id: Some(id.clone()),
                    match_type: MatchType::Exact,
                    similarity: 1.0,
                });
            }
            // Fallback to topology hash lookup
            if let Some(entry) = topology.find_by_hash(id) {
                return Ok(EnhancedResolvedNode {
                    node: build_node_from_entry(entry),
                    doc_id: entry.doc_id.clone(),
                    resolved_path: entry.path.clone(),
                    resolved_id: entry.node_id.parse().ok(),
                    match_type: MatchType::HashFallback,
                    similarity: 0.8,
                });
            }
            Err(ResolveError::NotFound {
                address: address.to_display_string(),
                doc_id: doc_id.to_string(),
            })
        }

        Address::Path(components) => {
            match mode {
                ResolveMode::Anchor => {
                    // Exact path match only
                    if let Some(entry) = topology.exact_path(doc_id, components) {
                        return Ok(EnhancedResolvedNode {
                            node: build_node_from_entry(entry),
                            doc_id: entry.doc_id.clone(),
                            resolved_path: entry.path.clone(),
                            resolved_id: entry.node_id.parse().ok(),
                            match_type: MatchType::Exact,
                            similarity: 1.0,
                        });
                    }
                    // Try case-insensitive match
                    if let Some(path_match) = topology.path_case_insensitive(doc_id, components) {
                        return Ok(EnhancedResolvedNode {
                            node: build_node_from_entry(&path_match.entry),
                            doc_id: path_match.doc_id,
                            resolved_path: path_match.path,
                            resolved_id: path_match.entry.node_id.parse().ok(),
                            match_type: path_match.match_type,
                            similarity: path_match.similarity_score,
                        });
                    }
                    Err(ResolveError::NotFound {
                        address: address.to_display_string(),
                        doc_id: doc_id.to_string(),
                    })
                }
                ResolveMode::Discover { fuzzy: true, max_results } => {
                    // Fuzzy matching with path drift
                    let query = components.join("/");
                    let matches = topology.fuzzy_resolve(&query, max_results);
                    if let Some(best_match) = matches.first() {
                        return Ok(EnhancedResolvedNode {
                            node: build_node_from_entry(&best_match.entry),
                            doc_id: best_match.doc_id.clone(),
                            resolved_path: best_match.path.clone(),
                            resolved_id: best_match.entry.node_id.parse().ok(),
                            match_type: best_match.match_type,
                            similarity: best_match.similarity_score,
                        });
                    }
                    Err(ResolveError::NotFound {
                        address: address.to_display_string(),
                        doc_id: doc_id.to_string(),
                    })
                }
                ResolveMode::Discover { fuzzy: false, .. } => {
                    // Non-fuzzy discover mode - same as anchor
                    if let Some(entry) = topology.exact_path(doc_id, components) {
                        return Ok(EnhancedResolvedNode {
                            node: build_node_from_entry(entry),
                            doc_id: entry.doc_id.clone(),
                            resolved_path: entry.path.clone(),
                            resolved_id: entry.node_id.parse().ok(),
                            match_type: MatchType::Exact,
                            similarity: 1.0,
                        });
                    }
                    Err(ResolveError::NotFound {
                        address: address.to_display_string(),
                        doc_id: doc_id.to_string(),
                    })
                }
            }
        }

        Address::Hash(hash) => {
            // Self-healing via content hash
            if let Some(entry) = topology.find_by_hash(hash) {
                return Ok(EnhancedResolvedNode {
                    node: build_node_from_entry(entry),
                    doc_id: entry.doc_id.clone(),
                    resolved_path: entry.path.clone(),
                    resolved_id: entry.node_id.parse().ok(),
                    match_type: MatchType::HashFallback,
                    similarity: 0.9,
                });
            }
            Err(ResolveError::NotFound {
                address: address.to_display_string(),
                doc_id: doc_id.to_string(),
            })
        }

        Address::Block { section_path, block_addr: _ } => {
            // Resolve section first using topology
            if let Some(entry) = topology.exact_path(doc_id, section_path) {
                // Note: Block resolution requires the full node data
                // For now, return the section with a note about block addressing
                return Ok(EnhancedResolvedNode {
                    node: build_node_from_entry(entry),
                    doc_id: entry.doc_id.clone(),
                    resolved_path: entry.path.clone(),
                    resolved_id: entry.node_id.parse().ok(),
                    match_type: MatchType::Exact,
                    similarity: 1.0,
                });
            }
            Err(ResolveError::NotFound {
                address: address.to_display_string(),
                doc_id: doc_id.to_string(),
            })
        }
    }
}

/// Build a PageIndexNode from a PathEntry.
///
/// This creates a minimal node with the essential information from the entry.
/// For full node data, use the tree-based resolution.
fn build_node_from_entry(entry: &PathEntry) -> PageIndexNode {
    use std::sync::Arc;
    PageIndexNode {
        node_id: entry.node_id.clone(),
        parent_id: None,
        title: entry.title.clone(),
        level: entry.level,
        text: Arc::from(""),
        summary: None,
        children: Vec::new(),
        blocks: Vec::new(),
        metadata: crate::link_graph::PageIndexMeta {
            line_range: (0, 0),
            byte_range: None,
            structural_path: entry.path.clone(),
            content_hash: entry.content_hash.clone(),
            attributes: std::collections::HashMap::new(),
            token_count: 0,
            is_thinned: false,
            logbook: Vec::new(),
        },
    }
}

/// Resolve a node within a document using the Triple-A protocol.
///
/// Attempts resolution in order: ID → Path → Hash.
/// Returns `None` if no resolution succeeds.
#[must_use]
pub fn resolve_node(
    trees: &std::collections::HashMap<String, Vec<PageIndexNode>>,
    address: &Address,
    doc_id: &str,
) -> Option<ResolvedNode> {
    let nodes = trees.get(doc_id)?;

    match address {
        Address::Id(id) => {
            // Try exact ID match
            if let Some(node) = find_by_id(nodes, id) {
                return Some(ResolvedNode {
                    node,
                    doc_id: doc_id.to_string(),
                    migrated_from: None,
                });
            }

            // Fallback to path resolution
            if let Some(node) = find_by_path_from_id(nodes, id) {
                return Some(ResolvedNode {
                    node,
                    doc_id: doc_id.to_string(),
                    migrated_from: Some(address.clone()),
                });
            }

            // Fallback to hash resolution
            if let Some(node) = find_by_hash_from_id(nodes, id) {
                return Some(ResolvedNode {
                    node,
                    doc_id: doc_id.to_string(),
                    migrated_from: Some(address.clone()),
                });
            }

            None
        }

        Address::Path(components) => {
            // Try path match
            if let Some(node) = find_by_path(nodes, components) {
                return Some(ResolvedNode {
                    node,
                    doc_id: doc_id.to_string(),
                    migrated_from: None,
                });
            }

            // Fallback to hash (path may have changed, but content same)
            if let Some(node) = find_by_hash_from_path(nodes, components) {
                return Some(ResolvedNode {
                    node,
                    doc_id: doc_id.to_string(),
                    migrated_from: Some(address.clone()),
                });
            }

            None
        }

        Address::Hash(hash) => {
            // Direct hash lookup
            find_by_hash(nodes, hash).map(|node| ResolvedNode {
                node,
                doc_id: doc_id.to_string(),
                migrated_from: None,
            })
        }

        Address::Block {
            section_path,
            block_addr,
        } => {
            // Resolve section first, then find block within
            if let Some(node) = find_by_path(nodes, section_path) {
                // Find the specific block within this node
                if find_block_in_node(&node, block_addr).is_some() {
                    return Some(ResolvedNode {
                        node,
                        doc_id: doc_id.to_string(),
                        migrated_from: None,
                    });
                }
            }
            None
        }
    }
}

/// Find a node by explicit `:ID:` attribute.
fn find_by_id(nodes: &[PageIndexNode], id: &str) -> Option<PageIndexNode> {
    for node in nodes {
        if node.metadata.attributes.get("ID").map(|s| s.as_str()) == Some(id) {
            return Some(node.clone());
        }
        // Recurse into children
        if let Some(found) = find_by_id(&node.children, id) {
            return Some(found);
        }
    }
    None
}

/// Find a node by structural path.
fn find_by_path(nodes: &[PageIndexNode], components: &[String]) -> Option<PageIndexNode> {
    if components.is_empty() {
        return None;
    }

    for node in nodes {
        // Check if this node's structural path matches
        if node.metadata.structural_path.as_slice() == components {
            return Some(node.clone());
        }

        // Check title match for first component
        if node.title == components[0] {
            if components.len() == 1 {
                return Some(node.clone());
            }
            // Recurse into children for remaining components
            if let Some(found) = find_by_path(&node.children, &components[1..]) {
                return Some(found);
            }
        }

        // Always recurse into children
        if let Some(found) = find_by_path(&node.children, components) {
            return Some(found);
        }
    }
    None
}

/// Find a node by content hash.
fn find_by_hash(nodes: &[PageIndexNode], hash: &str) -> Option<PageIndexNode> {
    for node in nodes {
        if node.metadata.content_hash.as_deref() == Some(hash) {
            return Some(node.clone());
        }
        if let Some(found) = find_by_hash(&node.children, hash) {
            return Some(found);
        }
    }
    None
}

/// Fallback: try to find by path when ID lookup failed.
fn find_by_path_from_id(_nodes: &[PageIndexNode], _id: &str) -> Option<PageIndexNode> {
    // TODO: Could parse ID as path components if it looks like a path
    None
}

/// Fallback: try to find by hash when ID lookup failed.
fn find_by_hash_from_id(_nodes: &[PageIndexNode], _id: &str) -> Option<PageIndexNode> {
    // ID is not a hash, no fallback
    None
}

/// Fallback: try to find by hash when path lookup failed.
fn find_by_hash_from_path(
    _nodes: &[PageIndexNode],
    _components: &[String],
) -> Option<PageIndexNode> {
    // TODO: Could compute hash from path-based content expectation
    None
}

/// Find a specific block within a node by block address.
///
/// Searches through the node's `blocks` vector to find a block matching
/// the specified kind and index.
fn find_block_in_node<'a>(
    node: &'a PageIndexNode,
    block_addr: &BlockAddress,
) -> Option<&'a crate::link_graph::models::MarkdownBlock> {
    let matching_blocks: Vec<_> = node
        .blocks
        .iter()
        .filter(|block| block.matches_kind(&block_addr.kind))
        .collect();

    matching_blocks.get(block_addr.index).copied()
}

/// Build a reverse index from ID to (doc_id, node).
#[must_use]
pub fn build_id_index(
    trees: &std::collections::HashMap<String, Vec<PageIndexNode>>,
) -> std::collections::HashMap<String, (String, String)> {
    let mut index = std::collections::HashMap::new();
    for (doc_id, nodes) in trees {
        collect_ids(nodes, doc_id, &mut index);
    }
    index
}

fn collect_ids(
    nodes: &[PageIndexNode],
    doc_id: &str,
    index: &mut std::collections::HashMap<String, (String, String)>,
) {
    for node in nodes {
        if let Some(id) = node.metadata.attributes.get("ID") {
            index.insert(id.clone(), (doc_id.to_string(), node.node_id.clone()));
        }
        collect_ids(&node.children, doc_id, index);
    }
}

/// Build a reverse index from content hash to (doc_id, node_id).
#[must_use]
pub fn build_hash_index(
    trees: &std::collections::HashMap<String, Vec<PageIndexNode>>,
) -> std::collections::HashMap<String, (String, String)> {
    let mut index = std::collections::HashMap::new();
    for (doc_id, nodes) in trees {
        collect_hashes(nodes, doc_id, &mut index);
    }
    index
}

fn collect_hashes(
    nodes: &[PageIndexNode],
    doc_id: &str,
    index: &mut std::collections::HashMap<String, (String, String)>,
) {
    for node in nodes {
        if let Some(hash) = &node.metadata.content_hash {
            index.insert(hash.clone(), (doc_id.to_string(), node.node_id.clone()));
        }
        collect_hashes(&node.children, doc_id, index);
    }
}

// ============================================================================
// Atomic Modification Interface
// ============================================================================

/// Result of a content modification operation.
#[derive(Debug, Clone)]
pub struct ModificationResult {
    /// The new content after modification.
    pub new_content: String,
    /// Number of bytes added (positive) or removed (negative).
    pub byte_delta: i64,
    /// Number of lines added (positive) or removed (negative).
    pub line_delta: i64,
    /// The new content hash after modification.
    pub new_hash: String,
}

/// Error during content modification.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ModificationError {
    /// Byte range is out of bounds.
    #[error("byte range {start:?}-{end:?} out of bounds (content length: {content_len})")]
    ByteRangeOutOfBounds {
        /// Start byte offset.
        start: usize,
        /// End byte offset.
        end: usize,
        /// Total content length.
        content_len: usize,
    },
    /// Content hash verification failed.
    #[error("content hash mismatch: expected {expected}, got {actual}")]
    HashMismatch {
        /// Expected hash value.
        expected: String,
        /// Actual hash value.
        actual: String,
    },
    /// Byte range not available.
    #[error("byte range not available for node")]
    NoByteRange,
}

/// Replace content at a specific byte range.
///
/// This is the core primitive for atomic section modifications.
/// It replaces the content between `byte_start` and `byte_end` with `new_text`.
///
/// # Arguments
///
/// * `content` - The original document content
/// * `byte_start` - Start byte offset (inclusive)
/// * `byte_end` - End byte offset (exclusive)
/// * `new_text` - The replacement text
/// * `expected_hash` - Optional content hash to verify before modification
///
/// # Returns
///
/// The modification result with new content and deltas, or an error.
///
/// # Example
///
/// ```ignore
/// let result = replace_byte_range(
///     &doc_content,
///     100,
///     200,
///     "new section content",
///     Some("abc123"),
/// )?;
/// ```
pub fn replace_byte_range(
    content: &str,
    byte_start: usize,
    byte_end: usize,
    new_text: &str,
    expected_hash: Option<&str>,
) -> Result<ModificationResult, ModificationError> {
    let content_bytes = content.as_bytes();
    let content_len = content_bytes.len();

    // Validate byte range
    if byte_start > content_len || byte_end > content_len || byte_start > byte_end {
        return Err(ModificationError::ByteRangeOutOfBounds {
            start: byte_start,
            end: byte_end,
            content_len,
        });
    }

    // Verify content hash if provided
    if let Some(expected) = expected_hash {
        let old_slice = &content[byte_start..byte_end];
        let actual = compute_hash(old_slice);
        if actual != expected {
            return Err(ModificationError::HashMismatch {
                expected: expected.to_string(),
                actual,
            });
        }
    }

    // Calculate deltas
    let old_len = byte_end - byte_start;
    let new_len = new_text.len();
    let byte_delta = new_len as i64 - old_len as i64;

    // Count lines
    let old_lines = content[byte_start..byte_end].lines().count();
    let new_lines = new_text.lines().count();
    let line_delta = new_lines as i64 - old_lines as i64;

    // Build new content - use saturating arithmetic to handle negative deltas
    let new_capacity = if byte_delta >= 0 {
        content_len + byte_delta as usize
    } else {
        content_len.saturating_sub((-byte_delta) as usize)
    };
    let mut new_content = String::with_capacity(new_capacity);
    new_content.push_str(&content[..byte_start]);
    new_content.push_str(new_text);
    new_content.push_str(&content[byte_end..]);

    // Compute new hash for the replaced section
    let new_hash = compute_hash(new_text);

    Ok(ModificationResult {
        new_content,
        byte_delta,
        line_delta,
        new_hash,
    })
}

/// Update a section's content using its byte range.
///
/// This function provides a higher-level interface for section updates.
/// It handles the byte range extraction and verification.
///
/// # Arguments
///
/// * `content` - The full document content
/// * `node` - The page index node with byte_range metadata
/// * `new_text` - The new section content
///
/// # Returns
///
/// The modification result or an error if the node has no byte range.
pub fn update_section_content(
    content: &str,
    node: &PageIndexNode,
    new_text: &str,
) -> Result<ModificationResult, ModificationError> {
    let (byte_start, byte_end) = node
        .metadata
        .byte_range
        .ok_or(ModificationError::NoByteRange)?;

    replace_byte_range(
        content,
        byte_start,
        byte_end,
        new_text,
        node.metadata.content_hash.as_deref(),
    )
}

/// Compute Blake3 hash for content verification.
fn compute_hash(text: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(text.as_bytes());
    let hash = hasher.finalize();
    hash.to_hex()[..16].to_string()
}

/// Calculate new line positions after a modification.
///
/// Given the original line range and the modification deltas,
/// compute the new line range for the modified section.
#[must_use]
pub fn adjust_line_range(
    original_start: usize,
    original_end: usize,
    line_delta: i64,
    modification_line: usize,
) -> (usize, usize) {
    if modification_line <= original_start {
        // Modification is before this section - shift both
        let shift = line_delta as isize;
        (
            (original_start as isize + shift).max(1) as usize,
            (original_end as isize + shift).max(1) as usize,
        )
    } else if modification_line <= original_end {
        // Modification is within this section - adjust end only
        (
            original_start,
            (original_end as i64 + line_delta).max(1) as usize,
        )
    } else {
        // Modification is after this section - no change
        (original_start, original_end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_address_parse_id() {
        let addr = Address::parse("#arch-v1");
        assert_eq!(addr, Some(Address::Id("arch-v1".to_string())));
    }

    #[test]
    fn test_address_parse_path() {
        let addr = Address::parse("/Architecture/Storage");
        assert_eq!(
            addr,
            Some(Address::Path(vec![
                "Architecture".to_string(),
                "Storage".to_string()
            ]))
        );
    }

    #[test]
    fn test_address_parse_hash() {
        let addr = Address::parse("@a1b2c3d4e5f6");
        assert_eq!(addr, Some(Address::Hash("a1b2c3d4e5f6".to_string())));
    }

    #[test]
    fn test_address_parse_empty() {
        assert!(Address::parse("").is_none());
        assert!(Address::parse("#").is_none());
        assert!(Address::parse("@").is_none());
    }

    #[test]
    fn test_address_to_display_string() {
        assert_eq!(Address::id("test").to_display_string(), "#test");
        assert_eq!(Address::path(vec!["A", "B"]).to_display_string(), "/A/B");
        assert_eq!(Address::hash("abc123").to_display_string(), "@abc123");
    }

    #[test]
    fn test_find_by_id() {
        let node = PageIndexNode {
            node_id: "doc#section".to_string(),
            parent_id: None,
            title: "Section".to_string(),
            level: 1,
            text: std::sync::Arc::from("content"),
            summary: None,
            children: Vec::new(),
            blocks: Vec::new(),
            metadata: crate::link_graph::models::PageIndexMeta {
                line_range: (1, 10),
                byte_range: Some((0, 100)),
                structural_path: vec!["Section".to_string()],
                content_hash: Some("abc123".to_string()),
                attributes: {
                    let mut attrs = HashMap::new();
                    attrs.insert("ID".to_string(), "my-section".to_string());
                    attrs
                },
                token_count: 10,
                is_thinned: false,
                logbook: Vec::new(),
            },
        };

        let found = find_by_id(&[node.clone()], "my-section");
        assert!(found.is_some());

        let not_found = find_by_id(&[node], "other-id");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_find_by_path() {
        let node = PageIndexNode {
            node_id: "doc#storage".to_string(),
            parent_id: None,
            title: "Storage".to_string(),
            level: 2,
            text: std::sync::Arc::from("content"),
            summary: None,
            children: Vec::new(),
            blocks: Vec::new(),
            metadata: crate::link_graph::models::PageIndexMeta {
                line_range: (1, 10),
                byte_range: Some((0, 100)),
                structural_path: vec!["Architecture".to_string(), "Storage".to_string()],
                content_hash: None,
                attributes: HashMap::new(),
                token_count: 10,
                is_thinned: false,
                logbook: Vec::new(),
            },
        };

        let found = find_by_path(
            &[node.clone()],
            &["Architecture".to_string(), "Storage".to_string()],
        );
        assert!(found.is_some());

        let found_by_title = find_by_path(&[node], &["Storage".to_string()]);
        assert!(found_by_title.is_some());
    }

    #[test]
    fn test_find_by_hash() {
        let node = PageIndexNode {
            node_id: "doc#section".to_string(),
            parent_id: None,
            title: "Section".to_string(),
            level: 1,
            text: std::sync::Arc::from("content"),
            summary: None,
            children: Vec::new(),
            blocks: Vec::new(),
            metadata: crate::link_graph::models::PageIndexMeta {
                line_range: (1, 10),
                byte_range: Some((0, 100)),
                structural_path: vec![],
                content_hash: Some("def456".to_string()),
                attributes: HashMap::new(),
                token_count: 10,
                is_thinned: false,
                logbook: Vec::new(),
            },
        };

        let found = find_by_hash(&[node.clone()], "def456");
        assert!(found.is_some());

        let not_found = find_by_hash(&[node], "other-hash");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_replace_byte_range_basic() {
        let content = "Hello, world!";
        let result = replace_byte_range(content, 7, 12, "Rust", None).unwrap();
        assert_eq!(result.new_content, "Hello, Rust!");
        assert_eq!(result.byte_delta, -1); // "world" (5) -> "Rust" (4)
        assert_eq!(result.line_delta, 0);
    }

    #[test]
    fn test_replace_byte_range_with_hash_verification() {
        let content = "Hello, world!";
        // Compute hash of "world"
        let hash = compute_hash("world");
        let result = replace_byte_range(content, 7, 12, "Rust", Some(&hash)).unwrap();
        assert_eq!(result.new_content, "Hello, Rust!");
    }

    #[test]
    fn test_replace_byte_range_hash_mismatch() {
        let content = "Hello, world!";
        let result = replace_byte_range(content, 7, 12, "Rust", Some("wronghash"));
        assert!(matches!(
            result,
            Err(ModificationError::HashMismatch { .. })
        ));
    }

    #[test]
    fn test_replace_byte_range_out_of_bounds() {
        let content = "Hello";
        let result = replace_byte_range(content, 0, 100, "test", None);
        assert!(matches!(
            result,
            Err(ModificationError::ByteRangeOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_update_section_content() {
        let node = PageIndexNode {
            node_id: "doc#section".to_string(),
            parent_id: None,
            title: "Section".to_string(),
            level: 1,
            text: std::sync::Arc::from("old content"),
            summary: None,
            children: Vec::new(),
            blocks: Vec::new(),
            metadata: crate::link_graph::models::PageIndexMeta {
                line_range: (1, 5),
                byte_range: Some((0, 11)),
                structural_path: vec![],
                content_hash: Some(compute_hash("old content")),
                attributes: HashMap::new(),
                token_count: 2,
                is_thinned: false,
            },
        };

        let doc_content = "old content here";
        let result = update_section_content(doc_content, &node, "new content").unwrap();
        assert_eq!(result.new_content, "new content here");
        assert_eq!(result.byte_delta, 0); // "old content" (11) -> "new content" (11) = 0
    }

    #[test]
    fn test_adjust_line_range_before() {
        // Modification before the section
        let (start, end) = adjust_line_range(10, 20, 5, 5);
        assert_eq!(start, 15);
        assert_eq!(end, 25);
    }

    #[test]
    fn test_adjust_line_range_within() {
        // Modification within the section
        let (start, end) = adjust_line_range(10, 20, 3, 15);
        assert_eq!(start, 10);
        assert_eq!(end, 23);
    }

    #[test]
    fn test_adjust_line_range_after() {
        // Modification after the section
        let (start, end) = adjust_line_range(10, 20, 5, 30);
        assert_eq!(start, 10);
        assert_eq!(end, 20);
    }

    #[test]
    fn test_compute_hash_consistency() {
        let hash1 = compute_hash("test content");
        let hash2 = compute_hash("test content");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16); // Blake3 truncated to 16 hex chars
    }
}
