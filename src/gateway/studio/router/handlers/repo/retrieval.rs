use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{
    RepoProjectedPageIndexTreeSearchQuery, RepoProjectedPageSearchQuery,
    RepoProjectedRetrievalContextQuery, RepoProjectedRetrievalHitQuery,
    RepoProjectedRetrievalQuery,
};
use crate::gateway::studio::router::handlers::repo::projected_service::{
    run_repo_projected_page_index_tree_search, run_repo_projected_page_search,
    run_repo_projected_retrieval, run_repo_projected_retrieval_context,
    run_repo_projected_retrieval_hit,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

use super::parse::{
    parse_projection_page_kind, required_page_id, required_repo_id, required_search_query,
};
use super::query::{
    RepoProjectedPageSearchApiQuery, RepoProjectedRetrievalContextApiQuery,
    RepoProjectedRetrievalHitApiQuery,
};

/// Projected retrieval hit endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, hit lookup fails, or the background task panics.
pub async fn projected_retrieval_hit(
    Query(query): Query<RepoProjectedRetrievalHitApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedRetrievalHitResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = query.node_id;
    let result = run_repo_projected_retrieval_hit(
        Arc::clone(&state),
        RepoProjectedRetrievalHitQuery {
            repo_id,
            page_id,
            node_id,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected retrieval context endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, retrieval context lookup fails, or the background task
/// panics.
pub async fn projected_retrieval_context(
    Query(query): Query<RepoProjectedRetrievalContextApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedRetrievalContextResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = query.node_id;
    let related_limit = query.related_limit.unwrap_or(5);
    let result = run_repo_projected_retrieval_context(
        Arc::clone(&state),
        RepoProjectedRetrievalContextQuery {
            repo_id,
            page_id,
            node_id,
            related_limit,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page index tree search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn projected_page_index_tree_search(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageIndexTreeSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = run_repo_projected_page_index_tree_search(
        Arc::clone(&state),
        RepoProjectedPageIndexTreeSearchQuery {
            repo_id,
            query: search_query,
            kind,
            limit,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected page search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn projected_page_search(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedPageSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = run_repo_projected_page_search(
        Arc::clone(&state),
        RepoProjectedPageSearchQuery {
            repo_id,
            query: search_query,
            kind,
            limit,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Projected retrieval endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn projected_retrieval(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoProjectedRetrievalResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = run_repo_projected_retrieval(
        Arc::clone(&state),
        RepoProjectedRetrievalQuery {
            repo_id,
            query: search_query,
            kind,
            limit,
        },
    )
    .await?;
    Ok(Json(result))
}
