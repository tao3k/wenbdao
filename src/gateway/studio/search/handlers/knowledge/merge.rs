use std::collections::VecDeque;

use tokio::task::JoinSet;

use crate::gateway::studio::router::{
    StudioApiError, StudioState, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
use crate::gateway::studio::search::handlers::code_search::{
    query::{collect_repo_search_targets, repo_search_parallelism},
    search::{search_repo_content_hits, search_repo_entity_hits},
    types::RepoSearchTarget,
};
use crate::gateway::studio::search::handlers::knowledge::intent::IntentSearchTransportMetadata;
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::SearchPlaneService;

#[derive(Debug, Default)]
pub(super) struct RepoIntentMerge {
    pub(super) hits: Vec<SearchHit>,
    pub(super) transport: IntentSearchTransportMetadata,
    pub(super) pending_repos: Vec<String>,
    pub(super) skipped_repos: Vec<String>,
}

pub(super) async fn build_repo_intent_merge(
    studio: &StudioState,
    raw_query: &str,
    repo_hint: Option<&str>,
    limit: usize,
) -> Result<RepoIntentMerge, StudioApiError> {
    let repo_ids = if let Some(repo_id) = repo_hint {
        vec![
            configured_repository(studio, repo_id)
                .map_err(map_repo_intelligence_error)?
                .id,
        ]
    } else {
        configured_repositories(studio)
            .into_iter()
            .map(|repository| repository.id)
            .collect::<Vec<_>>()
    };

    let publication_states = studio
        .search_plane
        .repo_search_publication_states(repo_ids.as_slice())
        .await;
    let dispatch = collect_repo_search_targets(repo_ids, &publication_states);
    studio.search_plane.record_repo_search_dispatch(
        dispatch.pending_repos.len()
            + dispatch.skipped_repos.len()
            + dispatch.searchable_repos.len(),
        dispatch.searchable_repos.len(),
        repo_search_parallelism(&studio.search_plane, dispatch.searchable_repos.len()),
    );
    let merge = RepoIntentMerge {
        transport: IntentSearchTransportMetadata {
            #[cfg(test)]
            repo_content_transport: dispatch
                .searchable_repos
                .iter()
                .any(|target| target.publication_state.content_published)
                .then_some("flight_contract"),
        },
        hits: search_repo_intent_hits_buffered(
            studio.search_plane.clone(),
            dispatch.searchable_repos,
            raw_query,
            limit,
        )
        .await?,
        pending_repos: dispatch.pending_repos,
        skipped_repos: dispatch.skipped_repos,
    };

    Ok(merge)
}

async fn search_repo_intent_hits_buffered(
    search_plane: SearchPlaneService,
    targets: Vec<RepoSearchTarget>,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    if targets.is_empty() {
        return Ok(Vec::new());
    }

    let mut queued = VecDeque::from(targets);
    let mut join_set = JoinSet::new();
    let raw_query = raw_query.to_string();
    let parallelism = repo_search_parallelism(&search_plane, queued.len());
    for _ in 0..parallelism {
        if let Some(target) = queued.pop_front() {
            spawn_repo_intent_search_task(
                &mut join_set,
                search_plane.clone(),
                target,
                raw_query.clone(),
                limit,
            );
        }
    }

    let mut hits = Vec::new();
    while let Some(result) = join_set.join_next().await {
        let repository_hits = result.map_err(|error| {
            StudioApiError::internal(
                "REPO_INTENT_SEARCH_TASK_FAILED",
                "Repo intent-search task failed",
                Some(error.to_string()),
            )
        })??;
        hits.extend(repository_hits);
        if let Some(target) = queued.pop_front() {
            spawn_repo_intent_search_task(
                &mut join_set,
                search_plane.clone(),
                target,
                raw_query.clone(),
                limit,
            );
        }
    }
    Ok(hits)
}

fn spawn_repo_intent_search_task(
    join_set: &mut JoinSet<Result<Vec<SearchHit>, StudioApiError>>,
    search_plane: SearchPlaneService,
    target: RepoSearchTarget,
    raw_query: String,
    limit: usize,
) {
    join_set.spawn(async move {
        let mut hits = Vec::new();
        if target.publication_state.entity_published {
            hits.extend(
                search_repo_entity_hits(
                    &search_plane,
                    target.repo_id.as_str(),
                    raw_query.as_str(),
                    limit,
                )
                .await?,
            );
        }
        if target.publication_state.content_published {
            hits.extend(
                search_repo_content_hits(
                    &search_plane,
                    target.repo_id.as_str(),
                    raw_query.as_str(),
                    limit,
                )
                .await?,
            );
        }
        Ok(hits)
    });
}
