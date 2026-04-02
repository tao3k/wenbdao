use std::path::Path;
use std::sync::Arc;

use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::query::RepoSyncResult;
use crate::analyzers::{
    RegisteredRepository, RepoSyncMode, RepoSyncQuery, analyze_registered_repository_with_registry,
    repo_sync_for_registered_repository,
};
use crate::gateway::studio::repo_index::state::collect::{
    await_analysis_completion, collect_code_documents,
};
use crate::gateway::studio::repo_index::state::coordinator::RepoIndexCoordinator;
use crate::gateway::studio::repo_index::state::task::REPO_INDEX_ANALYSIS_TIMEOUT;
use crate::gateway::studio::repo_index::types::RepoCodeDocument;

impl RepoIndexCoordinator {
    pub(crate) async fn run_repository_analysis(
        &self,
        repository: RegisteredRepository,
    ) -> Result<crate::analyzers::RepositoryAnalysisOutput, RepoIntelligenceError> {
        let repo_id = repository.id.clone();
        let project_root = self.project_root.clone();
        let plugin_registry = Arc::clone(&self.plugin_registry);
        let task = tokio::task::spawn_blocking(move || {
            analyze_registered_repository_with_registry(
                &repository,
                project_root.as_path(),
                plugin_registry.as_ref(),
            )
        });
        await_analysis_completion(repo_id.as_str(), task, REPO_INDEX_ANALYSIS_TIMEOUT).await
    }

    pub(crate) async fn run_repository_sync(
        &self,
        repo_id: &str,
        repository: RegisteredRepository,
        refresh: bool,
    ) -> Result<RepoSyncResult, RepoIntelligenceError> {
        let repo_id = repo_id.to_string();
        let repo_id_for_worker = repo_id.clone();
        let project_root = self.project_root.clone();
        let mode = if refresh {
            RepoSyncMode::Refresh
        } else {
            RepoSyncMode::Ensure
        };
        let permit = self.acquire_sync_permit(repo_id.as_str()).await?;
        self.bump_status(
            repo_id.as_str(),
            crate::gateway::studio::repo_index::types::RepoIndexPhase::Syncing,
            None,
            None,
        );
        let task = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            repo_sync_for_registered_repository(
                &RepoSyncQuery {
                    repo_id: repo_id_for_worker,
                    mode,
                },
                &repository,
                project_root.as_path(),
            )
        });
        match task.await {
            Ok(result) => result,
            Err(error) => Err(RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo sync worker for `{repo_id}` terminated unexpectedly: {error}"
                ),
            }),
        }
    }

    pub(crate) async fn collect_code_documents_for_task(
        &self,
        repo_id: &str,
        fingerprint: &str,
        checkout_path: &str,
    ) -> Result<Option<Vec<RepoCodeDocument>>, RepoIntelligenceError> {
        let repo_id = repo_id.to_string();
        let fingerprint = fingerprint.to_string();
        let checkout_path = checkout_path.to_string();
        let fingerprints = Arc::clone(&self.fingerprints);
        let repo_id_for_error = repo_id.clone();
        let repo_id_for_worker = repo_id.clone();
        let task = tokio::task::spawn_blocking(move || {
            Ok::<Option<Vec<RepoCodeDocument>>, RepoIntelligenceError>(collect_code_documents(
                Path::new(checkout_path.as_str()),
                || {
                    let current = fingerprints
                        .read()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .get(&repo_id_for_worker)
                        .cloned();
                    current.as_deref() != Some(fingerprint.as_str())
                },
            ))
        });

        match tokio::time::timeout(REPO_INDEX_ANALYSIS_TIMEOUT, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo `{repo_id_for_error}` code document worker terminated unexpectedly: {error}"
                ),
            }),
            Err(_) => Err(RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo `{repo_id_for_error}` code document collection timed out after {}s while indexing was running",
                    REPO_INDEX_ANALYSIS_TIMEOUT.as_secs()
                ),
            }),
        }
    }
}
