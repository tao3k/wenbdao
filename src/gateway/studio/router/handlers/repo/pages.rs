use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{
    RepoProjectedGapReportQuery, RepoProjectedPageIndexNodeQuery, RepoProjectedPageIndexTreeQuery,
    RepoProjectedPageIndexTreesQuery, RepoProjectedPageQuery, RepoProjectedPagesQuery,
};
use crate::gateway::studio::router::handlers::repo::projected_service::{
    run_repo_projected_gap_report, run_repo_projected_page, run_repo_projected_page_index_node,
    run_repo_projected_page_index_tree, run_repo_projected_page_index_trees,
    run_repo_projected_pages,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

use super::parse::{required_node_id, required_page_id, required_repo_id};
use super::query::{RepoApiQuery, RepoProjectedPageApiQuery, RepoProjectedPageIndexNodeApiQuery};

/// Projected pages endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup or analysis
/// fails, or the background task panics.
pub async fn projected_pages(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPagesResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result =
        run_repo_projected_pages(Arc::clone(&state), RepoProjectedPagesQuery { repo_id }).await?;
    Ok(Json(result))
}

/// Projected gap report endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup or analysis
/// fails, or the background task panics.
pub async fn projected_gap_report(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedGapReportResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result =
        run_repo_projected_gap_report(Arc::clone(&state), RepoProjectedGapReportQuery { repo_id })
            .await?;
    Ok(Json(result))
}

/// Projected page endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, projected page lookup fails, or the background task panics.
pub async fn projected_page(
    Query(query): Query<RepoProjectedPageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let result = run_repo_projected_page(
        Arc::clone(&state),
        RepoProjectedPageQuery { repo_id, page_id },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page index tree endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, page-index tree lookup fails, or the background task
/// panics.
pub async fn projected_page_index_tree(
    Query(query): Query<RepoProjectedPageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexTreeResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let result = run_repo_projected_page_index_tree(
        Arc::clone(&state),
        RepoProjectedPageIndexTreeQuery { repo_id, page_id },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page index node endpoint.
///
/// # Errors
///
/// Returns an error when `repo`, `page_id`, or `node_id` is missing,
/// repository lookup or analysis fails, page-index node lookup fails, or the
/// background task panics.
pub async fn projected_page_index_node(
    Query(query): Query<RepoProjectedPageIndexNodeApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexNodeResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = required_node_id(query.node_id.as_deref())?;
    let result = run_repo_projected_page_index_node(
        Arc::clone(&state),
        RepoProjectedPageIndexNodeQuery {
            repo_id,
            page_id,
            node_id,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page index trees endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup or analysis
/// fails, page-index tree construction fails, or the background task panics.
pub async fn projected_page_index_trees(
    Query(query): Query<RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexTreesResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result = run_repo_projected_page_index_trees(
        Arc::clone(&state),
        RepoProjectedPageIndexTreesQuery { repo_id },
    )
    .await?;
    Ok(Json(result))
}
