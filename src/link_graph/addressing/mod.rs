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
//! - **`RegistryIndex`**: O(1) lookup for stable ID anchoring (used by agents/systems)
//! - **`TopologyIndex`**: Fuzzy path discovery for human-friendly navigation
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

use std::collections::HashMap;
use std::hash::BuildHasher;

use crate::link_graph::models::{BlockAddress, PageIndexNode};

pub use registry::{IdCollision, IndexedNode, RegistryBuildResult, RegistryIndex};
pub use skeleton_rerank::{SkeletonRerankOptions, SkeletonValidatedHit, skeleton_rerank};
pub use structural_transaction::{
    StructuralTransaction, StructuralTransactionCoordinator, StructureUpdateSignal,
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
/// - O(1) ID lookup via `RegistryIndex`
/// - Fuzzy path discovery via `TopologyIndex`
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
///
/// # Errors
///
/// Returns [`ResolveError::NotFound`] when the requested address cannot be resolved from either
/// the registry index or the topology index for the provided document.
pub fn resolve_with_indices(
    registry: &RegistryIndex,
    topology: &TopologyIndex,
    address: &Address,
    doc_id: &str,
    mode: ResolveMode,
) -> Result<EnhancedResolvedNode, ResolveError> {
    match address {
        Address::Id(id) => resolve_id_with_indices(registry, topology, address, doc_id, id),
        Address::Path(components) => {
            resolve_path_with_indices(topology, address, doc_id, components, mode)
        }
        Address::Hash(hash) => resolve_hash_with_indices(topology, address, doc_id, hash),
        Address::Block {
            section_path,
            block_addr: _,
        } => resolve_block_with_indices(topology, address, doc_id, section_path),
    }
}

fn resolve_id_with_indices(
    registry: &RegistryIndex,
    topology: &TopologyIndex,
    address: &Address,
    doc_id: &str,
    id: &str,
) -> Result<EnhancedResolvedNode, ResolveError> {
    if let Some(indexed) = registry.get(id) {
        return Ok(build_registry_resolution(
            indexed,
            Some(id.to_string()),
            MatchType::Exact,
            1.0,
        ));
    }

    topology
        .find_by_hash(id)
        .map(|entry| build_entry_resolution(entry, MatchType::HashFallback, 0.8))
        .ok_or_else(|| not_found(address, doc_id))
}

fn resolve_path_with_indices(
    topology: &TopologyIndex,
    address: &Address,
    doc_id: &str,
    components: &[String],
    mode: ResolveMode,
) -> Result<EnhancedResolvedNode, ResolveError> {
    match mode {
        ResolveMode::Anchor => topology
            .exact_path(doc_id, components)
            .map(|entry| build_entry_resolution(entry, MatchType::Exact, 1.0))
            .or_else(|| {
                topology
                    .path_case_insensitive(doc_id, components)
                    .map(build_path_match_resolution)
            })
            .ok_or_else(|| not_found(address, doc_id)),
        ResolveMode::Discover {
            fuzzy: true,
            max_results,
        } => topology
            .fuzzy_resolve(&components.join("/"), max_results)
            .into_iter()
            .next()
            .map(build_path_match_resolution)
            .ok_or_else(|| not_found(address, doc_id)),
        ResolveMode::Discover { fuzzy: false, .. } => topology
            .exact_path(doc_id, components)
            .map(|entry| build_entry_resolution(entry, MatchType::Exact, 1.0))
            .ok_or_else(|| not_found(address, doc_id)),
    }
}

fn resolve_hash_with_indices(
    topology: &TopologyIndex,
    address: &Address,
    doc_id: &str,
    hash: &str,
) -> Result<EnhancedResolvedNode, ResolveError> {
    topology
        .find_by_hash(hash)
        .map(|entry| build_entry_resolution(entry, MatchType::HashFallback, 0.9))
        .ok_or_else(|| not_found(address, doc_id))
}

fn resolve_block_with_indices(
    topology: &TopologyIndex,
    address: &Address,
    doc_id: &str,
    section_path: &[String],
) -> Result<EnhancedResolvedNode, ResolveError> {
    topology
        .exact_path(doc_id, section_path)
        .map(|entry| build_entry_resolution(entry, MatchType::Exact, 1.0))
        .ok_or_else(|| not_found(address, doc_id))
}

fn build_registry_resolution(
    indexed: &IndexedNode,
    resolved_id: Option<String>,
    match_type: MatchType,
    similarity: f32,
) -> EnhancedResolvedNode {
    EnhancedResolvedNode {
        node: indexed.node.clone(),
        doc_id: indexed.doc_id.clone(),
        resolved_path: indexed.node.metadata.structural_path.clone(),
        resolved_id,
        match_type,
        similarity,
    }
}

fn build_entry_resolution(
    entry: &PathEntry,
    match_type: MatchType,
    similarity: f32,
) -> EnhancedResolvedNode {
    EnhancedResolvedNode {
        node: build_node_from_entry(entry),
        doc_id: entry.doc_id.clone(),
        resolved_path: entry.path.clone(),
        resolved_id: Some(entry.node_id.clone()),
        match_type,
        similarity,
    }
}

fn build_path_match_resolution(path_match: PathMatch) -> EnhancedResolvedNode {
    EnhancedResolvedNode {
        node: build_node_from_entry(&path_match.entry),
        doc_id: path_match.doc_id,
        resolved_path: path_match.path,
        resolved_id: Some(path_match.entry.node_id),
        match_type: path_match.match_type,
        similarity: path_match.similarity_score,
    }
}

fn not_found(address: &Address, doc_id: &str) -> ResolveError {
    ResolveError::NotFound {
        address: address.to_display_string(),
        doc_id: doc_id.to_string(),
    }
}

/// Build a `PageIndexNode` from a `PathEntry`.
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
            observations: Vec::new(),
        },
    }
}

