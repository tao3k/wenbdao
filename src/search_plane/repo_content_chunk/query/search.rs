use std::collections::HashSet;

use crate::gateway::studio::types::SearchHit;
use crate::search_plane::ranking::sort_by_rank;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::RepoContentChunkSearchError;
use super::compare_candidates;
use super::execution::execute_repo_content_search;
use super::retained_window;

pub(crate) async fn search_repo_content_chunks(
    service: &SearchPlaneService,
    repo_id: &str,
    search_term: &str,
    language_filters: &HashSet<String>,
    limit: usize,
) -> Result<Vec<SearchHit>, RepoContentChunkSearchError> {
    let trimmed = search_term.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let _read_permit = service.acquire_repo_search_read_permit().await?;
    let Some(publication) = service
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, repo_id)
        .await
        .and_then(|record| record.publication)
    else {
        return Ok(Vec::new());
    };
    if !publication.is_datafusion_readable() {
        return Ok(Vec::new());
    }

    let parquet_path = service.repo_publication_parquet_path(
        SearchCorpusKind::RepoContentChunk,
        publication.table_name.as_str(),
    );
    if !parquet_path.exists() {
        return Ok(Vec::new());
    }
    let engine_table_name = SearchPlaneService::repo_publication_engine_table_name(
        SearchCorpusKind::RepoContentChunk,
        publication.publication_id.as_str(),
    );
    service
        .search_engine()
        .ensure_parquet_table_registered(engine_table_name.as_str(), parquet_path.as_path(), &[])
        .await?;

    let execution = execute_repo_content_search(
        service.search_engine(),
        engine_table_name.as_str(),
        trimmed,
        language_filters,
        retained_window(limit),
    )
    .await?;
    let mut hits = execution.candidates;
    sort_by_rank(&mut hits, compare_candidates);
    hits.truncate(limit);
    let hits = hits
        .into_iter()
        .map(|candidate| candidate.into_search_hit(repo_id))
        .collect::<Vec<_>>();
    service.record_query_telemetry(
        SearchCorpusKind::RepoContentChunk,
        execution
            .telemetry
            .finish(execution.source, Some(repo_id.to_string()), hits.len()),
    );
    Ok(hits)
}
