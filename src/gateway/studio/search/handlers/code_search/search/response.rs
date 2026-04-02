use std::time::Duration;

use crate::gateway::studio::router::{
    StudioApiError, StudioState, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
use crate::gateway::studio::types::SearchResponse;
use crate::search_plane::{RepoSearchQueryCacheKeyInput, SearchCorpusKind, SearchPlaneCacheTtl};

use crate::gateway::studio::search::handlers::code_search::query::{
    collect_repo_search_targets, infer_repo_hint_from_query, parse_code_search_query,
    repo_search_parallelism, repo_search_result_limits, repo_wide_code_search_timeout,
};
use crate::gateway::studio::search::handlers::code_search::search::buffered::search_repo_code_hits_buffered;

/// Build one code-search response from the Studio search plane.
///
/// # Errors
///
/// Returns [`StudioApiError`] when repository configuration is invalid or the repo-backed search
/// plane encounters a failure while producing the response payload.
#[allow(clippy::too_many_lines)]
pub(crate) async fn build_code_search_response(
    studio: &StudioState,
    raw_query: String,
    repo_hint: Option<&str>,
    limit: usize,
) -> Result<SearchResponse, StudioApiError> {
    build_code_search_response_with_budget(studio, raw_query, repo_hint, limit, None).await
}

/// Build one code-search response with an optional repository-wide timeout budget.
///
/// # Errors
///
/// Returns [`StudioApiError`] when repository configuration is invalid or the repo-backed search
/// plane encounters a failure while producing the response payload.
#[allow(clippy::too_many_lines)]
pub(crate) async fn build_code_search_response_with_budget(
    studio: &StudioState,
    raw_query: String,
    repo_hint: Option<&str>,
    limit: usize,
    repo_wide_budget: Option<Duration>,
) -> Result<SearchResponse, StudioApiError> {
    let mut parsed = parse_code_search_query(raw_query.as_str(), repo_hint);
    let configured_repositories = configured_repositories(studio);
    if parsed.repo.is_none() {
        parsed.repo = infer_repo_hint_from_query(
            &parsed,
            configured_repositories
                .iter()
                .map(|repository| repository.id.as_str()),
        );
    }
    let effective_repo_hint = parsed.repo.as_deref();
    let effective_repo_wide_budget = if effective_repo_hint.is_some() {
        None
    } else {
        repo_wide_budget.or_else(|| repo_wide_code_search_timeout(None))
    };
    let repo_ids = if let Some(repo_id) = effective_repo_hint {
        vec![
            configured_repository(studio, repo_id)
                .map_err(map_repo_intelligence_error)?
                .id,
        ]
    } else {
        configured_repositories
            .into_iter()
            .map(|repository| repository.id)
            .collect()
    };

    if repo_ids.is_empty() {
        return Err(StudioApiError::bad_request(
            "UNKNOWN_REPOSITORY",
            "No configured repository is available for code search",
        ));
    }
    let cache_key = studio
        .search_plane
        .repo_search_query_cache_key(RepoSearchQueryCacheKeyInput {
            scope: "code_search",
            corpora: &[],
            repo_corpora: &[
                SearchCorpusKind::RepoEntity,
                SearchCorpusKind::RepoContentChunk,
            ],
            repo_ids: repo_ids.as_slice(),
            query: raw_query.as_str(),
            limit,
            intent: Some("code_search"),
            repo_hint: effective_repo_hint,
        })
        .await;
    if let Some(cache_key) = cache_key.as_ref()
        && let Some(cached) = studio
            .search_plane
            .cache_get_json::<SearchResponse>(cache_key)
            .await
    {
        return Ok(cached);
    }
    let mut hits = Vec::new();
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
    let pending_repos = dispatch.pending_repos;
    let skipped_repos = dispatch.skipped_repos;
    let buffered = search_repo_code_hits_buffered(
        studio.search_plane.clone(),
        dispatch.searchable_repos,
        raw_query.as_str(),
        repo_search_result_limits(effective_repo_hint, limit),
        effective_repo_wide_budget,
    )
    .await?;
    let partial_timeout = buffered.partial_timeout;
    hits.extend(buffered.hits);

    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.stem.cmp(&right.stem))
    });
    hits.truncate(limit);

    let hit_count = hits.len();
    let indexing_state = if partial_timeout {
        "partial".to_string()
    } else if pending_repos.is_empty() {
        "ready".to_string()
    } else if hit_count == 0 {
        "indexing".to_string()
    } else {
        "partial".to_string()
    };

    let response = SearchResponse {
        query: raw_query,
        hit_count,
        hits,
        graph_confidence_score: None,
        selected_mode: Some("code_search".to_string()),
        intent: Some("code_search".to_string()),
        intent_confidence: Some(1.0),
        search_mode: Some("code_search".to_string()),
        partial: partial_timeout || !pending_repos.is_empty() || !skipped_repos.is_empty(),
        indexing_state: Some(indexing_state),
        pending_repos,
        skipped_repos,
    };
    if let Some(cache_key) = cache_key.as_ref() {
        studio
            .search_plane
            .cache_set_json(cache_key, SearchPlaneCacheTtl::HotQuery, &response)
            .await;
    }
    Ok(response)
}
