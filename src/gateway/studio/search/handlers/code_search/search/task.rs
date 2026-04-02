use tokio::task::JoinSet;

use crate::gateway::studio::router::StudioApiError;
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::SearchPlaneService;

use crate::gateway::studio::search::handlers::code_search::query::RepoSearchResultLimits;
use crate::gateway::studio::search::handlers::code_search::search::repo_search::search_repo_code_hits;
use crate::gateway::studio::search::handlers::code_search::types::RepoSearchTarget;

pub(super) fn spawn_repo_code_search_task(
    join_set: &mut JoinSet<Result<Vec<SearchHit>, StudioApiError>>,
    search_plane: SearchPlaneService,
    target: RepoSearchTarget,
    raw_query: String,
    per_repo_limits: RepoSearchResultLimits,
) {
    join_set.spawn(async move {
        search_repo_code_hits(&search_plane, &target, raw_query.as_str(), per_repo_limits).await
    });
}
