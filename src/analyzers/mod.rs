//! Repo Intelligence common-core contracts and plugin registry.
//!
//! This module defines the initial Wendao-native contracts for repository
//! intelligence. The first landing focuses on:
//!
//! - repository registration metadata
//! - normalized records for repository understanding
//! - query request/response contracts
//! - plugin registration and dispatch boundaries

/// Analysis cache layer for repository intelligence results.
pub mod cache;
/// Configuration types for repository registration.
pub mod config;
/// Error types for repository intelligence operations.
pub mod errors;
/// Language-specific plugin placeholders and registration guidance.
pub mod languages;
/// Plugin trait definitions and analysis context types.
pub mod plugin;
/// Projection layer for transforming analysis records into consumable outputs.
pub mod projection;
/// Query request and response contracts.
pub mod query;
/// Normalized record types for repository understanding.
pub mod records;
/// Plugin registry for dynamic analyzer registration.
pub mod registry;
/// Saliency scoring for symbol and module importance.
pub mod saliency;
/// Analysis orchestration and repository processing services.
pub mod service;
/// Verification auditing (skeptic) for documentation coverage.
pub mod skeptic;

pub use config::{
    RegisteredRepository, RepoIntelligenceConfig, RepositoryPluginConfig, RepositoryRef,
    RepositoryRefreshPolicy, load_repo_intelligence_config,
};
pub use errors::RepoIntelligenceError;
pub use plugin::{
    AnalysisContext, PluginAnalysisOutput, PluginLinkContext, RepoIntelligencePlugin,
    RepoSourceFile, RepositoryAnalysisOutput,
};
pub use projection::{
    ProjectedMarkdownDocument, ProjectedPageIndexDocument, ProjectedPageIndexNode,
    ProjectedPageIndexSection, ProjectedPageIndexTree, ProjectedPageRecord, ProjectedPageSection,
    ProjectionInputBundle, ProjectionPageKind, ProjectionPageSeed, build_projected_gap_report,
    build_projected_page, build_projected_page_family_cluster, build_projected_page_family_context,
    build_projected_page_family_search, build_projected_page_index_documents,
    build_projected_page_index_node, build_projected_page_index_tree,
    build_projected_page_index_tree_search, build_projected_page_index_trees,
    build_projected_page_navigation, build_projected_page_navigation_search,
    build_projected_page_search, build_projected_pages, build_projected_retrieval,
    build_projected_retrieval_context, build_projected_retrieval_hit, build_projection_inputs,
    render_projected_markdown_documents,
};
pub use query::{
    DocCoverageQuery, DocCoverageResult, DocsFamilyClusterQuery, DocsFamilyClusterResult,
    DocsFamilyContextQuery, DocsFamilyContextResult, DocsFamilySearchQuery, DocsFamilySearchResult,
    DocsMarkdownDocumentsQuery, DocsMarkdownDocumentsResult, DocsNavigationQuery,
    DocsNavigationResult, DocsNavigationSearchQuery, DocsNavigationSearchResult,
    DocsPageIndexDocumentsQuery, DocsPageIndexDocumentsResult, DocsPageIndexNodeQuery,
    DocsPageIndexNodeResult, DocsPageIndexTreeQuery, DocsPageIndexTreeResult,
    DocsPageIndexTreeSearchQuery, DocsPageIndexTreeSearchResult, DocsPageIndexTreesQuery,
    DocsPageIndexTreesResult, DocsPageQuery, DocsPageResult, DocsPlannerItemQuery,
    DocsPlannerItemResult, DocsPlannerQueueGroup, DocsPlannerQueueQuery, DocsPlannerQueueResult,
    DocsPlannerRankHit, DocsPlannerRankQuery, DocsPlannerRankReason, DocsPlannerRankReasonCode,
    DocsPlannerRankResult, DocsPlannerSearchHit, DocsPlannerSearchQuery, DocsPlannerSearchResult,
    DocsPlannerWorksetBalance, DocsPlannerWorksetFamilyBalanceEntry, DocsPlannerWorksetFamilyGroup,
    DocsPlannerWorksetGapKindBalanceEntry, DocsPlannerWorksetGroup, DocsPlannerWorksetQuery,
    DocsPlannerWorksetQuotaHint, DocsPlannerWorksetResult, DocsProjectedGapReportQuery,
    DocsProjectedGapReportResult, DocsRetrievalContextQuery, DocsRetrievalContextResult,
    DocsRetrievalHitQuery, DocsRetrievalHitResult, DocsRetrievalQuery, DocsRetrievalResult,
    DocsSearchQuery, DocsSearchResult, ExampleSearchHit, ExampleSearchQuery, ExampleSearchResult,
    ImportSearchHit, ImportSearchQuery, ImportSearchResult, ModuleSearchHit, ModuleSearchQuery,
    ModuleSearchResult, ProjectedGapKind, ProjectedGapRecord, ProjectedGapSummary,
    ProjectedGapSummaryEntry, ProjectedPageFamilyCluster, ProjectedPageFamilyContextEntry,
    ProjectedPageFamilySearchHit, ProjectedPageIndexNodeContext, ProjectedPageIndexNodeHit,
    ProjectedPageNavigationSearchHit, ProjectedRetrievalHit, ProjectedRetrievalHitKind,
    RefineEntityDocRequest, RefineEntityDocResponse, RepoBacklinkItem, RepoOverviewQuery,
    RepoOverviewResult, RepoProjectedGapReportQuery, RepoProjectedGapReportResult,
    RepoProjectedPageFamilyClusterQuery, RepoProjectedPageFamilyClusterResult,
    RepoProjectedPageFamilyContextQuery, RepoProjectedPageFamilyContextResult,
    RepoProjectedPageFamilySearchQuery, RepoProjectedPageFamilySearchResult,
    RepoProjectedPageIndexNodeQuery, RepoProjectedPageIndexNodeResult,
    RepoProjectedPageIndexTreeQuery, RepoProjectedPageIndexTreeResult,
    RepoProjectedPageIndexTreeSearchQuery, RepoProjectedPageIndexTreeSearchResult,
    RepoProjectedPageIndexTreesQuery, RepoProjectedPageIndexTreesResult,
    RepoProjectedPageNavigationQuery, RepoProjectedPageNavigationResult,
    RepoProjectedPageNavigationSearchQuery, RepoProjectedPageNavigationSearchResult,
    RepoProjectedPageQuery, RepoProjectedPageResult, RepoProjectedPageSearchQuery,
    RepoProjectedPageSearchResult, RepoProjectedPagesQuery, RepoProjectedPagesResult,
    RepoProjectedRetrievalContextQuery, RepoProjectedRetrievalContextResult,
    RepoProjectedRetrievalHitQuery, RepoProjectedRetrievalHitResult, RepoProjectedRetrievalQuery,
    RepoProjectedRetrievalResult, RepoSourceKind, RepoSyncDriftState, RepoSyncFreshnessSummary,
    RepoSyncHealthState, RepoSyncLifecycleSummary, RepoSyncMode, RepoSyncQuery, RepoSyncResult,
    RepoSyncRevisionSummary, RepoSyncStalenessState, RepoSyncState, RepoSyncStatusSummary,
    SymbolSearchHit, SymbolSearchQuery, SymbolSearchResult,
};
pub use records::{
    DiagnosticRecord, DocRecord, ExampleRecord, ImportKind, ImportRecord, ModuleRecord,
    RelationKind, RelationRecord, RepoSymbolKind, RepositoryRecord, SymbolRecord,
};
pub use registry::PluginRegistry;
pub use service::{
    JULIA_ARROW_ANALYZER_SCORE_COLUMN, JULIA_ARROW_DOC_ID_COLUMN, JULIA_ARROW_EMBEDDING_COLUMN,
    JULIA_ARROW_FINAL_SCORE_COLUMN, JULIA_ARROW_QUERY_EMBEDDING_COLUMN,
    JULIA_ARROW_TRACE_ID_COLUMN, JULIA_ARROW_VECTOR_SCORE_COLUMN, julia_arrow_request_schema,
    julia_arrow_response_schema,
};
#[cfg(feature = "julia")]
pub use service::{
    JuliaArrowRequestRow, JuliaArrowScoreRow, PluginArrowRequestRow, PluginArrowScoreRow,
    build_julia_arrow_request_batch, build_plugin_arrow_request_batch,
    decode_julia_arrow_score_rows, decode_plugin_arrow_score_rows,
    fetch_julia_flight_score_rows_for_repository, fetch_plugin_arrow_score_rows_for_repository,
};
pub use service::{
    analyze_registered_repository, analyze_registered_repository_cached_with_registry,
    analyze_registered_repository_with_registry, analyze_repository_from_config,
    analyze_repository_from_config_with_registry, bootstrap_builtin_registry, build_doc_coverage,
    build_docs_family_cluster, build_docs_family_context, build_docs_family_search,
    build_docs_markdown_documents, build_docs_navigation, build_docs_navigation_search,
    build_docs_page, build_docs_page_index_documents, build_docs_page_index_node,
    build_docs_page_index_tree, build_docs_page_index_tree_search, build_docs_page_index_trees,
    build_docs_planner_item, build_docs_planner_queue, build_docs_planner_rank,
    build_docs_planner_search, build_docs_planner_workset, build_docs_projected_gap_report,
    build_docs_retrieval, build_docs_retrieval_context, build_docs_retrieval_hit,
    build_docs_search, build_example_search, build_import_search, build_module_search,
    build_repo_overview, build_repo_projected_gap_report, build_repo_projected_page,
    build_repo_projected_page_family_cluster, build_repo_projected_page_family_context,
    build_repo_projected_page_family_search, build_repo_projected_page_index_node,
    build_repo_projected_page_index_tree, build_repo_projected_page_index_tree_search,
    build_repo_projected_page_index_trees, build_repo_projected_page_navigation,
    build_repo_projected_page_navigation_search, build_repo_projected_page_search,
    build_repo_projected_pages, build_repo_projected_retrieval,
    build_repo_projected_retrieval_context, build_repo_projected_retrieval_hit,
    build_symbol_search, doc_coverage_from_config, doc_coverage_from_config_with_registry,
    docs_family_cluster_from_config, docs_family_cluster_from_config_with_registry,
    docs_family_context_from_config, docs_family_context_from_config_with_registry,
    docs_family_search_from_config, docs_family_search_from_config_with_registry,
    docs_markdown_documents_from_config, docs_markdown_documents_from_config_with_registry,
    docs_navigation_from_config, docs_navigation_from_config_with_registry,
    docs_navigation_search_from_config, docs_navigation_search_from_config_with_registry,
    docs_page_from_config, docs_page_from_config_with_registry,
    docs_page_index_documents_from_config, docs_page_index_documents_from_config_with_registry,
    docs_page_index_node_from_config, docs_page_index_node_from_config_with_registry,
    docs_page_index_tree_from_config, docs_page_index_tree_from_config_with_registry,
    docs_page_index_tree_search_from_config, docs_page_index_tree_search_from_config_with_registry,
    docs_page_index_trees_from_config, docs_page_index_trees_from_config_with_registry,
    docs_planner_item_from_config, docs_planner_item_from_config_with_registry,
    docs_planner_queue_from_config, docs_planner_queue_from_config_with_registry,
    docs_planner_rank_from_config, docs_planner_rank_from_config_with_registry,
    docs_planner_search_from_config, docs_planner_search_from_config_with_registry,
    docs_planner_workset_from_config, docs_planner_workset_from_config_with_registry,
    docs_projected_gap_report_from_config, docs_projected_gap_report_from_config_with_registry,
    docs_retrieval_context_from_config, docs_retrieval_context_from_config_with_registry,
    docs_retrieval_from_config, docs_retrieval_from_config_with_registry,
    docs_retrieval_hit_from_config, docs_retrieval_hit_from_config_with_registry,
    docs_search_from_config, docs_search_from_config_with_registry, example_search_from_config,
    example_search_from_config_with_registry, import_search_from_config,
    import_search_from_config_with_registry, load_registered_repository, module_search_from_config,
    module_search_from_config_with_registry, repo_overview_from_config,
    repo_overview_from_config_with_registry, repo_projected_gap_report_from_config,
    repo_projected_gap_report_from_config_with_registry,
    repo_projected_page_family_cluster_from_config,
    repo_projected_page_family_cluster_from_config_with_registry,
    repo_projected_page_family_context_from_config,
    repo_projected_page_family_context_from_config_with_registry,
    repo_projected_page_family_search_from_config,
    repo_projected_page_family_search_from_config_with_registry, repo_projected_page_from_config,
    repo_projected_page_from_config_with_registry, repo_projected_page_index_node_from_config,
    repo_projected_page_index_node_from_config_with_registry,
    repo_projected_page_index_tree_from_config,
    repo_projected_page_index_tree_from_config_with_registry,
    repo_projected_page_index_tree_search_from_config,
    repo_projected_page_index_tree_search_from_config_with_registry,
    repo_projected_page_index_trees_from_config,
    repo_projected_page_index_trees_from_config_with_registry,
    repo_projected_page_navigation_from_config,
    repo_projected_page_navigation_from_config_with_registry,
    repo_projected_page_navigation_search_from_config,
    repo_projected_page_navigation_search_from_config_with_registry,
    repo_projected_page_search_from_config, repo_projected_page_search_from_config_with_registry,
    repo_projected_pages_from_config, repo_projected_pages_from_config_with_registry,
    repo_projected_retrieval_context_from_config,
    repo_projected_retrieval_context_from_config_with_registry,
    repo_projected_retrieval_from_config, repo_projected_retrieval_from_config_with_registry,
    repo_projected_retrieval_hit_from_config,
    repo_projected_retrieval_hit_from_config_with_registry, repo_sync_for_registered_repository,
    repo_sync_from_config, symbol_search_from_config, symbol_search_from_config_with_registry,
};
