use xiuxian_vector::{SearchEngineContext, VectorStoreError};

use crate::gateway::studio::types::ReferenceSearchHit;
use crate::search_plane::ranking::{
    RetainedWindow, StreamingRerankSource, StreamingRerankTelemetry, sort_by_rank,
};
use crate::search_plane::reference_occurrence::schema::{filter_column, projected_columns};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::candidates::{ReferenceOccurrenceCandidate, collect_candidates, compare_candidates};
use super::decode::decode_reference_hits;

const MIN_RETAINED_REFERENCE_OCCURRENCES: usize = 64;
const RETAINED_REFERENCE_OCCURRENCE_MULTIPLIER: usize = 4;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReferenceOccurrenceSearchError {
    #[error("reference occurrence index has no published epoch")]
    NotReady,
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
    #[error("{0}")]
    Decode(String),
}

pub(crate) async fn search_reference_occurrences(
    service: &SearchPlaneService,
    query: &str,
    limit: usize,
) -> Result<Vec<ReferenceSearchHit>, ReferenceOccurrenceSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::ReferenceOccurrence);
    let Some(active_epoch) = status.active_epoch else {
        return Err(ReferenceOccurrenceSearchError::NotReady);
    };

    let normalized_query = query.trim().to_ascii_lowercase();
    if normalized_query.is_empty() {
        return Ok(Vec::new());
    }

    let parquet_path =
        service.local_epoch_parquet_path(SearchCorpusKind::ReferenceOccurrence, active_epoch);
    if !parquet_path.exists() {
        return Err(ReferenceOccurrenceSearchError::NotReady);
    }
    let table_name = SearchPlaneService::local_epoch_engine_table_name(
        SearchCorpusKind::ReferenceOccurrence,
        active_epoch,
    );
    service
        .search_engine()
        .ensure_parquet_table_registered(table_name.as_str(), parquet_path.as_path(), &[])
        .await?;
    let execution = execute_reference_occurrence_search(
        service.search_engine(),
        table_name.as_str(),
        query,
        normalized_query.as_str(),
        retained_window(limit),
    )
    .await?;
    let mut candidates = execution.candidates;
    sort_by_rank(&mut candidates, compare_candidates);
    candidates.truncate(limit);
    let hits =
        decode_reference_hits(service.search_engine(), table_name.as_str(), candidates).await?;
    service.record_query_telemetry(
        SearchCorpusKind::ReferenceOccurrence,
        execution
            .telemetry
            .finish(execution.source, Some("search".to_string()), hits.len()),
    );
    Ok(hits)
}

struct ReferenceOccurrenceSearchExecution {
    candidates: Vec<ReferenceOccurrenceCandidate>,
    telemetry: StreamingRerankTelemetry,
    source: StreamingRerankSource,
}

async fn execute_reference_occurrence_search(
    engine: &SearchEngineContext,
    table_name: &str,
    query: &str,
    normalized_query: &str,
    window: RetainedWindow,
) -> Result<ReferenceOccurrenceSearchExecution, ReferenceOccurrenceSearchError> {
    let mut telemetry = StreamingRerankTelemetry::new(window, None, None);
    let mut candidates = Vec::with_capacity(window.target);
    let sql = build_reference_occurrence_stage1_sql(table_name, normalized_query);
    let batches = engine.sql_batches(sql.as_str()).await?;
    for batch in batches {
        collect_candidates(&batch, query, &mut candidates, window, &mut telemetry)?;
    }
    Ok(ReferenceOccurrenceSearchExecution {
        candidates,
        telemetry,
        source: StreamingRerankSource::Scan,
    })
}

fn retained_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(
        limit,
        RETAINED_REFERENCE_OCCURRENCE_MULTIPLIER,
        MIN_RETAINED_REFERENCE_OCCURRENCES,
    )
}

fn build_reference_occurrence_stage1_sql(table_name: &str, normalized_query: &str) -> String {
    format!(
        "SELECT {} FROM {table_name} WHERE {} = {}",
        projected_columns().join(", "),
        filter_column(),
        sql_string_literal(normalized_query),
    )
}

fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
