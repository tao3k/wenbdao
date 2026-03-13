use super::orchestrate::QuantumContextBuildError;
use super::scoring::QUANTUM_SALIENCY_COLUMN;
use crate::link_graph::models::QuantumContext;
use arrow::array::{Array, Float64Array};
use arrow::record_batch::RecordBatch;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub(super) struct QuantumContextCandidate {
    pub(super) batch_row: usize,
    pub(super) batch_anchor_id: String,
    pub(super) anchor_id: String,
    pub(super) doc_id: String,
    pub(super) path: String,
    pub(super) semantic_path: Vec<String>,
    pub(super) trace_label: Option<String>,
    pub(super) related_clusters: Vec<String>,
    pub(super) vector_score: f64,
    pub(super) topology_score: f64,
}

pub(super) fn quantum_contexts_from_scored_batch(
    candidates: Vec<QuantumContextCandidate>,
    scored_batch: &RecordBatch,
) -> Result<Vec<QuantumContext>, QuantumContextBuildError> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let saliency_scores = saliency_scores_column(scored_batch)?;
    let mut contexts = candidates
        .into_iter()
        .map(|candidate| {
            let saliency_score = saliency_score_at_row(saliency_scores, candidate.batch_row)?;
            Ok(QuantumContext {
                anchor_id: candidate.anchor_id,
                doc_id: candidate.doc_id,
                path: candidate.path,
                semantic_path: candidate.semantic_path,
                trace_label: candidate.trace_label,
                related_clusters: candidate.related_clusters,
                saliency_score,
                vector_score: candidate.vector_score,
                topology_score: candidate.topology_score,
            })
        })
        .collect::<Result<Vec<_>, QuantumContextBuildError>>()?;

    contexts.sort_by(|left, right| {
        right
            .saliency_score
            .partial_cmp(&left.saliency_score)
            .unwrap_or(Ordering::Equal)
            .then(left.anchor_id.cmp(&right.anchor_id))
    });
    Ok(contexts)
}

fn saliency_scores_column(batch: &RecordBatch) -> Result<&Float64Array, QuantumContextBuildError> {
    let saliency_column = batch
        .column_by_name(QUANTUM_SALIENCY_COLUMN)
        .ok_or_else(|| QuantumContextBuildError::MissingSaliencyColumn {
            column: QUANTUM_SALIENCY_COLUMN.to_string(),
        })?;
    saliency_column
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or_else(|| QuantumContextBuildError::InvalidSaliencyColumn {
            column: QUANTUM_SALIENCY_COLUMN.to_string(),
            data_type: saliency_column.data_type().clone(),
        })
}

fn saliency_score_at_row(
    saliency_scores: &Float64Array,
    row: usize,
) -> Result<f64, QuantumContextBuildError> {
    if saliency_scores.is_null(row) {
        return Err(QuantumContextBuildError::NullSaliencyValue {
            column: QUANTUM_SALIENCY_COLUMN.to_string(),
            row,
        });
    }
    Ok(saliency_scores.value(row))
}
