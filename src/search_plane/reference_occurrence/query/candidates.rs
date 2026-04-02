use std::cmp::Ordering;

use arrow::array::{Array, StringArray, StringViewArray, UInt64Array};
use xiuxian_vector::EngineRecordBatch;

use crate::gateway::studio::search::support::score_reference_hit;
use crate::search_plane::ranking::{RetainedWindow, StreamingRerankTelemetry, trim_ranked_vec};
use crate::search_plane::reference_occurrence::ReferenceOccurrenceSearchError;

#[derive(Debug)]
pub(super) struct ReferenceOccurrenceCandidate {
    pub(super) id: String,
    pub(super) score: f64,
    pub(super) path: String,
    pub(super) line: usize,
    pub(super) column: usize,
}

pub(super) fn collect_candidates(
    batch: &EngineRecordBatch,
    query: &str,
    candidates: &mut Vec<ReferenceOccurrenceCandidate>,
    window: RetainedWindow,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), ReferenceOccurrenceSearchError> {
    telemetry.observe_batch(batch.num_rows());
    let id = string_column(batch, "id")?;
    let path = string_column(batch, "path")?;
    let line = u64_column(batch, "line")?;
    let column = u64_column(batch, "column")?;
    let line_text = string_column(batch, "line_text")?;

    for row in 0..batch.num_rows() {
        let score = score_reference_hit(line_text.value(row), query);
        if score <= 0.0 {
            continue;
        }

        telemetry.observe_match();
        candidates.push(ReferenceOccurrenceCandidate {
            id: id.value(row).to_string(),
            score,
            path: path.value(row).to_string(),
            line: usize::try_from(line.value(row)).unwrap_or(usize::MAX),
            column: usize::try_from(column.value(row)).unwrap_or(usize::MAX),
        });
        telemetry.observe_working_set(candidates.len());
        if candidates.len() > window.threshold {
            let before_len = candidates.len();
            trim_ranked_vec(candidates, window.target, compare_candidates);
            telemetry.observe_trim(before_len, candidates.len());
        }
    }

    Ok(())
}

pub(super) fn compare_candidates(
    left: &ReferenceOccurrenceCandidate,
    right: &ReferenceOccurrenceCandidate,
) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.line.cmp(&right.line))
        .then_with(|| left.column.cmp(&right.column))
}

fn string_column<'a>(
    batch: &'a EngineRecordBatch,
    name: &str,
) -> Result<EngineStringColumn<'a>, ReferenceOccurrenceSearchError> {
    let column = batch.column_by_name(name).ok_or_else(|| {
        ReferenceOccurrenceSearchError::Decode(format!("missing engine string column `{name}`"))
    })?;
    if let Some(array) = column.as_any().downcast_ref::<StringArray>() {
        return Ok(EngineStringColumn::Utf8(array));
    }
    if let Some(array) = column.as_any().downcast_ref::<StringViewArray>() {
        return Ok(EngineStringColumn::Utf8View(array));
    }
    Err(ReferenceOccurrenceSearchError::Decode(format!(
        "engine column `{name}` is not utf8-like"
    )))
}

fn u64_column<'a>(
    batch: &'a EngineRecordBatch,
    name: &str,
) -> Result<&'a UInt64Array, ReferenceOccurrenceSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<UInt64Array>())
        .ok_or_else(|| {
            ReferenceOccurrenceSearchError::Decode(format!("missing engine u64 column `{name}`"))
        })
}

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
