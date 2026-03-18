use super::anchor_batch::{QuantumAnchorBatchError, QuantumAnchorBatchView};
use super::scored_context::quantum_contexts_from_scored_batch;
use super::scoring::{
    BatchQuantumScorer, BatchQuantumScorerError, QUANTUM_SALIENCY_COLUMN,
    topology_score_from_ranked,
};
use super::topology_expansion::{
    collect_related_clusters, quantum_related_limit_for_doc, resolve_quantum_related_limits,
};
use crate::link_graph::index::LinkGraphIndex;
use crate::link_graph::models::{QuantumAnchorHit, QuantumContext, QuantumFusionOptions};
use arrow::array::{Array, Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;

const ANCHOR_ID_COLUMN: &str = "anchor_id";
const VECTOR_SCORE_COLUMN: &str = "vector_score";

#[derive(Debug, Clone)]
struct QuantumContextCandidate {
    anchor_id: String,
    semantic_path: Vec<String>,
    related_clusters: Vec<String>,
    vector_score: f64,
    topology_score: f64,
}

/// Error returned when Wendao cannot construct scored quantum contexts.
#[derive(Debug, Error)]
pub enum QuantumContextBuildError {
    /// Input batch is missing the configured identifier column.
    #[error("missing required input batch column `{column}`")]
    MissingInputColumn {
        /// Name of the missing column.
        column: String,
    },
    /// Input batch identifier column is not Arrow `Utf8`.
    #[error("input batch column `{column}` must be Utf8, found `{data_type:?}`")]
    InvalidInputUtf8Column {
        /// Name of the offending input column.
        column: String,
        /// Actual Arrow type found in the input batch.
        data_type: DataType,
    },
    /// Input batch score column is not Arrow `Float64`.
    #[error("input batch column `{column}` must be Float64, found `{data_type:?}`")]
    InvalidInputFloat64Column {
        /// Name of the offending input column.
        column: String,
        /// Actual Arrow type found in the input batch.
        data_type: DataType,
    },
    /// Input batch unexpectedly contained a null value.
    #[error("input batch column `{column}` contains null at row {row}")]
    NullInputValue {
        /// Name of the offending input column.
        column: String,
        /// Zero-based row index of the null value.
        row: usize,
    },
    /// Failed to build the Arrow batch fed into the batch scorer.
    #[error("failed to build Arrow batch for quantum-context scoring")]
    BuildScoringBatch(#[source] ArrowError),
    /// The batch scorer rejected the prepared scoring batch.
    #[error("failed to compute quantum-context saliency from scoring batch")]
    ScoreScoringBatch(#[source] BatchQuantumScorerError),
    /// The scored batch did not contain the fused saliency column.
    #[error("scored quantum-context batch is missing fused saliency column `{column}`")]
    MissingSaliencyColumn {
        /// Name of the required fused saliency column.
        column: String,
    },
    /// The fused saliency column had the wrong Arrow type.
    #[error(
        "scored quantum-context batch column `{column}` must be Float64, found `{data_type:?}`"
    )]
    InvalidSaliencyColumn {
        /// Name of the offending scored column.
        column: String,
        /// Actual Arrow type found in the scored batch.
        data_type: DataType,
    },
    /// The fused saliency column unexpectedly contained a null value.
    #[error("scored quantum-context batch column `{column}` contains null at row {row}")]
    NullSaliencyValue {
        /// Name of the offending scored column.
        column: String,
        /// Zero-based row index of the null value.
        row: usize,
    },
}

impl From<QuantumAnchorBatchError> for QuantumContextBuildError {
    fn from(error: QuantumAnchorBatchError) -> Self {
        match error {
            QuantumAnchorBatchError::MissingColumn { column } => {
                Self::MissingInputColumn { column }
            }
            QuantumAnchorBatchError::InvalidUtf8Column { column, data_type } => {
                Self::InvalidInputUtf8Column { column, data_type }
            }
            QuantumAnchorBatchError::InvalidFloat64Column { column, data_type } => {
                Self::InvalidInputFloat64Column { column, data_type }
            }
            QuantumAnchorBatchError::NullValue { column, row } => {
                Self::NullInputValue { column, row }
            }
        }
    }
}

impl LinkGraphIndex {
    /// Build traceable quantum-fusion contexts from Arrow anchor batches.
    ///
    /// # Errors
    ///
    /// Returns [`QuantumContextBuildError`] when the anchor batch cannot be
    /// decoded or the Arrow-native scorer cannot produce the fused saliency
    /// output column.
    pub fn quantum_contexts_from_anchor_batch(
        &self,
        batch: &RecordBatch,
        id_col: &str,
        score_col: &str,
        options: &QuantumFusionOptions,
    ) -> Result<Vec<QuantumContext>, QuantumContextBuildError> {
        let options = options.normalized();
        let view = QuantumAnchorBatchView::new(batch, id_col, score_col)?;
        let resolved = self.resolve_quantum_anchors(&view);
        if resolved.is_empty() {
            return Ok(Vec::new());
        }
        let candidates = self.expand_quantum_context_candidates(&resolved, &options);
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let topology_scores = candidates
            .iter()
            .map(|candidate| (candidate.batch_anchor_id.clone(), candidate.topology_score))
            .collect::<HashMap<_, _>>();
        let scorer = BatchQuantumScorer::new(&options);
        let scored_batch = scorer
            .score_batch(view.batch(), &topology_scores, id_col, score_col)
            .map_err(QuantumContextBuildError::ScoreScoringBatch)?;

        quantum_contexts_from_scored_batch(candidates, &scored_batch)
    }

    /// Build traceable quantum-fusion contexts from precomputed semantic anchors.
    ///
    /// # Errors
    ///
    /// Returns [`QuantumContextBuildError`] when the prepared scoring batch
    /// cannot be constructed or the Arrow-native scorer cannot produce the
    /// fused saliency output column.
    pub fn quantum_contexts_from_anchors(
        &self,
        anchors: &[QuantumAnchorHit],
        options: &QuantumFusionOptions,
    ) -> Result<Vec<QuantumContext>, QuantumContextBuildError> {
        let options = options.normalized();
        let candidates = self.quantum_context_candidates(anchors, &options);
        let saliency_scores = score_quantum_context_candidates(&candidates, &options)?;

        let mut contexts = candidates
            .into_iter()
            .zip(saliency_scores)
            .map(|(candidate, saliency_score)| {
                let anchor_id = candidate.anchor_id;
                let doc_id = anchor_id
                    .split_once('#')
                    .map_or(anchor_id.as_str(), |(doc_id, _)| doc_id)
                    .to_string();
                let path = self
                    .get_doc(doc_id.as_str())
                    .map_or_else(|| doc_id.clone(), |doc| doc.path.clone());
                let trace_label =
                    QuantumContext::trace_label_from_semantic_path(&candidate.semantic_path);
                QuantumContext {
                    anchor_id,
                    doc_id,
                    path,
                    semantic_path: candidate.semantic_path,
                    trace_label,
                    related_clusters: candidate.related_clusters,
                    saliency_score,
                    vector_score: candidate.vector_score,
                    topology_score: candidate.topology_score,
                }
            })
            .collect::<Vec<_>>();

        contexts.sort_by(|left, right| {
            right
                .saliency_score
                .partial_cmp(&left.saliency_score)
                .unwrap_or(Ordering::Equal)
                .then(left.anchor_id.cmp(&right.anchor_id))
        });
        Ok(contexts)
    }

    fn quantum_context_candidates(
        &self,
        anchors: &[QuantumAnchorHit],
        options: &QuantumFusionOptions,
    ) -> Vec<QuantumContextCandidate> {
        let related_limits = resolve_quantum_related_limits(
            &anchors
                .iter()
                .filter_map(|anchor| self.quantum_anchor_doc_id(anchor.anchor_id.as_str()))
                .collect::<Vec<_>>(),
            options.related_limit,
        );
        let mut candidates = Vec::new();

        for anchor in anchors {
            let anchor_id = anchor.anchor_id.trim();
            if anchor_id.is_empty() {
                continue;
            }
            let Some(seed_doc_id) = self.quantum_anchor_doc_id(anchor_id) else {
                continue;
            };
            // 2026 Refinement: use hierarchical lineage for anchor semantic path
            let semantic_path = self.extract_lineage(anchor_id).unwrap_or_default();

            let effective_related_limit = quantum_related_limit_for_doc(
                seed_doc_id.as_str(),
                &related_limits,
                options.related_limit,
            );
            let ranked = self.quantum_related_ranked_doc_ids(
                seed_doc_id.as_str(),
                anchor.vector_score,
                options,
            );
            let related_clusters = collect_related_clusters(&ranked, effective_related_limit);
            let topology_score = topology_score_from_ranked(&ranked, effective_related_limit);

            candidates.push(QuantumContextCandidate {
                anchor_id: anchor_id.to_string(),
                semantic_path,
                related_clusters,
                vector_score: anchor.vector_score.clamp(0.0, 1.0),
                topology_score,
            });
        }

        candidates
    }

    pub(super) fn quantum_related_ranked_doc_ids(
        &self,
        seed_doc_id: &str,
        _vector_score: f64,
        options: &QuantumFusionOptions,
    ) -> Vec<(String, usize, f64)> {
        let seeds = HashSet::from([seed_doc_id.to_string()]);
        self.related_ppr_ranked_doc_ids(&seeds, options.max_distance, options.ppr.as_ref())
    }
}

fn score_quantum_context_candidates(
    candidates: &[QuantumContextCandidate],
    options: &QuantumFusionOptions,
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

fn build_quantum_context_batch(
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

fn extract_saliency_scores(batch: &RecordBatch) -> Result<Vec<f64>, QuantumContextBuildError> {
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
