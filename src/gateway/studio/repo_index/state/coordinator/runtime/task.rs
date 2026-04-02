use std::sync::Arc;
use std::time::Instant;

use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::query::{RepoSourceKind, RepoSyncResult};
use crate::gateway::studio::repo_index::state::coordinator::RepoIndexCoordinator;
use crate::gateway::studio::repo_index::state::task::{
    RepoIndexTask, RepoTaskFeedback, RepoTaskOutcome, should_retry_sync_failure,
};
use crate::gateway::studio::repo_index::types::RepoIndexPhase;
use crate::search_plane::{SearchCorpusKind, SearchRepoCorpusRecord};

impl RepoIndexCoordinator {
    pub(crate) async fn process_task(self: Arc<Self>, task: RepoIndexTask) -> RepoTaskFeedback {
        let repo_id = task.repository.id.clone();
        let started_at = Instant::now();
        if !self.fingerprint_matches(repo_id.as_str(), task.fingerprint.as_str()) {
            return RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Skipped,
            };
        }

        self.bump_status(&repo_id, RepoIndexPhase::Checking, None, None);

        let sync_result = match self
            .run_repository_sync(repo_id.as_str(), task.repository.clone(), task.refresh)
            .await
        {
            Ok(result) => result,
            Err(error) if should_retry_sync_failure(&error, task.retry_count) => {
                let mut retry_task = task.clone();
                retry_task.retry_count = retry_task.retry_count.saturating_add(1);
                return RepoTaskFeedback {
                    repo_id,
                    elapsed: started_at.elapsed(),
                    outcome: RepoTaskOutcome::Requeued {
                        task: retry_task,
                        error,
                    },
                };
            }
            Err(error) => {
                return RepoTaskFeedback {
                    repo_id,
                    elapsed: started_at.elapsed(),
                    outcome: RepoTaskOutcome::Failure {
                        revision: None,
                        error,
                    },
                };
            }
        };

        if !self.fingerprint_matches(repo_id.as_str(), task.fingerprint.as_str()) {
            return RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Skipped,
            };
        }

        self.bump_status(
            &repo_id,
            RepoIndexPhase::Indexing,
            sync_result.revision.clone(),
            None,
        );

        if self
            .repo_publications_are_current(repo_id.as_str(), &sync_result)
            .await
        {
            return RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Success {
                    revision: sync_result.revision,
                },
            };
        }

        match self.run_repository_analysis(task.repository.clone()).await {
            Ok(analysis) => {
                self.complete_indexing(repo_id, started_at, task, sync_result, analysis)
                    .await
            }
            Err(error) => RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Failure {
                    revision: sync_result.revision,
                    error,
                },
            },
        }
    }

    pub(crate) async fn repo_publications_are_current(
        &self,
        repo_id: &str,
        sync_result: &RepoSyncResult,
    ) -> bool {
        let Some(revision) = sync_result.revision.as_deref() else {
            return false;
        };
        if sync_result.source_kind != RepoSourceKind::ManagedRemote {
            return false;
        }

        let (entity_record, content_record) = tokio::join!(
            self.search_plane
                .repo_corpus_record_for_reads(SearchCorpusKind::RepoEntity, repo_id),
            self.search_plane
                .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, repo_id)
        );

        repo_publication_matches_revision(entity_record.as_ref(), revision)
            && repo_publication_matches_revision(content_record.as_ref(), revision)
    }

    async fn complete_indexing(
        &self,
        repo_id: String,
        started_at: Instant,
        task: RepoIndexTask,
        sync_result: RepoSyncResult,
        analysis: crate::analyzers::RepositoryAnalysisOutput,
    ) -> RepoTaskFeedback {
        if !self.fingerprint_matches(repo_id.as_str(), task.fingerprint.as_str()) {
            return RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Skipped,
            };
        }

        let code_documents = match self
            .collect_code_documents_for_task(
                repo_id.as_str(),
                task.fingerprint.as_str(),
                sync_result.checkout_path.as_str(),
            )
            .await
        {
            Ok(Some(code_documents)) => code_documents,
            Ok(None) => {
                return RepoTaskFeedback {
                    repo_id,
                    elapsed: started_at.elapsed(),
                    outcome: RepoTaskOutcome::Skipped,
                };
            }
            Err(error) => {
                return RepoTaskFeedback {
                    repo_id,
                    elapsed: started_at.elapsed(),
                    outcome: RepoTaskOutcome::Failure {
                        revision: sync_result.revision,
                        error,
                    },
                };
            }
        };

        if !self.fingerprint_matches(repo_id.as_str(), task.fingerprint.as_str()) {
            return RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Skipped,
            };
        }

        if let Err(error) = self
            .search_plane
            .publish_repo_entities_with_revision(
                repo_id.as_str(),
                &analysis,
                &code_documents,
                sync_result.revision.as_deref(),
            )
            .await
        {
            let failed_repo_id = repo_id.clone();
            return RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Failure {
                    revision: sync_result.revision,
                    error: RepoIntelligenceError::AnalysisFailed {
                        message: format!(
                            "repo `{failed_repo_id}` repo-entity publish failed: {error}"
                        ),
                    },
                },
            };
        }

        if let Err(error) = self
            .search_plane
            .publish_repo_content_chunks_with_revision(
                repo_id.as_str(),
                &code_documents,
                sync_result.revision.as_deref(),
            )
            .await
        {
            let failed_repo_id = repo_id.clone();
            return RepoTaskFeedback {
                repo_id,
                elapsed: started_at.elapsed(),
                outcome: RepoTaskOutcome::Failure {
                    revision: sync_result.revision,
                    error: RepoIntelligenceError::AnalysisFailed {
                        message: format!(
                            "repo `{failed_repo_id}` repo-content chunk publish failed: {error}"
                        ),
                    },
                },
            };
        }

        RepoTaskFeedback {
            repo_id: repo_id.clone(),
            elapsed: started_at.elapsed(),
            outcome: RepoTaskOutcome::Success {
                revision: sync_result.revision,
            },
        }
    }
}

fn repo_publication_matches_revision(
    record: Option<&SearchRepoCorpusRecord>,
    revision: &str,
) -> bool {
    record
        .and_then(|record| record.publication.as_ref())
        .is_some_and(|publication| {
            publication.source_revision.as_deref() == Some(revision)
                && publication.is_datafusion_readable()
        })
}
