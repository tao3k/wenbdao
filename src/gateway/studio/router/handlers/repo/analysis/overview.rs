use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::gateway::studio::router::handlers::repo::analysis::service::run_repo_overview;
use crate::gateway::studio::router::handlers::repo::required_repo_id;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Repository overview endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup fails,
/// repository analysis fails, or the background task panics.
pub async fn overview(
    Query(query): Query<crate::gateway::studio::router::handlers::repo::RepoApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoOverviewResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result = run_repo_overview(Arc::clone(&state), repo_id).await?;
    Ok(Json(result))
}
