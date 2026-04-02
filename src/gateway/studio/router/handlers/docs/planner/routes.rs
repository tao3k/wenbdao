use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{
    DocsPlannerItemQuery, DocsPlannerQueueQuery, DocsPlannerRankQuery, DocsPlannerSearchQuery,
    DocsPlannerWorksetQuery,
};
use crate::gateway::studio::router::handlers::docs::service::{
    run_docs_planner_item, run_docs_planner_queue, run_docs_planner_rank, run_docs_planner_search,
    run_docs_planner_workset,
};
use crate::gateway::studio::router::handlers::docs::types::{
    DocsPlannerItemApiQuery, DocsPlannerQueueApiQuery, DocsPlannerRankApiQuery,
    DocsPlannerSearchApiQuery, DocsPlannerWorksetApiQuery,
};
use crate::gateway::studio::router::handlers::repo::{
    parse_projected_gap_kind, parse_projection_page_kind, required_gap_id, required_repo_id,
    required_search_query,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Docs planner-item endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `gap_id` is missing, the family filter is invalid,
/// repository lookup or analysis fails, planner-item lookup fails, or the background task panics.
pub async fn planner_item(
    Query(query): Query<DocsPlannerItemApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPlannerItemResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let gap_id = required_gap_id(query.gap_id.as_deref())?;
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let result = run_docs_planner_item(
        Arc::clone(&state),
        DocsPlannerItemQuery {
            repo_id,
            gap_id,
            family_kind,
            related_limit,
            family_limit,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs planner-search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, a filter is invalid, repository lookup or
/// analysis fails, or the background task panics.
pub async fn planner_search(
    Query(query): Query<DocsPlannerSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPlannerSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let gap_kind = parse_projected_gap_kind(query.gap_kind.as_deref())?;
    let page_kind = parse_projection_page_kind(query.page_kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = run_docs_planner_search(
        Arc::clone(&state),
        DocsPlannerSearchQuery {
            repo_id,
            query: search_query,
            gap_kind,
            page_kind,
            limit,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs planner-queue endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, a filter is invalid, repository lookup or analysis
/// fails, or the background task panics.
pub async fn planner_queue(
    Query(query): Query<DocsPlannerQueueApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPlannerQueueResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let gap_kind = parse_projected_gap_kind(query.gap_kind.as_deref())?;
    let page_kind = parse_projection_page_kind(query.page_kind.as_deref())?;
    let per_kind_limit = query.per_kind_limit.unwrap_or(3).max(1);
    let result = run_docs_planner_queue(
        Arc::clone(&state),
        DocsPlannerQueueQuery {
            repo_id,
            gap_kind,
            page_kind,
            per_kind_limit,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs planner-rank endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, a filter is invalid, repository lookup or analysis
/// fails, or the background task panics.
pub async fn planner_rank(
    Query(query): Query<DocsPlannerRankApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPlannerRankResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let gap_kind = parse_projected_gap_kind(query.gap_kind.as_deref())?;
    let page_kind = parse_projection_page_kind(query.page_kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = run_docs_planner_rank(
        Arc::clone(&state),
        DocsPlannerRankQuery {
            repo_id,
            gap_kind,
            page_kind,
            limit,
        },
    )
    .await?;
    Ok(Json(result))
}

/// Docs planner-workset endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, a filter is invalid, repository lookup or analysis
/// fails, one selected planner item cannot be reopened, or the background task panics.
pub async fn planner_workset(
    Query(query): Query<DocsPlannerWorksetApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPlannerWorksetResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let gap_kind = parse_projected_gap_kind(query.gap_kind.as_deref())?;
    let page_kind = parse_projection_page_kind(query.page_kind.as_deref())?;
    let per_kind_limit = query.per_kind_limit.unwrap_or(3).max(1);
    let limit = query.limit.unwrap_or(3).max(1);
    let family_kind = parse_projection_page_kind(query.family_kind.as_deref())?;
    let related_limit = query.related_limit.unwrap_or(5);
    let family_limit = query.family_limit.unwrap_or(3).max(1);
    let result = run_docs_planner_workset(
        Arc::clone(&state),
        DocsPlannerWorksetQuery {
            repo_id,
            gap_kind,
            page_kind,
            per_kind_limit,
            limit,
            family_kind,
            related_limit,
            family_limit,
        },
    )
    .await?;
    Ok(Json(result))
}
