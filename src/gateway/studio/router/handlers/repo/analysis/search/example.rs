use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::gateway::studio::router::handlers::repo::analysis::search::service::run_repo_example_search;
use crate::gateway::studio::router::handlers::repo::{required_repo_id, required_search_query};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Example search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `query` is missing, repository lookup or
/// analysis fails, or the background task panics.
pub async fn example_search(
    Query(query): Query<crate::gateway::studio::router::handlers::repo::RepoSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::ExampleSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let search_query = required_search_query(query.query.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result = run_repo_example_search(Arc::clone(&state), repo_id, search_query, limit).await?;
    Ok(Json(result))
}
