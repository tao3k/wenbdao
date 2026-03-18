//! Core index build + query algorithms for markdown link graph.

use super::models::{
    LinkGraphAttachment, LinkGraphDirection, LinkGraphDocument, LinkGraphEdgeType, LinkGraphHit,
    LinkGraphLinkFilter, LinkGraphMatchStrategy, LinkGraphMetadata, LinkGraphNeighbor,
    LinkGraphPassage, LinkGraphPprSubgraphMode, LinkGraphPromotedOverlayTelemetry,
    LinkGraphRelatedFilter, LinkGraphRelatedPprDiagnostics, LinkGraphRelatedPprOptions,
    LinkGraphScope, LinkGraphSearchFilters, LinkGraphSearchOptions, LinkGraphSortField,
    LinkGraphSortOrder, LinkGraphSortTerm, LinkGraphStats, PageIndexNode,
};
use super::query::parse_search_query;
use serde::{Deserialize, Serialize};
mod agentic_expansion;
mod agentic_overlay;
mod build;
mod ids;
mod page_indices;
mod passages;
mod ppr;
mod rank;
mod scoring;
pub(crate) mod search;
mod semantic_documents;
mod shared;
mod traversal;

pub use search::quantum_fusion::orchestrate::QuantumContextBuildError;
pub use search::quantum_fusion::semantic_ignition::{
    QuantumSemanticIgnition, QuantumSemanticIgnitionError, QuantumSemanticIgnitionFuture,
};

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const INCOMING_RANK_FACTOR: f64 = 2.0;
const OUTGOING_RANK_FACTOR: f64 = 1.0;
const MAX_GRAPH_RANK_BOOST: f64 = 0.35;
const WEIGHT_FTS_LEXICAL: f64 = 0.62;
const WEIGHT_FTS_SECTION: f64 = 0.23;
const WEIGHT_FTS_PATH: f64 = 0.15;
const WEIGHT_PATH_FUZZY_PATH: f64 = 0.70;
const WEIGHT_PATH_FUZZY_SECTION: f64 = 0.30;
const INCREMENTAL_REBUILD_THRESHOLD: usize = 256;
const DEFAULT_PER_DOC_SECTION_CAP: usize = 3;
const DEFAULT_MIN_SECTION_WORDS: usize = 24;
const SECTION_AGGREGATION_BETA: f64 = 0.15;

use scoring::{
    normalize_with_case, score_document, score_document_exact, score_document_regex,
    score_path_fields, section_tree_distance, token_match_ratio, tokenize,
};
use shared::{
    ScoredSearchRow, deterministic_random_key, doc_contains_phrase, doc_sort_key,
    normalize_path_filter, path_matches_filter,
};

/// A virtual node synthesized from collapsed dense clusters during knowledge distillation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkGraphVirtualNode {
    /// Synthesized identifier (e.g., "virtual:cluster:0:abc123").
    pub id: String,
    /// Original member node IDs that were collapsed.
    pub members: Vec<String>,
    /// Average saliency of collapsed nodes.
    pub avg_saliency: f64,
    /// Synthesized title.
    pub title: String,
    /// Internal edge count (edges between members).
    pub internal_edges: usize,
    /// Edge density within cluster (0.0-1.0).
    pub edge_density: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IndexedSection {
    pub(crate) heading_title: String,
    pub(crate) heading_path: String,
    pub(crate) heading_path_lower: String,
    pub(crate) heading_level: usize,
    pub(crate) line_start: usize,
    pub(crate) line_end: usize,
    pub(crate) byte_start: usize,
    pub(crate) byte_end: usize,
    pub(crate) section_text: String,
    pub(crate) section_text_lower: String,
    pub(crate) entities: Vec<String>,
    /// Property drawer attributes extracted from heading (e.g., :ID: arch-v1).
    pub(crate) attributes: std::collections::HashMap<String, String>,
    /// Execution log entries from :LOGBOOK: drawer (Blueprint v2.4).
    pub(crate) logbook: Vec<super::parser::LogbookEntry>,
    /// Code observations from :OBSERVE: property drawer (Blueprint v2.7).
    pub(crate) observations: Vec<super::parser::CodeObservation>,
}

