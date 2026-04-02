//! High-level repository intelligence service orchestration.

mod analysis;
mod bootstrap;
mod cached;
mod helpers;
mod julia_transport;
#[cfg(test)]
mod julia_transport_tests;
mod merge;
mod projection;
mod registry;
mod relation_dedupe;
mod search;
mod sync;

pub use analysis::{
    analyze_registered_repository, analyze_registered_repository_with_registry,
    analyze_repository_from_config, analyze_repository_from_config_with_registry,
};
pub use bootstrap::bootstrap_builtin_registry;
pub use cached::analyze_registered_repository_cached_with_registry;
pub(crate) use cached::{
    CachedRepositoryAnalysis, analyze_registered_repository_cached_bundle_with_registry,
};
pub use helpers::relation_kind_label;
pub(crate) use helpers::{
    backlinks_for, documents_backlink_lookup, example_match_score, example_relation_lookup,
    hierarchy_segments_from_path, import_match_score, infer_ecosystem, module_match_score,
    normalized_rank_score, projection_page_lookup, projection_pages_for, record_hierarchical_uri,
    related_modules_for_example, related_symbols_for_example, symbol_match_score,
};
pub use julia_transport::{
    JULIA_ARROW_ANALYZER_SCORE_COLUMN, JULIA_ARROW_DOC_ID_COLUMN, JULIA_ARROW_EMBEDDING_COLUMN,
    JULIA_ARROW_FINAL_SCORE_COLUMN, JULIA_ARROW_QUERY_EMBEDDING_COLUMN,
    JULIA_ARROW_TRACE_ID_COLUMN, JULIA_ARROW_VECTOR_SCORE_COLUMN, julia_arrow_request_schema,
    julia_arrow_response_schema,
};
#[cfg(feature = "julia")]
pub use julia_transport::{
    JuliaArrowRequestRow, JuliaArrowScoreRow, PluginArrowRequestRow, PluginArrowScoreRow,
    build_julia_arrow_request_batch, build_plugin_arrow_request_batch,
    decode_julia_arrow_score_rows, decode_plugin_arrow_score_rows,
    fetch_julia_flight_score_rows_for_repository, fetch_plugin_arrow_score_rows_for_repository,
};

pub(crate) use projection::build_repo_projected_page_search_with_artifacts;
pub use projection::*;
pub use registry::load_registered_repository;
pub(crate) use search::ExampleSearchMetadata;
pub use search::*;
pub(crate) use search::{
    RepoAnalysisFallbackContract, canonical_import_query_text, example_fallback_contract,
    import_fallback_contract, module_fallback_contract, repository_search_artifacts,
    symbol_fallback_contract,
};
pub use sync::*;
#[cfg(test)]
mod tests;
