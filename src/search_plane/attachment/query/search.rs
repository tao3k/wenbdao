use std::collections::BTreeMap;

use crate::gateway::studio::types::AttachmentSearchHit;
use crate::search_plane::attachment::query::scan::execute_attachment_search;
use crate::search_plane::attachment::query::scoring::{
    build_query_tokens, compare_candidates, normalize_extension_filters, normalize_kind_filters,
    retained_window,
};
use crate::search_plane::attachment::query::types::{
    AttachmentCandidate, AttachmentCandidateQuery, AttachmentSearchError,
};
use crate::search_plane::ranking::sort_by_rank;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};
use xiuxian_vector::SearchEngineContext;

use crate::search_plane::attachment::schema::{hit_json_column, id_column};

pub(crate) async fn search_attachment_hits(
    service: &SearchPlaneService,
    query: &str,
    limit: usize,
    extensions: &[String],
    kinds: &[crate::link_graph::LinkGraphAttachmentKind],
    case_sensitive: bool,
) -> Result<Vec<AttachmentSearchHit>, AttachmentSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::Attachment);
    let Some(active_epoch) = status.active_epoch else {
        return Err(AttachmentSearchError::NotReady);
    };

    let query_text = query.trim();
    if query_text.is_empty() {
        return Ok(Vec::new());
    }

    let normalized_extensions = normalize_extension_filters(extensions);
    let normalized_kinds = normalize_kind_filters(kinds);

    let parquet_path = service.local_epoch_parquet_path(SearchCorpusKind::Attachment, active_epoch);
    if !parquet_path.exists() {
        return Err(AttachmentSearchError::NotReady);
    }
    let engine_table_name = SearchPlaneService::local_epoch_engine_table_name(
        SearchCorpusKind::Attachment,
        active_epoch,
    );
    service
        .search_engine()
        .ensure_parquet_table_registered(engine_table_name.as_str(), parquet_path.as_path(), &[])
        .await?;

    let normalized_query = if case_sensitive {
        query_text.to_string()
    } else {
        query_text.to_ascii_lowercase()
    };
    let query_tokens = build_query_tokens(normalized_query.as_str());
    let candidate_query = AttachmentCandidateQuery {
        case_sensitive,
        normalized_query: normalized_query.as_str(),
        query_tokens: query_tokens.as_slice(),
        extensions: &normalized_extensions,
        kinds: &normalized_kinds,
        window: retained_window(limit),
    };
    let execution = execute_attachment_search(
        service.search_engine(),
        engine_table_name.as_str(),
        &candidate_query,
    )
    .await?;
    let mut candidates = execution.candidates;
    sort_by_rank(&mut candidates, compare_candidates);
    candidates.truncate(limit);
    let hits = decode_attachment_hits(
        service.search_engine(),
        engine_table_name.as_str(),
        candidates,
    )
    .await?;
    service.record_query_telemetry(
        SearchCorpusKind::Attachment,
        execution
            .telemetry
            .finish(execution.source, None, hits.len()),
    );
    Ok(hits)
}

async fn decode_attachment_hits(
    engine: &SearchEngineContext,
    table_name: &str,
    candidates: Vec<AttachmentCandidate>,
) -> Result<Vec<AttachmentSearchHit>, AttachmentSearchError> {
    let payloads = load_hit_payloads_by_id(engine, table_name, candidates.as_slice()).await?;
    candidates
        .into_iter()
        .map(|candidate| {
            let hit_json = payloads.get(candidate.id.as_str()).ok_or_else(|| {
                AttachmentSearchError::Decode(format!(
                    "attachment hydration missing payload for id `{}`",
                    candidate.id
                ))
            })?;
            let mut hit: AttachmentSearchHit = serde_json::from_str(hit_json.as_str())
                .map_err(|error| AttachmentSearchError::Decode(error.to_string()))?;
            hit.score = candidate.score;
            Ok(hit)
        })
        .collect()
}

async fn load_hit_payloads_by_id(
    engine: &SearchEngineContext,
    table_name: &str,
    candidates: &[AttachmentCandidate],
) -> Result<BTreeMap<String, String>, AttachmentSearchError> {
    if candidates.is_empty() {
        return Ok(BTreeMap::new());
    }

    let sql = format!(
        "SELECT {id_column}, {hit_json_column} FROM {table_name} WHERE {id_column} IN ({ids})",
        id_column = id_column(),
        hit_json_column = hit_json_column(),
        ids = candidates
            .iter()
            .map(|candidate| sql_string_literal(candidate.id.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut payloads = BTreeMap::new();
    let batches = engine.sql_batches(sql.as_str()).await?;

    for batch in batches {
        let id = batch
            .column_by_name(id_column())
            .ok_or_else(|| AttachmentSearchError::Decode("missing engine id column".to_string()))?;
        let hit_json = batch.column_by_name(hit_json_column()).ok_or_else(|| {
            AttachmentSearchError::Decode("missing engine hit_json column".to_string())
        })?;
        let id = if let Some(array) = id.as_any().downcast_ref::<arrow::array::StringArray>() {
            EitherString::Utf8(array)
        } else if let Some(array) = id.as_any().downcast_ref::<arrow::array::StringViewArray>() {
            EitherString::Utf8View(array)
        } else {
            return Err(AttachmentSearchError::Decode(
                "engine id column is not utf8-like".to_string(),
            ));
        };
        let hit_json = if let Some(array) = hit_json
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
        {
            EitherString::Utf8(array)
        } else if let Some(array) = hit_json
            .as_any()
            .downcast_ref::<arrow::array::StringViewArray>()
        {
            EitherString::Utf8View(array)
        } else {
            return Err(AttachmentSearchError::Decode(
                "engine hit_json column is not utf8-like".to_string(),
            ));
        };
        for row in 0..batch.num_rows() {
            payloads.insert(id.value(row).to_string(), hit_json.value(row).to_string());
        }
    }

    Ok(payloads)
}

fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

enum EitherString<'a> {
    Utf8(&'a arrow::array::StringArray),
    Utf8View(&'a arrow::array::StringViewArray),
}

impl<'a> EitherString<'a> {
    fn value(&self, row: usize) -> &str {
        match self {
            Self::Utf8(column) => column.value(row),
            Self::Utf8View(column) => column.value(row),
        }
    }
}
