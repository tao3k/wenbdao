//! Core index build + query algorithms for markdown link graph.

pub(in crate::link_graph::index) use super::models::{
    LinkGraphDirection, LinkGraphDocument, LinkGraphEdgeType, LinkGraphHit, LinkGraphLinkFilter,
    LinkGraphMatchStrategy, LinkGraphMetadata, LinkGraphNeighbor, LinkGraphPprSubgraphMode,
    LinkGraphRelatedFilter, LinkGraphRelatedPprDiagnostics, LinkGraphRelatedPprOptions,
    LinkGraphScope, LinkGraphSearchFilters, LinkGraphSearchOptions, LinkGraphSortField,
    LinkGraphSortOrder, LinkGraphSortTerm, LinkGraphStats, PageIndexNode,
};
pub(in crate::link_graph::index) use super::query::parse_search_query;

mod agentic_expansion;
mod agentic_overlay;
mod build;
mod constants;
mod ids;
mod lookup;
mod page_indices;
mod passages;
mod ppr;
mod rank;
mod scoring;
pub(crate) mod search;
mod semantic_documents;
mod shared;
mod symbol_cache;
mod traversal;
mod types;

pub use search::quantum_fusion::orchestrate::QuantumContextBuildError;
pub use search::quantum_fusion::semantic_ignition::{
    QuantumSemanticIgnition, QuantumSemanticIgnitionError, QuantumSemanticIgnitionFuture,
};

pub(in crate::link_graph::index) use constants::{
    DEFAULT_MIN_SECTION_WORDS, DEFAULT_PER_DOC_SECTION_CAP, INCOMING_RANK_FACTOR,
    INCREMENTAL_REBUILD_THRESHOLD, MAX_GRAPH_RANK_BOOST, OUTGOING_RANK_FACTOR,
    SECTION_AGGREGATION_BETA, WEIGHT_FTS_LEXICAL, WEIGHT_FTS_PATH, WEIGHT_FTS_SECTION,
    WEIGHT_PATH_FUZZY_PATH, WEIGHT_PATH_FUZZY_SECTION,
};
pub(in crate::link_graph::index) use scoring::{
    normalize_with_case, score_document, score_document_exact, score_document_regex,
    score_path_fields, section_tree_distance, token_match_ratio, tokenize,
};
pub(in crate::link_graph::index) use shared::{
    ScoredSearchRow, deterministic_random_key, doc_contains_phrase, doc_sort_key,
    normalize_path_filter, path_matches_filter,
};
pub(crate) use types::{IndexedSection, SectionCandidate, SectionMatch};
pub use types::{
    LinkGraphCacheBuildMeta, LinkGraphIndex, LinkGraphRefreshMode, LinkGraphVirtualNode,
    PageIndexParent, SymbolCacheStats, SymbolRef,
};

#[cfg(test)]
#[path = "../../../tests/unit/link_graph/index.rs"]
mod tests;
