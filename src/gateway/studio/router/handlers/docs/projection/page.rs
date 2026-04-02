use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::DocsPageQuery;
use crate::gateway::studio::router::handlers::docs::service::run_docs_page;
use crate::gateway::studio::router::handlers::repo::{
    RepoProjectedPageApiQuery, required_page_id, required_repo_id,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Docs page endpoint.
///
/// # Errors
///
/// Returns an error when `repo` or `page_id` is missing, repository lookup or
/// analysis fails, projected page lookup fails, or the background task panics.
pub async fn page(
    Query(query): Query<RepoProjectedPageApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsPageResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let page_id = required_page_id(query.page_id.as_deref())?;
    let result = run_docs_page(Arc::clone(&state), DocsPageQuery { repo_id, page_id }).await?;
    Ok(Json(result))
}