impl IndexedSection {
    pub(crate) fn from_parsed(value: &super::parser::ParsedSection) -> Self {
        Self {
            heading_title: value.heading_title.clone(),
            heading_path: value.heading_path.clone(),
            heading_path_lower: value.heading_path_lower.clone(),
            heading_level: value.heading_level,
            line_start: value.line_start,
            line_end: value.line_end,
            byte_start: value.byte_start,
            byte_end: value.byte_end,
            section_text: value.section_text.clone(),
            section_text_lower: value.section_text_lower.clone(),
            entities: value.entities.clone(),
            attributes: value.attributes.clone(),
            logbook: value.logbook.clone(),
            observations: value.observations.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct SectionMatch {
    score: f64,
    heading_path: Option<String>,
    reason: &'static str,
}

#[derive(Debug, Clone)]
struct SectionCandidate {
    heading_path: String,
    score: f64,
    reason: &'static str,
}

/// Cache build metadata emitted by the Valkey-backed `LinkGraph` bootstrap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkGraphCacheBuildMeta {
    /// Cache backend name.
    pub backend: String,
    /// Cache status (`hit` or `miss`).
    pub status: String,
    /// Miss reason when status is `miss`.
    pub miss_reason: Option<String>,
    /// Cache schema version string.
    pub schema_version: String,
    /// Cache schema fingerprint (derived from schema JSON content).
    pub schema_fingerprint: String,
}

/// Reference to a code observation symbol within a document.
///
/// Used by the Symbol-to-Node Inverted Index for O(1) semantic change propagation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRef {
    /// Document ID where the symbol is observed.
    pub doc_id: String,
    /// Node ID within the document.
    pub node_id: String,
    /// The observation pattern containing this symbol.
    pub pattern: String,
    /// Language of the observation.
    pub language: String,
    /// Line number in the document.
    pub line_number: Option<usize>,
    /// Optional scope filter for the observation.
    pub scope: Option<String>,
}

/// Statistics about the symbol cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolCacheStats {
    /// Number of unique symbols in the cache.
    pub unique_symbols: usize,
    /// Total number of symbol-to-document references.
    pub total_references: usize,
}

/// Parent resolution for a page-index node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageIndexParent<'a> {
    /// The node exists and is a root node.
    Root,
    /// The node exists and is nested under the given parent node id.
    Parent(&'a str),
}

/// Fast in-memory markdown link graph index.
#[derive(Debug, Clone)]
pub struct LinkGraphIndex {
    root: PathBuf,
    include_dirs: Vec<String>,
    excluded_dirs: Vec<String>,
    docs_by_id: HashMap<String, LinkGraphDocument>,
    sections_by_doc: HashMap<String, Vec<IndexedSection>>,
    passages_by_id: HashMap<String, LinkGraphPassage>,
    attachments_by_doc: HashMap<String, Vec<LinkGraphAttachment>>,
    trees_by_doc: HashMap<String, Vec<PageIndexNode>>,
    /// Map page-index node ids to parent node ids (None for roots).
    node_parent_map: HashMap<String, Option<String>>,
    /// Explicit anchor registry (`doc_id#ID`) for fast semantic resolution.
    explicit_id_registry: HashMap<String, PageIndexNode>,
    alias_to_doc_id: HashMap<String, String>,
    outgoing: HashMap<String, HashSet<String>>,
    incoming: HashMap<String, HashSet<String>>,
    rank_by_id: HashMap<String, f64>,
    edge_count: usize,
    /// Virtual nodes created by knowledge distillation (collapsed dense clusters).
    virtual_nodes: HashMap<String, build::VirtualNode>,
    /// Symbol-to-Node Inverted Index for O(1) semantic change propagation.
    /// Maps symbol names extracted from :OBSERVE: patterns to their document locations.
    symbol_to_docs: HashMap<String, Vec<SymbolRef>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Refresh execution mode selected by `LinkGraph` incremental refresh logic.
pub enum LinkGraphRefreshMode {
    /// No-op (no changed paths provided).
    Noop,
    /// Apply incremental delta updates.
    Delta,
    /// Run full index rebuild.
    Full,
}

impl LinkGraphIndex {
    /// Default threshold where delta refresh switches to full rebuild.
    #[must_use]
    pub const fn incremental_rebuild_threshold() -> usize {
        INCREMENTAL_REBUILD_THRESHOLD
    }

    /// Notebook root used by this index.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Directories included in this index (from configuration).
    #[must_use]
    pub fn include_dirs(&self) -> &[String] {
        &self.include_dirs
    }