/// Resolve a node within a document using the Triple-A protocol.
///
/// Attempts resolution in order: ID → Path → Hash.
/// Returns `None` if no resolution succeeds.
#[must_use]
pub fn resolve_node<S>(
    trees: &HashMap<String, Vec<PageIndexNode>, S>,
    address: &Address,
    doc_id: &str,
) -> Option<ResolvedNode>
where
    S: BuildHasher,
{
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
        if node
            .metadata
            .attributes
            .get("ID")
            .map(std::string::String::as_str)
            == Some(id)
        {
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

/// Build a reverse index from ID to (`doc_id`, node).
#[must_use]
pub fn build_id_index<S>(
    trees: &HashMap<String, Vec<PageIndexNode>, S>,
) -> HashMap<String, (String, String)>
where
    S: BuildHasher,
{
    let mut index = HashMap::new();
    for (doc_id, nodes) in trees {
        collect_ids(nodes, doc_id, &mut index);
    }
    index
}

fn collect_ids(
    nodes: &[PageIndexNode],
    doc_id: &str,
    index: &mut HashMap<String, (String, String)>,
) {
    for node in nodes {
        if let Some(id) = node.metadata.attributes.get("ID") {
            index.insert(id.clone(), (doc_id.to_string(), node.node_id.clone()));
        }
        collect_ids(&node.children, doc_id, index);
    }
}

/// Build a reverse index from content hash to (`doc_id`, `node_id`).
#[must_use]
pub fn build_hash_index<S>(
    trees: &HashMap<String, Vec<PageIndexNode>, S>,
) -> HashMap<String, (String, String)>
where
    S: BuildHasher,
{
    let mut index = HashMap::new();
    for (doc_id, nodes) in trees {
        collect_hashes(nodes, doc_id, &mut index);
    }
    index
}

fn collect_hashes(
    nodes: &[PageIndexNode],
    doc_id: &str,
    index: &mut HashMap<String, (String, String)>,
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
    /// Signed delta exceeded the supported `i64` range.
    #[error("signed delta overflow while comparing lengths {lhs} and {rhs}")]
    DeltaOverflow {
        /// Left-hand length operand.
        lhs: usize,
        /// Right-hand length operand.
        rhs: usize,
    },
    /// Adjusting a `usize` position by a signed delta overflowed.
    #[error("range adjustment overflow for base {base} with delta {delta}")]
    RangeAdjustmentOverflow {
        /// Original base value.
        base: usize,
        /// Signed delta to apply.
        delta: i64,
    },
}

fn signed_len_delta(lhs: usize, rhs: usize) -> Result<i64, ModificationError> {
    let lhs_i64 = i64::try_from(lhs).map_err(|_| ModificationError::DeltaOverflow { lhs, rhs })?;
    let rhs_i64 = i64::try_from(rhs).map_err(|_| ModificationError::DeltaOverflow { lhs, rhs })?;
    lhs_i64
        .checked_sub(rhs_i64)
        .ok_or(ModificationError::DeltaOverflow { lhs, rhs })
}

fn apply_signed_delta(base: usize, delta: i64) -> Result<usize, ModificationError> {
    if delta >= 0 {
        let magnitude = usize::try_from(delta)
            .map_err(|_| ModificationError::RangeAdjustmentOverflow { base, delta })?;
        base.checked_add(magnitude)
            .ok_or(ModificationError::RangeAdjustmentOverflow { base, delta })
    } else {
        let magnitude = match usize::try_from(delta.unsigned_abs()) {
            Ok(magnitude) => magnitude,
            Err(_) => usize::MAX,
        };
        Ok(base.saturating_sub(magnitude))
    }
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
/// # Errors
///
/// Returns [`ModificationError::ByteRangeOutOfBounds`] when the byte range is invalid,
/// [`ModificationError::HashMismatch`] when the provided hash does not match the target slice,
/// and overflow variants when the signed delta cannot be represented safely.
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
    let byte_delta = signed_len_delta(new_len, old_len)?;

    // Count lines
    let old_lines = content[byte_start..byte_end].lines().count();
    let new_lines = new_text.lines().count();
    let line_delta = signed_len_delta(new_lines, old_lines)?;

    // Build new content - use saturating arithmetic to handle negative deltas
    let new_capacity = apply_signed_delta(content_len, byte_delta)?;
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
/// * `node` - The page index node with `byte_range` metadata
/// * `new_text` - The new section content
///
/// # Returns
///
/// The modification result or an error if the node has no byte range.
///
/// # Errors
///
/// Returns [`ModificationError::NoByteRange`] when the node lacks byte metadata and forwards any
/// replacement failure from [`replace_byte_range`].
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
        (
            apply_signed_delta(original_start, line_delta)
                .unwrap_or(1)
                .max(1),
            apply_signed_delta(original_end, line_delta)
                .unwrap_or(1)
                .max(1),
        )
    } else if modification_line <= original_end {
        // Modification is within this section - adjust end only
        (
            original_start,
            apply_signed_delta(original_end, line_delta)
                .unwrap_or(1)
                .max(1),
        )
    } else {
        // Modification is after this section - no change
        (original_start, original_end)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/link_graph/addressing/mod.rs"]
mod tests;
