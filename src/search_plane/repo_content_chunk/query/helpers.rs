use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::Path;

use arrow::array::{Array, StringArray, StringViewArray, UInt64Array};
use xiuxian_vector::EngineRecordBatch;

use crate::search_plane::repo_content_chunk::schema::{language_column, projected_columns};

use super::RepoContentChunkCandidate;
use super::RepoContentChunkSearchError;

#[derive(Clone, Copy)]
pub(crate) enum EngineStringColumn<'a> {
    Utf8(&'a StringArray),
    Utf8View(&'a StringViewArray),
}

impl<'a> EngineStringColumn<'a> {
    pub(crate) fn value(self, row: usize) -> &'a str {
        match self {
            Self::Utf8(column) => column.value(row),
            Self::Utf8View(column) => column.value(row),
        }
    }

    pub(crate) fn is_null(self, row: usize) -> bool {
        match self {
            Self::Utf8(column) => column.is_null(row),
            Self::Utf8View(column) => column.is_null(row),
        }
    }
}

pub(crate) fn compare_candidates(
    left: &RepoContentChunkCandidate,
    right: &RepoContentChunkCandidate,
) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.line_number.cmp(&right.line_number))
}

pub(crate) fn candidate_path_key(candidate: &RepoContentChunkCandidate) -> String {
    candidate.path.clone()
}

pub(crate) fn infer_code_language(path: &str) -> Option<String> {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("jl") || ext.eq_ignore_ascii_case("julia") => {
            Some("julia".to_string())
        }
        Some(ext) if ext.eq_ignore_ascii_case("mo") || ext.eq_ignore_ascii_case("modelica") => {
            Some("modelica".to_string())
        }
        Some(ext) if ext.eq_ignore_ascii_case("rs") => Some("rust".to_string()),
        Some(ext) if ext.eq_ignore_ascii_case("py") => Some("python".to_string()),
        Some(ext) if ext.eq_ignore_ascii_case("ts") || ext.eq_ignore_ascii_case("tsx") => {
            Some("typescript".to_string())
        }
        _ => None,
    }
}

pub(crate) fn truncate_content_search_snippet(value: &str, max_chars: usize) -> String {
    let truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    }
}

pub(crate) fn engine_string_column<'a>(
    batch: &'a EngineRecordBatch,
    name: &str,
) -> Result<EngineStringColumn<'a>, RepoContentChunkSearchError> {
    let column = batch.column_by_name(name).ok_or_else(|| {
        RepoContentChunkSearchError::Decode(format!("missing engine string column `{name}`"))
    })?;

    if let Some(array) = column.as_any().downcast_ref::<StringArray>() {
        return Ok(EngineStringColumn::Utf8(array));
    }
    if let Some(array) = column.as_any().downcast_ref::<StringViewArray>() {
        return Ok(EngineStringColumn::Utf8View(array));
    }

    Err(RepoContentChunkSearchError::Decode(format!(
        "engine column `{name}` is not utf8-like"
    )))
}

pub(crate) fn engine_u64_column<'a>(
    batch: &'a EngineRecordBatch,
    name: &str,
) -> Result<&'a UInt64Array, RepoContentChunkSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<UInt64Array>())
        .ok_or_else(|| {
            RepoContentChunkSearchError::Decode(format!("missing engine u64 column `{name}`"))
        })
}

pub(crate) fn projected_repo_content_columns() -> Vec<String> {
    projected_columns()
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub(crate) fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub(crate) fn language_filter_expression(language_filters: &HashSet<String>) -> Option<String> {
    if language_filters.is_empty() {
        return None;
    }

    let mut sorted = language_filters.iter().cloned().collect::<Vec<_>>();
    sorted.sort_unstable();
    Some(format!(
        "{column} IN ({values})",
        column = language_column(),
        values = sorted
            .into_iter()
            .map(|value| sql_string_literal(value.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}
