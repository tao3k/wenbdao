use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{DocsRetrievalContextQuery, DocsRetrievalHitQuery, DocsRetrievalQuery};
use crate::gateway::studio::router::handlers::docs::service::{
    run_docs_retrieval, run_docs_retrieval_context, run_docs_retrieval_hit,
};
use crate::gateway::studio::router::handlers::repo::{
    RepoProjectedPageSearchApiQuery, RepoProjectedRetrievalContextApiQuery,
    RepoProjectedRetrievalHitApiQuery, parse_projection_page_kind, required_page_id,
    required_repo_id, required_search_query,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Docs retrieval endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn retrieval(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsRetrievalResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = run_docs_retrieval(
        Arc::clone(&state),
        DocsRetrievalQuery {
            repo_id,
            query: search_query,
            kind,
            limit,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs retrieval context endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, retrieval context lookup fails, or the background task
/// panics.
pub async fn retrieval_context(
    Query(query): Query<RepoProjectedRetrievalContextApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsRetrievalContextResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = query.node_id;
    let related_limit = query.related_limit.unwrap_or(5);
    let result = run_docs_retrieval_context(
        Arc::clone(&state),
        DocsRetrievalContextQuery {
            repo_id,
            page_id,
            node_id,
            related_limit,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs retrieval hit endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, retrieval-hit lookup fails, or the background task panics.
pub async fn retrieval_hit(
    Query(query): Query<RepoProjectedRetrievalHitApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsRetrievalHitResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let node_id = query.node_id;
    let result = run_docs_retrieval_hit(
        Arc::clone(&state),
        DocsRetrievalHitQuery {
            repo: repo_id,
            page: page_id,
            node: node_id,
        },
    )
    .await?;
    Ok(Json(result))
}
