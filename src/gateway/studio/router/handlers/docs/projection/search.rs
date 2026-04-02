use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::DocsSearchQuery;
use crate::gateway::studio::router::handlers::docs::service::run_docs_search;
use crate::gateway::studio::router::handlers::repo::{
    RepoProjectedPageSearchApiQuery, parse_projection_page_kind, required_repo_id,
    required_search_query,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Docs search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, the kind filter is
/// invalid, repository lookup or analysis fails, or the background task
/// panics.
pub async fn search(
    Query(query): Query<RepoProjectedPageSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let kind = parse_projection_page_kind(query.kind.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = run_docs_search(
        Arc::clone(&state),
        DocsSearchQuery {
            repo_id,
            query: search_query,
            kind,
            limit,
        },
    )
    .await?;
    Ok(Json(result))
}
