use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{Array, Float64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use thiserror::Error;

use super::anchor_batch::{QuantumAnchorBatchError, QuantumAnchorBatchView};
use crate::link_graph::models::QuantumFusionOptions;

/// Output column appended by [`BatchQuantumScorer`].
pub const QUANTUM_SALIENCY_COLUMN: &str = "quantum_saliency";

/// Arrow-native scorer that fuses semantic and topology scores in one batch pass.
#[derive(Debug, Clone)]
pub struct BatchQuantumScorer {
    options: QuantumFusionOptions,
}

impl BatchQuantumScorer {
    /// Create a new batch scorer with normalized fusion options.
    #[must_use]
    pub fn new(options: &QuantumFusionOptions) -> Self {
        Self {
            options: options.normalized(),
        }
    }

    /// Fuse semantic and topology scores for every row in an Arrow `RecordBatch`.
    ///
    /// The `ppr_map` values are expected to be pre-normalized topology saliency
    /// scores keyed by the same identifiers stored in `id_col`.
    ///
    /// # Errors
    ///
    /// Returns [`BatchQuantumScorerError`] when required columns are missing,
    /// column types do not match the expected Arrow layout, a required value is
    /// null, or the fused output batch cannot be constructed.
    pub fn score_batch(
        &self,
        batch: &RecordBatch,
        ppr_map: &HashMap<String, f64>,
        id_col: &str,
        sim_col: &str,
    ) -> Result<RecordBatch, BatchQuantumScorerError> {
        let batch_view = QuantumAnchorBatchView::new(batch, id_col, sim_col)?;
        self.score_anchor_batch_view(&batch_view, ppr_map)
    }

    pub(in crate::link_graph::index::search::quantum_fusion) fn score_anchor_batch_view(
        &self,
        batch_view: &QuantumAnchorBatchView<'_>,
        ppr_map: &HashMap<String, f64>,
    ) -> Result<RecordBatch, BatchQuantumScorerError> {
        let mut fused_scores = Vec::with_capacity(batch_view.batch().num_rows());
        for row in batch_view.rows() {
            let topology_score = ppr_map.get(row.anchor_id).copied().unwrap_or(0.0);
            fused_scores.push(fuse_saliency_score(
                row.vector_score,
                topology_score,
                &self.options,
            ));
        }

        let fused_array: Arc<dyn Array> = Arc::new(Float64Array::from(fused_scores));
        let mut fields = batch_view
            .batch()
            .schema_ref()
            .fields()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        fields.push(Arc::new(Field::new(
            QUANTUM_SALIENCY_COLUMN,
            DataType::Float64,
            false,
        )));
        let schema = Arc::new(Schema::new_with_metadata(
            fields,
            batch_view.batch().schema_ref().metadata().clone(),
        ));

        let mut columns = batch_view.batch().columns().to_vec();
        columns.push(fused_array);
        RecordBatch::try_new(schema, columns).map_err(BatchQuantumScorerError::Arrow)
    }
}

/// Error returned when Arrow-native batch scoring cannot be completed.
#[derive(Debug, Error)]
pub enum BatchQuantumScorerError {
    /// Required input column is missing from the batch schema.
    #[error("missing required batch column `{column}`")]
    MissingColumn {
        /// Name of the missing column.
        column: String,
    },
    /// Input id column is not Arrow `Utf8`.
    #[error("batch column `{column}` must be Utf8, found `{data_type:?}`")]
    InvalidUtf8Column {
        /// Name of the offending column.
        column: String,
        /// Actual Arrow data type found in the batch.
        data_type: DataType,
    },
    /// Input similarity column is not Arrow `Float64`.
    #[error("batch column `{column}` must be Float64, found `{data_type:?}`")]
    InvalidFloat64Column {
        /// Name of the offending column.
        column: String,
        /// Actual Arrow data type found in the batch.
        data_type: DataType,
    },
    /// Required cell is null.
    #[error("batch column `{column}` contains null at row {row}")]
    NullValue {
        /// Name of the offending column.
        column: String,
        /// Zero-based row index carrying the null value.
        row: usize,
    },
    /// Arrow failed to construct the fused batch.
    #[error("failed to construct fused RecordBatch: {0}")]
    Arrow(ArrowError),
}

impl From<QuantumAnchorBatchError> for BatchQuantumScorerError {
    fn from(value: QuantumAnchorBatchError) -> Self {
        match value {
            QuantumAnchorBatchError::MissingColumn { column } => Self::MissingColumn { column },
            QuantumAnchorBatchError::InvalidUtf8Column { column, data_type } => {
                Self::InvalidUtf8Column { column, data_type }
            }
            QuantumAnchorBatchError::InvalidFloat64Column { column, data_type } => {
                Self::InvalidFloat64Column { column, data_type }
            }
            QuantumAnchorBatchError::NullValue { column, row } => Self::NullValue { column, row },
        }
    }
}

pub(in crate::link_graph::index::search::quantum_fusion) fn fuse_saliency_score(
    vector_score: f64,
    topology_score: f64,
    options: &QuantumFusionOptions,
) -> f64 {
    let alpha = options.alpha.clamp(0.0, 1.0);
    let semantic = vector_score.clamp(0.0, 1.0);
    let topology = topology_score.clamp(0.0, 1.0);
    alpha * semantic + (1.0 - alpha) * topology
}

pub(in crate::link_graph::index::search::quantum_fusion) fn topology_score_from_ranked(
    ranked: &[(String, usize, f64)],
    related_limit: usize,
) -> f64 {
    let mut total = 0.0_f64;
    for (_, _, score) in ranked.iter().take(related_limit.max(1)) {
        total += score.max(0.0);
    }
    total.clamp(0.0, 1.0)
}