    #[allow(dead_code)]
    pub(in crate::link_graph::index) fn execute_direct_id_lookup(
        &self,
        direct_id: &str,
        _limit: usize,
        _options: &LinkGraphSearchOptions,
    ) -> Vec<LinkGraphHit> {
        let mut out = Vec::new();
        if let Some(doc_id) = self.resolve_doc_id(direct_id)
            && let Some(doc) = self.docs_by_id.get(doc_id)
        {
            out.push(LinkGraphHit {
                stem: doc.stem.clone(),
                title: doc.title.clone(),
                path: doc.path.clone(),
                doc_type: doc.doc_type.clone(),
                tags: doc.tags.clone(),
                score: 1.0,
                best_section: None,
                match_reason: Some("direct_id".to_string()),
            });
        }
        out
    }

    /// Resolve one document or anchor id into its semantic breadcrumb trail.
    #[must_use]
    pub fn page_index_semantic_path(&self, anchor_id: &str) -> Option<Vec<String>> {
        self.extract_lineage(anchor_id)
    }

    pub(crate) fn has_doc(&self, doc_id: &str) -> bool {
        self.docs_by_id.contains_key(doc_id)
    }

    pub(crate) fn get_doc(&self, doc_id: &str) -> Option<&LinkGraphDocument> {
        self.docs_by_id.get(doc_id)
    }

    pub(crate) fn get_tree(&self, doc_id: &str) -> Option<&Vec<PageIndexNode>> {
        self.trees_by_doc.get(doc_id)
    }

    pub(crate) fn get_node_parent_map(&self) -> &HashMap<String, Option<String>> {
        &self.node_parent_map
    }

    pub(crate) fn resolve_doc_id_pub(&self, stem_or_id: &str) -> Option<&str> {
        self.resolve_doc_id(stem_or_id)
    }

    /// Get document relative path by stem or ID.
    #[must_use]
    pub fn doc_path(&self, stem_or_id: &str) -> Option<&str> {
        let doc_id = self.resolve_doc_id(stem_or_id)?;
        self.docs_by_id.get(doc_id).map(|doc| doc.path.as_str())
    }

    /// Get document title by stem or ID.
    #[must_use]
    pub fn doc_title(&self, stem_or_id: &str) -> Option<&str> {
        let doc_id = self.resolve_doc_id(stem_or_id)?;
        self.docs_by_id.get(doc_id).map(|doc| doc.title.as_str())
    }

    /// Get all page index trees for Triple-A addressing.
    #[must_use]
    pub fn all_page_index_trees(&self) -> &HashMap<String, Vec<PageIndexNode>> {
        &self.trees_by_doc
    }

    // =========================================================================
    // Symbol-to-Node Inverted Index (Phase 6.3/6.4)
    // =========================================================================

    /// Look up documents containing a specific code symbol.
    ///
    /// This is the O(1) lookup for semantic change propagation.
    /// Given a symbol name (e.g., "`process_data`"), returns all documents
    /// with `:OBSERVE:` patterns that reference this symbol.
    #[must_use]
    pub fn lookup_symbol(&self, symbol: &str) -> Option<&[SymbolRef]> {
        self.symbol_to_docs.get(symbol).map(Vec::as_slice)
    }

    /// Get all symbols in the inverted index.
    pub fn all_symbols(&self) -> impl Iterator<Item = &String> {
        self.symbol_to_docs.keys()
    }

    /// Get the total number of indexed symbols.
    #[must_use]
    pub fn symbol_count(&self) -> usize {
        self.symbol_to_docs.len()
    }

    /// Check if any symbols are indexed.
    #[must_use]
    pub fn has_symbols(&self) -> bool {
        !self.symbol_to_docs.is_empty()
    }

    // =========================================================================
    // Phase 6.5: Incremental Symbol Cache Updates
    // =========================================================================

    /// Refresh the symbol cache for a single document.
    ///
    /// Call this when a document's `:OBSERVE:` patterns may have changed.
    /// This performs a targeted update without rebuilding the entire index.
    pub fn refresh_symbol_cache_for_doc(&mut self, doc_id: &str) {
        // First, remove existing entries for this document
        self.remove_symbol_refs_for_doc(doc_id);

        // Clone the tree to avoid borrow issues
        let tree_clone = self.trees_by_doc.get(doc_id).cloned();

        // Then, re-index if the document has a page index tree
        if let Some(tree) = tree_clone {
            self.index_symbols_from_tree_cloned(doc_id, &tree);
        }
    }

