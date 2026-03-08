use super::anchor_batch::{QuantumAnchorBatchError, QuantumAnchorBatchView};
use super::scored_context::{QuantumContextCandidate, quantum_contexts_from_scored_batch};
use super::scoring::BatchQuantumScorer;
use crate::link_graph::index::LinkGraphIndex;
use crate::link_graph::models::{QuantumAnchorHit, QuantumContext, QuantumFusionOptions};
use arrow::array::{Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

const ANCHOR_ID_COLUMN: &str = "anchor_id";
const VECTOR_SCORE_COLUMN: &str = "vector_score";

/// Error returned when Wendao cannot construct scored quantum contexts.
#[derive(Debug, Error)]
pub enum QuantumContextBuildError {
    /// Failed to build the Arrow batch fed into the batch scorer.
    #[error("failed to build Arrow batch for quantum-context scoring")]
    BuildScoringBatch(#[source] ArrowError),
    /// Required anchor-input column is missing from the semantic batch.
    #[error("missing required quantum-anchor batch column `{column}`")]
    MissingInputColumn {
        /// Name of the missing anchor-input column.
        column: String,
    },
    /// Anchor-id input column is not Arrow `Utf8`.
    #[error("quantum-anchor batch column `{column}` must be Utf8, found `{data_type:?}`")]
    InvalidInputUtf8Column {
        /// Name of the offending anchor-input column.
        column: String,
        /// Actual Arrow type found in the input batch.
        data_type: DataType,
    },
    /// Vector-score input column is not Arrow `Float64`.
    #[error("quantum-anchor batch column `{column}` must be Float64, found `{data_type:?}`")]
    InvalidInputFloat64Column {
        /// Name of the offending score-input column.
        column: String,
        /// Actual Arrow type found in the input batch.
        data_type: DataType,
    },
    /// Required anchor-input cell is null.
    #[error("quantum-anchor batch column `{column}` contains null at row {row}")]
    NullInputValue {
        /// Name of the offending anchor-input column.
        column: String,
        /// Zero-based row index of the null input value.
        row: usize,
    },
    /// The batch scorer rejected the prepared scoring batch.
    #[error("failed to compute quantum-context saliency from scoring batch")]
    ScoreScoringBatch(#[source] super::scoring::BatchQuantumScorerError),
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
    fn from(value: QuantumAnchorBatchError) -> Self {
        match value {
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
    /// Build traceable quantum-fusion contexts from precomputed semantic anchors.
    ///
    /// # Errors
    ///
    /// Returns [`QuantumContextBuildError`] when the anchor slice cannot be
    /// converted into the canonical scoring batch or the Arrow-native scorer
    /// cannot produce the fused saliency output column.
    pub fn quantum_contexts_from_anchors(
        &self,
        anchors: &[QuantumAnchorHit],
        options: &QuantumFusionOptions,
    ) -> Result<Vec<QuantumContext>, QuantumContextBuildError> {
        let batch = build_anchor_batch_from_hits(anchors)?;
        self.quantum_contexts_from_anchor_batch(
            &batch,
            ANCHOR_ID_COLUMN,
            VECTOR_SCORE_COLUMN,
            options,
        )
    }

    /// Build traceable quantum-fusion contexts from a prepared Arrow batch of
    /// semantic anchors.
    ///
    /// The input batch must contain one `Utf8` anchor-id column and one
    /// `Float64` semantic-score column, identified by `id_col` and `score_col`.
    /// Rows with blank or unresolved anchor ids are ignored after validation.
    ///
    /// # Errors
    ///
    /// Returns [`QuantumContextBuildError`] when required input columns are
    /// missing, input types do not match the expected Arrow layout, a required
    /// input cell is null, or the Arrow-native scorer cannot produce the fused
    /// saliency output column.
    pub fn quantum_contexts_from_anchor_batch(
        &self,
        batch: &RecordBatch,
        id_col: &str,
        score_col: &str,
        options: &QuantumFusionOptions,
    ) -> Result<Vec<QuantumContext>, QuantumContextBuildError> {
        let options = options.normalized();
        let batch_view = QuantumAnchorBatchView::new(batch, id_col, score_col)?;
        let resolved_anchors = self.resolve_quantum_anchors(&batch_view);
        let candidates = self.expand_quantum_context_candidates(&resolved_anchors, &options);
        let scored_batch = score_quantum_context_batch(&batch_view, &candidates, &options)?;
        quantum_contexts_from_scored_batch(candidates, &scored_batch)
    }
}

fn score_quantum_context_batch(
    batch: &QuantumAnchorBatchView<'_>,
    candidates: &[QuantumContextCandidate],
    options: &QuantumFusionOptions,
) -> Result<RecordBatch, QuantumContextBuildError> {
    if candidates.is_empty() {
        return Ok(batch.batch().clone());
    }

    let topology_scores = candidates
        .iter()
        .map(|candidate| (candidate.batch_anchor_id.clone(), candidate.topology_score))
        .collect::<HashMap<_, _>>();
    let scorer = BatchQuantumScorer::new(options);
    scorer
        .score_anchor_batch_view(batch, &topology_scores)
        .map_err(QuantumContextBuildError::ScoreScoringBatch)
}

fn build_anchor_batch_from_hits(
    anchors: &[QuantumAnchorHit],
) -> Result<RecordBatch, QuantumContextBuildError> {
    let schema = Arc::new(Schema::new(vec![
        Field::new(ANCHOR_ID_COLUMN, DataType::Utf8, false),
        Field::new(VECTOR_SCORE_COLUMN, DataType::Float64, false),
    ]));
    let anchor_ids =
        StringArray::from_iter_values(anchors.iter().map(|anchor| anchor.anchor_id.as_str()));
    let vector_scores = Float64Array::from(
        anchors
            .iter()
            .map(|anchor| anchor.vector_score)
            .collect::<Vec<_>>(),
    );

    RecordBatch::try_new(schema, vec![Arc::new(anchor_ids), Arc::new(vector_scores)])
        .map_err(QuantumContextBuildError::BuildScoringBatch)
}
