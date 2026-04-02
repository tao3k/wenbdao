use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{RepoSyncQuery, repo_sync_for_registered_repository};
use crate::gateway::studio::router::handlers::repo::parse::parse_repo_sync_mode;
use crate::gateway::studio::router::handlers::repo::{required_repo_id, shared::with_repository};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Repo sync endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, the sync mode is invalid,
/// repository lookup fails, syncing fails, or the background task panics.
pub async fn sync(
    Query(query): Query<crate::gateway::studio::router::handlers::repo::RepoSyncApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::RepoSyncResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let mode = parse_repo_sync_mode(query.mode.as_deref())?;
    let result = with_repository(
        Arc::clone(&state),
        repo_id.clone(),
        "REPO_SYNC_PANIC",
        "Repo sync task failed unexpectedly",
        !matches!(mode, crate::analyzers::RepoSyncMode::Status),
        move |repository, cwd| {
            repo_sync_for_registered_repository(
                &RepoSyncQuery { repo_id, mode },
                &repository,
                cwd.as_path(),
            )
        },
    )
    .await?;
    Ok(Json(result))
}