    /// Remove all symbol references for a document from the cache.
    fn remove_symbol_refs_for_doc(&mut self, doc_id: &str) {
        for refs in self.symbol_to_docs.values_mut() {
            refs.retain(|r| r.doc_id != doc_id);
        }
        // Clean up empty symbol entries
        self.symbol_to_docs.retain(|_, refs| !refs.is_empty());
    }

    /// Index symbols from a cloned page index tree.
    fn index_symbols_from_tree_cloned(&mut self, doc_id: &str, nodes: &[PageIndexNode]) {
        use crate::zhenfa_router::native::sentinel::extract_pattern_symbols;

        for node in nodes {
            for obs in &node.metadata.observations {
                let symbols = extract_pattern_symbols(&obs.pattern);
                for symbol in symbols {
                    let symbol_ref = SymbolRef {
                        doc_id: doc_id.to_string(),
                        node_id: node.node_id.clone(),
                        pattern: obs.pattern.clone(),
                        language: obs.language.clone(),
                        line_number: obs.line_number,
                        scope: obs.scope.clone(),
                    };
                    self.symbol_to_docs
                        .entry(symbol)
                        .or_default()
                        .push(symbol_ref);
                }
            }
            // Recurse into children
            self.index_symbols_from_tree_cloned(doc_id, &node.children);
        }
    }

    /// Get statistics about the symbol cache.
    #[must_use]
    pub fn symbol_cache_stats(&self) -> SymbolCacheStats {
        let total_refs: usize = self.symbol_to_docs.values().map(std::vec::Vec::len).sum();
        SymbolCacheStats {
            unique_symbols: self.symbol_to_docs.len(),
            total_references: total_refs,
        }
    }

    /// Check if a document has any indexed symbols.
    #[must_use]
    pub fn doc_has_symbols(&self, doc_id: &str) -> bool {
        self.symbol_to_docs
            .values()
            .any(|refs| refs.iter().any(|r| r.doc_id == doc_id))
    }

    /// Get all documents that have indexed symbols.
    #[must_use]
    pub fn docs_with_symbols(&self) -> Vec<&str> {
        let mut doc_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for refs in self.symbol_to_docs.values() {
            for r in refs {
                doc_ids.insert(&r.doc_id);
            }
        }
        doc_ids.into_iter().collect()
    }

    /// Get all virtual nodes created by knowledge distillation.
    ///
    /// Virtual nodes represent collapsed dense clusters of high-saliency nodes.
    /// They inherit edges from their member nodes and can be used for graph traversal.
    #[must_use]
    pub fn virtual_nodes(&self) -> Vec<LinkGraphVirtualNode> {
        self.virtual_nodes
            .values()
            .map(|vn| LinkGraphVirtualNode {
                id: vn.id.clone(),
                members: vn.members.clone(),
                avg_saliency: vn.avg_saliency,
                title: vn.title.clone(),
                internal_edges: vn.internal_edges,
                edge_density: vn.edge_density,
            })
            .collect()
    }

    /// Extract semantic intent targets for a document.
    #[must_use]
    pub fn intent_targets(&self, doc_id: &str) -> (Vec<String>, Vec<String>) {
        let Some(doc) = self.docs_by_id.get(doc_id) else {
            return (Vec::new(), Vec::new());
        };
        // This is a simplification, actual implementation might need more parsing
        (doc.tags.clone(), Vec::new())
    }

    /// Build a `RegistryIndex` for O(1) ID lookups.
    ///
    /// The registry index provides fast access to nodes with explicit `:ID:` attributes.
    #[must_use]
    pub fn build_registry_index(&self) -> super::addressing::RegistryIndex {
        super::addressing::RegistryIndex::build_from_trees(&self.trees_by_doc)
    }

    /// Build a `RegistryIndex` with collision detection.
    ///
    /// Returns both the registry index and any ID collisions detected.
    /// This is the recommended method for semantic validation.
    #[must_use]
    pub fn build_registry_index_with_collisions(&self) -> super::addressing::RegistryBuildResult {
        super::addressing::RegistryIndex::build_from_trees_with_collisions(&self.trees_by_doc)
    }

    /// Build a `TopologyIndex` for fuzzy path discovery.
    ///
    /// The topology index enables structural path lookup and fuzzy matching.
    #[must_use]
    pub fn build_topology_index(&self) -> super::addressing::TopologyIndex {
        super::addressing::TopologyIndex::build_from_trees(&self.trees_by_doc)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/link_graph/index.rs"]
mod tests;
