//! Repository Intelligence endpoint handlers for Studio API.

mod analysis;
mod command_service;
mod family;
mod index;
mod pages;
mod parse;
mod projected_service;
mod query;
mod refine;
mod retrieval;
pub(super) mod shared;

pub use analysis::{
    doc_coverage, example_search, import_search, module_search, overview, symbol_search, sync,
};
pub use family::{
    projected_page_family_cluster, projected_page_family_context, projected_page_family_search,
    projected_page_navigation, projected_page_navigation_search,
};
pub use index::{repo_index, repo_index_status};
pub use pages::{
    projected_gap_report, projected_page, projected_page_index_node, projected_page_index_tree,
    projected_page_index_trees, projected_pages,
};
pub(crate) use parse::{
    parse_projected_gap_kind, parse_projection_page_kind, required_gap_id,
    required_import_search_filters, required_page_id, required_projection_page_kind,
    required_repo_id, required_search_query,
};
pub use query::{
    RepoApiQuery, RepoDocCoverageApiQuery, RepoImportSearchApiQuery, RepoIndexStatusApiQuery,
    RepoProjectedPageApiQuery, RepoProjectedPageFamilyClusterApiQuery,
    RepoProjectedPageFamilyContextApiQuery, RepoProjectedPageFamilySearchApiQuery,
    RepoProjectedPageIndexNodeApiQuery, RepoProjectedPageNavigationApiQuery,
    RepoProjectedPageNavigationSearchApiQuery, RepoProjectedPageSearchApiQuery,
    RepoProjectedRetrievalContextApiQuery, RepoProjectedRetrievalHitApiQuery, RepoSearchApiQuery,
    RepoSyncApiQuery,
};
pub use refine::refine_entity_doc;
pub use retrieval::{
    projected_page_index_tree_search, projected_page_search, projected_retrieval,
    projected_retrieval_context, projected_retrieval_hit,
};
