use std::collections::HashSet;

use arrow::array::{Array, StringArray, StringViewArray};
use xiuxian_vector::{EngineRecordBatch, SearchEngineContext};

use crate::search_plane::attachment::query::scoring::{candidate_score, compare_candidates};
use crate::search_plane::attachment::query::types::{
    AttachmentCandidate, AttachmentCandidateQuery, AttachmentSearchError, AttachmentSearchExecution,
};
use crate::search_plane::attachment::schema::{
    attachment_ext_column, attachment_name_column, attachment_name_folded_column, id_column,
    kind_column, projected_columns,
};
use crate::search_plane::ranking::{
    StreamingRerankSource, StreamingRerankTelemetry, trim_ranked_vec,
};

#[derive(Clone, Copy)]
enum EngineStringColumn<'a> {
    Utf8(&'a StringArray),
    Utf8View(&'a StringViewArray),
}

impl<'a> EngineStringColumn<'a> {
    fn value(self, row: usize) -> &'a str {
        match self {
            Self::Utf8(column) => column.value(row),
            Self::Utf8View(column) => column.value(row),
        }
    }
}

pub(crate) async fn execute_attachment_search(
    engine: &SearchEngineContext,
    table_name: &str,
    candidate_query: &AttachmentCandidateQuery<'_>,
) -> Result<AttachmentSearchExecution, AttachmentSearchError> {
    let sql = build_attachment_stage1_sql(
        table_name,
        candidate_query.extensions,
        candidate_query.kinds,
    );
    let batches = engine.sql_batches(sql.as_str()).await?;
    let mut telemetry = StreamingRerankTelemetry::new(candidate_query.window, None, None);
    let mut candidates = Vec::with_capacity(candidate_query.window.target);

    for batch in batches {
        collect_candidates(&batch, candidate_query, &mut candidates, &mut telemetry)?;
    }

    Ok(AttachmentSearchExecution {
        candidates,
        telemetry,
        source: StreamingRerankSource::Scan,
    })
}

pub(crate) fn build_attachment_stage1_sql(
    table_name: &str,
    normalized_extensions: &HashSet<String>,
    normalized_kinds: &HashSet<String>,
) -> String {
    let projections = projected_columns().join(", ");
    match filter_expression(normalized_extensions, normalized_kinds) {
        Some(filter) => format!("SELECT {projections} FROM {table_name} WHERE {filter}"),
        None => format!("SELECT {projections} FROM {table_name}"),
    }
}

fn collect_candidates(
    batch: &EngineRecordBatch,
    query: &AttachmentCandidateQuery<'_>,
    candidates: &mut Vec<AttachmentCandidate>,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), AttachmentSearchError> {
    telemetry.observe_batch(batch.num_rows());
    let id = string_column(batch, id_column())?;
    let source_path = string_column(batch, "source_path")?;
    let source_title = string_column(batch, "source_title")?;
    let source_stem = string_column(batch, "source_stem")?;
    let attachment_path = string_column(batch, "attachment_path")?;
    let attachment_name = string_column(batch, attachment_name_column())?;
    let source_path_folded = string_column(batch, "source_path_folded")?;
    let source_title_folded = string_column(batch, "source_title_folded")?;
    let source_stem_folded = string_column(batch, "source_stem_folded")?;
    let attachment_path_folded = string_column(batch, "attachment_path_folded")?;
    let attachment_name_folded = string_column(batch, attachment_name_folded_column())?;

    for row in 0..batch.num_rows() {
        let fields = if query.case_sensitive {
            [
                attachment_path.value(row),
                attachment_name.value(row),
                source_path.value(row),
                source_title.value(row),
                source_stem.value(row),
            ]
        } else {
            [
                attachment_path_folded.value(row),
                attachment_name_folded.value(row),
                source_path_folded.value(row),
                source_title_folded.value(row),
                source_stem_folded.value(row),
            ]
        };
        let score = candidate_score(query.normalized_query, query.query_tokens, &fields);
        if score <= 0.0 {
            continue;
        }

        telemetry.observe_match();
        candidates.push(AttachmentCandidate {
            id: id.value(row).to_string(),
            score,
            source_path: source_path.value(row).to_string(),
            attachment_path: attachment_path.value(row).to_string(),
        });
        telemetry.observe_working_set(candidates.len());
        if candidates.len() > query.window.threshold {
            let before_len = candidates.len();
            trim_ranked_vec(candidates, query.window.target, compare_candidates);
            telemetry.observe_trim(before_len, candidates.len());
        }
    }

    Ok(())
}

fn filter_expression(extensions: &HashSet<String>, kinds: &HashSet<String>) -> Option<String> {
    let extension_clause = disjunction(attachment_ext_column(), extensions);
    let kind_clause = disjunction(kind_column(), kinds);
    match (extension_clause, kind_clause) {
        (Some(left), Some(right)) => Some(format!("({left}) AND ({right})")),
        (Some(clause), None) | (None, Some(clause)) => Some(clause),
        (None, None) => None,
    }
}

fn disjunction(column: &str, values: &HashSet<String>) -> Option<String> {
    if values.is_empty() {
        return None;
    }

    let mut sorted = values.iter().cloned().collect::<Vec<_>>();
    sorted.sort_unstable();
    Some(format!(
        "{column} IN ({})",
        sorted
            .into_iter()
            .map(|value| sql_string_literal(value.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn string_column<'a>(
    batch: &'a EngineRecordBatch,
    name: &str,
) -> Result<EngineStringColumn<'a>, AttachmentSearchError> {
    let column = batch.column_by_name(name).ok_or_else(|| {
        AttachmentSearchError::Decode(format!("missing engine string column `{name}`"))
    })?;
    if let Some(array) = column.as_any().downcast_ref::<StringArray>() {
        return Ok(EngineStringColumn::Utf8(array));
    }
    if let Some(array) = column.as_any().downcast_ref::<StringViewArray>() {
        return Ok(EngineStringColumn::Utf8View(array));
    }
    Err(AttachmentSearchError::Decode(format!(
        "engine column `{name}` is not utf8-like"
    )))
}
