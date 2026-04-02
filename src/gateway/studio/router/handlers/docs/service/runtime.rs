use std::sync::Arc;

use crate::analyzers::{RepoIntelligenceError, RepositoryAnalysisOutput};
use crate::gateway::studio::router::GatewayState;
use crate::gateway::studio::router::StudioApiError;
use crate::gateway::studio::router::handlers::repo::shared::with_repo_analysis;

pub(super) async fn run_docs_analysis<T, F>(
    state: Arc<GatewayState>,
    repo_id: String,
    panic_code: &'static str,
    panic_message: &'static str,
    build: F,
) -> Result<T, StudioApiError>
where
    T: Send + 'static,
    F: FnOnce(RepositoryAnalysisOutput) -> Result<T, RepoIntelligenceError> + Send + 'static,
{
    with_repo_analysis(state, repo_id, panic_code, panic_message, build).await
}
