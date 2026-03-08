//! Core index build + query algorithms for markdown link graph.

use super::models::{
    LinkGraphAttachment, LinkGraphAttachmentHit, LinkGraphAttachmentKind, LinkGraphDirection,
    LinkGraphDocument, LinkGraphEdgeType, LinkGraphHit, LinkGraphLinkFilter,
    LinkGraphMatchStrategy, LinkGraphMetadata, LinkGraphNeighbor, LinkGraphPassage,
    LinkGraphPprSubgraphMode, LinkGraphPromotedOverlayTelemetry, LinkGraphRelatedFilter,
    LinkGraphRelatedPprDiagnostics, LinkGraphRelatedPprOptions, LinkGraphScope,
    LinkGraphSearchFilters, LinkGraphSearchOptions, LinkGraphSortField, LinkGraphSortOrder,
    LinkGraphSortTerm, LinkGraphStats, PageIndexNode,
};
use super::parser::ParsedSection;
use super::query::{ParsedLinkGraphQuery, parse_search_query};
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
mod search;
mod semantic_documents;
mod shared;
mod traversal;
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
pub use search::{
    BatchQuantumScorer, BatchQuantumScorerError, QUANTUM_SALIENCY_COLUMN, QuantumContextBuildError,
    QuantumSemanticIgnition, QuantumSemanticIgnitionError, QuantumSemanticIgnitionFuture,
};

use shared::{
    ScoredSearchRow, deterministic_random_key, doc_contains_phrase, doc_sort_key,
    normalize_path_filter, path_matches_filter, sort_hits,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::link_graph) struct IndexedSection {
    pub(in crate::link_graph) heading_title: String,
    pub(in crate::link_graph) heading_path: String,
    pub(in crate::link_graph) heading_path_lower: String,
    pub(in crate::link_graph) heading_level: usize,
    pub(in crate::link_graph) line_start: usize,
    pub(in crate::link_graph) line_end: usize,
    pub(in crate::link_graph) section_text: String,
    pub(in crate::link_graph) section_text_lower: String,
    #[serde(default)]
    pub(in crate::link_graph) entities: Vec<String>,
}

impl IndexedSection {
    fn from_parsed(value: &ParsedSection) -> Self {
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

/// Cache build metadata emitted by the `Valkey`-backed `LinkGraph` bootstrap.
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
    passages_by_id: HashMap<String, LinkGraphPassage>,
    sections_by_doc: HashMap<String, Vec<IndexedSection>>,
    trees_by_doc: HashMap<String, Vec<PageIndexNode>>,
    attachments_by_doc: HashMap<String, Vec<LinkGraphAttachment>>,
    alias_to_doc_id: HashMap<String, String>,
    outgoing: HashMap<String, HashSet<String>>,
    incoming: HashMap<String, HashSet<String>>,
    rank_by_id: HashMap<String, f64>,
    edge_count: usize,
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
}
