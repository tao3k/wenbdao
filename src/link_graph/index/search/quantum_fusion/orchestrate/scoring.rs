use crate::QuantumContextBuildError;
use crate::link_graph::index::search::quantum_fusion::orchestrate::candidates::QuantumContextCandidate;
use crate::link_graph::index::search::quantum_fusion::scoring::{
    BatchQuantumScorer, QUANTUM_SALIENCY_COLUMN,
};
use arrow::array::{Array, Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) fn score_quantum_context_candidates(
    candidates: &[QuantumContextCandidate],
    options: &crate::link_graph::models::QuantumFusionOptions,
) -> Result<Vec<f64>, QuantumContextBuildError> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let batch = build_quantum_context_batch(candidates)?;
    let topology_scores = candidates
        .iter()
        .map(|candidate| (candidate.anchor_id.clone(), candidate.topology_score))
        .collect::<HashMap<_, _>>();
    let scorer = BatchQuantumScorer::new(options);
    let scored_batch = scorer
        .score_batch(
            &batch,
            &topology_scores,
            ANCHOR_ID_COLUMN,
            VECTOR_SCORE_COLUMN,
        )
        .map_err(QuantumContextBuildError::ScoreScoringBatch)?;

    extract_saliency_scores(&scored_batch)
}

const ANCHOR_ID_COLUMN: &str = "anchor_id";
const VECTOR_SCORE_COLUMN: &str = "vector_score";

pub(crate) fn build_quantum_context_batch(
    candidates: &[QuantumContextCandidate],
) -> Result<RecordBatch, QuantumContextBuildError> {
    let schema = Arc::new(Schema::new(vec![
        Field::new(ANCHOR_ID_COLUMN, DataType::Utf8, false),
        Field::new(VECTOR_SCORE_COLUMN, DataType::Float64, false),
    ]));
    let anchor_ids = StringArray::from_iter_values(
        candidates
            .iter()
            .map(|candidate| candidate.anchor_id.as_str()),
    );
    let vector_scores = Float64Array::from(
        candidates
            .iter()
            .map(|candidate| candidate.vector_score)
            .collect::<Vec<_>>(),
    );

    RecordBatch::try_new(schema, vec![Arc::new(anchor_ids), Arc::new(vector_scores)])
        .map_err(QuantumContextBuildError::BuildScoringBatch)
}

pub(crate) fn extract_saliency_scores(
    batch: &RecordBatch,
) -> Result<Vec<f64>, QuantumContextBuildError> {
    let saliency_column = batch
        .column_by_name(QUANTUM_SALIENCY_COLUMN)
        .ok_or_else(|| QuantumContextBuildError::MissingSaliencyColumn {
            column: QUANTUM_SALIENCY_COLUMN.to_string(),
        })?;
    let saliency_scores = saliency_column
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or_else(|| QuantumContextBuildError::InvalidSaliencyColumn {
            column: QUANTUM_SALIENCY_COLUMN.to_string(),
            data_type: saliency_column.data_type().clone(),
        })?;

    let mut scores = Vec::with_capacity(saliency_scores.len());
    for index in 0..saliency_scores.len() {
        if saliency_scores.is_null(index) {
            return Err(QuantumContextBuildError::NullSaliencyValue {
                column: QUANTUM_SALIENCY_COLUMN.to_string(),
                row: index,
            });
        }
        scores.push(saliency_scores.value(index));
    }

    Ok(scores)
}
