use std::collections::VecDeque;
use std::time::Duration;

use tokio::task::JoinSet;
use tokio::time::{Instant, timeout_at};

use crate::gateway::studio::router::StudioApiError;
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::SearchPlaneService;

use crate::gateway::studio::search::handlers::code_search::query::{
    RepoSearchResultLimits, repo_search_parallelism,
};
use crate::gateway::studio::search::handlers::code_search::search::task::spawn_repo_code_search_task;
use crate::gateway::studio::search::handlers::code_search::types::RepoSearchTarget;

#[derive(Debug, Default)]
pub(super) struct BufferedRepoSearchResult {
    pub(super) hits: Vec<SearchHit>,
    pub(super) partial_timeout: bool,
}

pub(super) async fn search_repo_code_hits_buffered(
    search_plane: SearchPlaneService,
    targets: Vec<RepoSearchTarget>,
    raw_query: &str,
    per_repo_limits: RepoSearchResultLimits,
    repo_wide_budget: Option<Duration>,
) -> Result<BufferedRepoSearchResult, StudioApiError> {
    if targets.is_empty() {
        return Ok(BufferedRepoSearchResult::default());
    }

    let mut queued = VecDeque::from(targets);
    let mut join_set = JoinSet::new();
    let raw_query = raw_query.to_string();
    let parallelism = repo_search_parallelism(&search_plane, queued.len());
    let deadline = repo_wide_budget.map(|budget| Instant::now() + budget);
    for _ in 0..parallelism {
        if let Some(target) = queued.pop_front() {
            spawn_repo_code_search_task(
                &mut join_set,
                search_plane.clone(),
                target,
                raw_query.clone(),
                per_repo_limits,
            );
        }
    }

    let mut hits = Vec::new();
    while !join_set.is_empty() {
        let next_result = if let Some(deadline) = deadline {
            match timeout_at(deadline, join_set.join_next()).await {
                Ok(result) => result,
                Err(_) => {
                    join_set.abort_all();
                    while join_set.join_next().await.is_some() {}
                    return Ok(BufferedRepoSearchResult {
                        hits,
                        partial_timeout: true,
                    });
                }
            }
        } else {
            join_set.join_next().await
        };
        let Some(result) = next_result else {
            break;
        };
        let repository_hits = result.map_err(|error| {
            StudioApiError::internal(
                "REPO_CODE_SEARCH_TASK_FAILED",
                "Repo code-search task failed",
                Some(error.to_string()),
            )
        })??;
        hits.extend(repository_hits);
        if let Some(target) = queued.pop_front() {
            spawn_repo_code_search_task(
                &mut join_set,
                search_plane.clone(),
                target,
                raw_query.clone(),
                per_repo_limits,
            );
        }
    }
    Ok(BufferedRepoSearchResult {
        hits,
        partial_timeout: false,
    })
}
