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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IndexedSection {
    pub(crate) heading_title: String,
    pub(crate) heading_path: String,
    pub(crate) heading_path_lower: String,
    pub(crate) heading_level: usize,
    pub(crate) line_start: usize,
    pub(crate) line_end: usize,
    pub(crate) section_text: String,
    pub(crate) section_text_lower: String,
    pub(crate) entities: Vec<String>,
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
            section_text: value.section_text.clone(),
            section_text_lower: value.section_text_lower.clone(),
            entities: value.entities.clone(),
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

/// Cache build metadata emitted by the Valkey-backed LinkGraph bootstrap.
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
    alias_to_doc_id: HashMap<String, String>,
    outgoing: HashMap<String, HashSet<String>>,
    incoming: HashMap<String, HashSet<String>>,
    rank_by_id: HashMap<String, f64>,
    edge_count: usize,
    /// Virtual nodes created by knowledge distillation (collapsed dense clusters).
    virtual_nodes: HashMap<String, build::VirtualNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Refresh execution mode selected by LinkGraph incremental refresh logic.
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

    #[allow(dead_code)]
    pub(in crate::link_graph::index) fn execute_direct_id_lookup(
        &self,
        direct_id: &str,
        _limit: usize,
        _options: &LinkGraphSearchOptions,
    ) -> Vec<LinkGraphHit> {
        let mut out = Vec::new();
        if let Some(doc_id) = self.resolve_doc_id(direct_id) {
            if let Some(doc) = self.docs_by_id.get(doc_id) {
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

    /// Extract semantic intent targets for a document.
    pub fn intent_targets(&self, doc_id: &str) -> (Vec<String>, Vec<String>) {
        let Some(doc) = self.docs_by_id.get(doc_id) else {
            return (Vec::new(), Vec::new());
        };
        // This is a simplification, actual implementation might need more parsing
        (doc.tags.clone(), Vec::new())
    }
}
