use crate::gateway::studio::repo_index::types::{
    RepoIndexEntryStatus, RepoIndexPhase, RepoIndexStatusResponse,
};

use super::task::AdaptiveConcurrencySnapshot;

pub(super) fn filter_status_response(
    snapshot: RepoIndexStatusResponse,
    repo_id: Option<&str>,
) -> RepoIndexStatusResponse {
    let Some(repo_id) = repo_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return snapshot;
    };
    let repos = snapshot
        .repos
        .into_iter()
        .filter(|status| repo_id_matches(status.repo_id.as_str(), repo_id))
        .collect::<Vec<_>>();
    let active_repo_ids = snapshot
        .active_repo_ids
        .into_iter()
        .filter(|active| repo_id_matches(active.as_str(), repo_id))
        .collect::<Vec<_>>();
    aggregate_status_response(
        repos,
        active_repo_ids,
        AdaptiveConcurrencySnapshot {
            current_limit: snapshot.target_concurrency,
            max_limit: snapshot.max_concurrency,
        },
        snapshot.sync_concurrency_limit,
    )
}

pub(super) fn aggregate_status_response(
    repos: Vec<RepoIndexEntryStatus>,
    active_repo_ids: Vec<String>,
    concurrency: AdaptiveConcurrencySnapshot,
    sync_concurrency_limit: usize,
) -> RepoIndexStatusResponse {
    let mut response = RepoIndexStatusResponse {
        total: repos.len(),
        active: active_repo_ids.len(),
        target_concurrency: concurrency.current_limit,
        max_concurrency: concurrency.max_limit,
        sync_concurrency_limit,
        current_repo_id: active_repo_ids.first().cloned(),
        active_repo_ids,
        repos,
        ..RepoIndexStatusResponse::default()
    };
    for status in &response.repos {
        match status.phase {
            RepoIndexPhase::Idle => {}
            RepoIndexPhase::Queued => response.queued += 1,
            RepoIndexPhase::Checking => response.checking += 1,
            RepoIndexPhase::Syncing => response.syncing += 1,
            RepoIndexPhase::Indexing => response.indexing += 1,
            RepoIndexPhase::Ready => response.ready += 1,
            RepoIndexPhase::Unsupported => response.unsupported += 1,
            RepoIndexPhase::Failed => response.failed += 1,
        }
    }
    response
}

fn repo_id_matches(candidate: &str, requested: &str) -> bool {
    candidate == requested || candidate.eq_ignore_ascii_case(requested)
}
