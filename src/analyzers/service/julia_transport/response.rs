#[cfg(feature = "julia")]
use std::collections::BTreeMap;

#[cfg(feature = "julia")]
use arrow::array::{Array, Float64Array, StringArray};
#[cfg(feature = "julia")]
use arrow::record_batch::RecordBatch;

#[cfg(feature = "julia")]
use crate::analyzers::errors::RepoIntelligenceError;

#[cfg(feature = "julia")]
use super::errors::contract_decode_error;
#[cfg(feature = "julia")]
use super::schema::{
    JULIA_ARROW_ANALYZER_SCORE_COLUMN, JULIA_ARROW_DOC_ID_COLUMN, JULIA_ARROW_FINAL_SCORE_COLUMN,
    JULIA_ARROW_TRACE_ID_COLUMN,
};

/// One typed row materialized from the WendaoArrow plugin response contract.
#[cfg(feature = "julia")]
#[derive(Debug, Clone, PartialEq)]
pub struct PluginArrowScoreRow {
    /// Stable document identifier emitted by the Rust request batch.
    pub doc_id: String,
    /// Julia-side analyzer score for the document.
    pub analyzer_score: f64,
    /// Final score after Julia-side reranking.
    pub final_score: f64,
    /// Optional trace identifier materialized from additive Julia response columns.
    pub trace_id: Option<String>,
}

/// Decode plugin Arrow response batches into a `doc_id` keyed score map.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the response batch shape does not
/// match the WendaoArrow `v1` response contract.
#[cfg(feature = "julia")]
pub fn decode_plugin_arrow_score_rows(
    batches: &[RecordBatch],
) -> Result<BTreeMap<String, PluginArrowScoreRow>, RepoIntelligenceError> {
    let mut rows = BTreeMap::new();

    for batch in batches {
        let doc_id = batch
            .column_by_name(JULIA_ARROW_DOC_ID_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| contract_decode_error("missing required Utf8 column `doc_id`"))?;
        let analyzer_score = batch
            .column_by_name(JULIA_ARROW_ANALYZER_SCORE_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .ok_or_else(|| {
                contract_decode_error("missing required Float64 column `analyzer_score`")
            })?;
        let final_score = batch
            .column_by_name(JULIA_ARROW_FINAL_SCORE_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .ok_or_else(|| {
                contract_decode_error("missing required Float64 column `final_score`")
            })?;
        let trace_id = batch
            .column_by_name(JULIA_ARROW_TRACE_ID_COLUMN)
            .map(|array| {
                array
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| contract_decode_error("optional `trace_id` column must be Utf8"))
            })
            .transpose()?;

        for row in 0..batch.num_rows() {
            let doc_id_value = doc_id
                .is_valid(row)
                .then(|| doc_id.value(row).to_string())
                .ok_or_else(|| contract_decode_error("`doc_id` must be non-null"))?;
            let analyzer_score_value = analyzer_score
                .is_valid(row)
                .then(|| analyzer_score.value(row))
                .ok_or_else(|| contract_decode_error("`analyzer_score` must be non-null"))?;
            let final_score_value = final_score
                .is_valid(row)
                .then(|| final_score.value(row))
                .ok_or_else(|| contract_decode_error("`final_score` must be non-null"))?;

            rows.insert(
                doc_id_value.clone(),
                PluginArrowScoreRow {
                    doc_id: doc_id_value,
                    analyzer_score: analyzer_score_value,
                    final_score: final_score_value,
                    trace_id: trace_id.and_then(|array| {
                        array.is_valid(row).then(|| array.value(row).to_string())
                    }),
                },
            );
        }
    }

    Ok(rows)
}

/// Compatibility alias for the legacy Julia-named response row.
#[cfg(feature = "julia")]
pub type JuliaArrowScoreRow = PluginArrowScoreRow;

/// Compatibility shim for the legacy Julia-named response decoder.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the response batch shape does not
/// match the WendaoArrow `v1` response contract.
#[cfg(feature = "julia")]
pub fn decode_julia_arrow_score_rows(
    batches: &[RecordBatch],
) -> Result<BTreeMap<String, JuliaArrowScoreRow>, RepoIntelligenceError> {
    decode_plugin_arrow_score_rows(batches)
}
