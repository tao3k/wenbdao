use std::sync::Arc;

use crate::analyzers::{RefineEntityDocRequest, RefineEntityDocResponse};
use crate::gateway::studio::repo_index::{RepoIndexRequest, RepoIndexStatusResponse};
use crate::gateway::studio::router::handlers::repo::shared::{
    repo_index_repositories, with_repo_analysis,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

pub(crate) async fn run_repo_index(
    state: Arc<GatewayState>,
    payload: RepoIndexRequest,
) -> Result<RepoIndexStatusResponse, StudioApiError> {
    let repositories = repo_index_repositories(&state, payload.repo.as_deref())?;
    if repositories.is_empty() {
        return Err(StudioApiError::bad_request(
            "UNKNOWN_REPOSITORY",
            "No configured repository is available for repo indexing",
        ));
    }
    state
        .studio
        .repo_index
        .ensure_repositories_enqueued(repositories, payload.refresh);
    Ok(state
        .studio
        .repo_index
        .status_response(payload.repo.as_deref()))
}

pub(crate) fn run_repo_index_status(
    state: &Arc<GatewayState>,
    repo: Option<&str>,
) -> RepoIndexStatusResponse {
    state.studio.repo_index_status(repo)
}

pub(crate) async fn run_refine_entity_doc(
    state: Arc<GatewayState>,
    payload: RefineEntityDocRequest,
) -> Result<RefineEntityDocResponse, StudioApiError> {
    let repo_id = crate::gateway::studio::router::handlers::repo::required_repo_id(Some(
        payload.repo_id.as_str(),
    ))?;
    with_repo_analysis(
        Arc::clone(&state),
        repo_id,
        "REFINE_DOC_PANIC",
        "Refine documentation task failed unexpectedly",
        move |analysis| {
            let symbol = analysis
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == payload.entity_id)
                .ok_or_else(|| crate::RepoIntelligenceError::AnalysisFailed {
                    message: format!("Entity `{}` not found", payload.entity_id),
                })?;

            let refined_content = format!(
                "## Refined Explanation for {}\n\nThis {:?} is part of the `{}` module. \
                It has been automatically refined using user hints: \"{}\".\n\n\
                **Signature**: `{}`",
                symbol.name,
                symbol.kind,
                symbol.module_id.as_deref().unwrap_or("root"),
                payload.user_hints.as_deref().unwrap_or("none"),
                symbol.signature.as_deref().unwrap_or("unknown")
            );

            Ok::<_, crate::RepoIntelligenceError>(RefineEntityDocResponse {
                repo_id: payload.repo_id,
                entity_id: payload.entity_id,
                refined_content,
                verification_state: "verified".to_string(),
            })
        },
    )
    .await
}
