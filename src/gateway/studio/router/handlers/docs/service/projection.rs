use std::sync::Arc;

use crate::analyzers::{
    DocsFamilyClusterQuery, DocsFamilyClusterResult, DocsFamilyContextQuery,
    DocsFamilyContextResult, DocsFamilySearchQuery, DocsFamilySearchResult, DocsNavigationQuery,
    DocsNavigationResult, DocsNavigationSearchQuery, DocsNavigationSearchResult, DocsPageQuery,
    DocsPageResult, DocsProjectedGapReportQuery, DocsProjectedGapReportResult,
    DocsRetrievalContextQuery, DocsRetrievalContextResult, DocsRetrievalHitQuery,
    DocsRetrievalHitResult, DocsRetrievalQuery, DocsRetrievalResult, DocsSearchQuery,
    DocsSearchResult, RepoIntelligenceError, build_docs_family_cluster, build_docs_family_context,
    build_docs_family_search, build_docs_navigation, build_docs_navigation_search, build_docs_page,
    build_docs_projected_gap_report, build_docs_retrieval, build_docs_retrieval_context,
    build_docs_retrieval_hit, build_docs_search,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError, map_repo_intelligence_error};

use crate::gateway::studio::router::handlers::docs::service::runtime::run_docs_analysis;

pub(crate) async fn run_docs_search(
    state: Arc<GatewayState>,
    query: DocsSearchQuery,
) -> Result<DocsSearchResult, StudioApiError> {
    run_docs_analysis(
        Arc::clone(&state),
        query.repo_id.clone(),
        "DOCS_SEARCH_PANIC",
        "Docs search task failed unexpectedly",
        move |analysis| Ok::<_, RepoIntelligenceError>(build_docs_search(&query, &analysis)),
    )
    .await
}

pub(crate) async fn run_docs_retrieval(
    state: Arc<GatewayState>,
    query: DocsRetrievalQuery,
) -> Result<DocsRetrievalResult, StudioApiError> {
    run_docs_analysis(
        Arc::clone(&state),
        query.repo_id.clone(),
        "DOCS_RETRIEVAL_PANIC",
        "Docs retrieval task failed unexpectedly",
        move |analysis| Ok::<_, RepoIntelligenceError>(build_docs_retrieval(&query, &analysis)),
    )
    .await
}

pub(crate) async fn run_docs_retrieval_context(
    state: Arc<GatewayState>,
    query: DocsRetrievalContextQuery,
) -> Result<DocsRetrievalContextResult, StudioApiError> {
    run_docs_analysis(
        Arc::clone(&state),
        query.repo_id.clone(),
        "DOCS_RETRIEVAL_CONTEXT_PANIC",
        "Docs retrieval context task failed unexpectedly",
        move |analysis| build_docs_retrieval_context(&query, &analysis),
    )
    .await
}

pub(crate) async fn run_docs_retrieval_hit(
    state: Arc<GatewayState>,
    query: DocsRetrievalHitQuery,
) -> Result<DocsRetrievalHitResult, StudioApiError> {
    let result = run_docs_analysis(
        Arc::clone(&state),
        query.repo.clone(),
        "DOCS_RETRIEVAL_HIT_PANIC",
        "Docs retrieval hit task failed unexpectedly",
        move |analysis| Ok::<_, RepoIntelligenceError>(build_docs_retrieval_hit(&query, &analysis)),
    )
    .await?;
    result.map_err(map_repo_intelligence_error)
}

pub(crate) async fn run_docs_page(
    state: Arc<GatewayState>,
    query: DocsPageQuery,
) -> Result<DocsPageResult, StudioApiError> {
    run_docs_analysis(
        Arc::clone(&state),
        query.repo_id.clone(),
        "DOCS_PAGE_PANIC",
        "Docs page task failed unexpectedly",
        move |analysis| build_docs_page(&query, &analysis),
    )
    .await
}

pub(crate) async fn run_docs_family_context(
    state: Arc<GatewayState>,
    query: DocsFamilyContextQuery,
) -> Result<DocsFamilyContextResult, StudioApiError> {
    run_docs_analysis(
        Arc::clone(&state),
        query.repo_id.clone(),
        "DOCS_FAMILY_CONTEXT_PANIC",
        "Docs family context task failed unexpectedly",
        move |analysis| build_docs_family_context(&query, &analysis),
    )
    .await
}

pub(crate) async fn run_docs_family_search(
    state: Arc<GatewayState>,
    query: DocsFamilySearchQuery,
) -> Result<DocsFamilySearchResult, StudioApiError> {
    run_docs_analysis(
        Arc::clone(&state),
        query.repo_id.clone(),
        "DOCS_FAMILY_SEARCH_PANIC",
        "Docs family search task failed unexpectedly",
        move |analysis| Ok::<_, RepoIntelligenceError>(build_docs_family_search(&query, &analysis)),
    )
    .await
}

pub(crate) async fn run_docs_family_cluster(
    state: Arc<GatewayState>,
    query: DocsFamilyClusterQuery,
) -> Result<DocsFamilyClusterResult, StudioApiError> {
    run_docs_analysis(
        Arc::clone(&state),
        query.repo_id.clone(),
        "DOCS_FAMILY_CLUSTER_PANIC",
        "Docs family cluster task failed unexpectedly",
        move |analysis| build_docs_family_cluster(&query, &analysis),
    )
    .await
}

pub(crate) async fn run_docs_navigation(
    state: Arc<GatewayState>,
    query: DocsNavigationQuery,
) -> Result<DocsNavigationResult, StudioApiError> {
    run_docs_analysis(
        Arc::clone(&state),
        query.repo_id.clone(),
        "DOCS_NAVIGATION_PANIC",
        "Docs navigation task failed unexpectedly",
        move |analysis| build_docs_navigation(&query, &analysis),
    )
    .await
}

pub(crate) async fn run_docs_navigation_search(
    state: Arc<GatewayState>,
    query: DocsNavigationSearchQuery,
) -> Result<DocsNavigationSearchResult, StudioApiError> {
    run_docs_analysis(
        Arc::clone(&state),
        query.repo_id.clone(),
        "DOCS_NAVIGATION_SEARCH_PANIC",
        "Docs navigation search task failed unexpectedly",
        move |analysis| build_docs_navigation_search(&query, &analysis),
    )
    .await
}

pub(crate) async fn run_docs_projected_gap_report(
    state: Arc<GatewayState>,
    query: DocsProjectedGapReportQuery,
) -> Result<DocsProjectedGapReportResult, StudioApiError> {
    run_docs_analysis(
        Arc::clone(&state),
        query.repo_id.clone(),
        "DOCS_PROJECTED_GAP_REPORT_PANIC",
        "Docs projected gap report task failed unexpectedly",
        move |analysis| {
            Ok::<_, RepoIntelligenceError>(build_docs_projected_gap_report(&query, &analysis))
        },
    )
    .await
}
