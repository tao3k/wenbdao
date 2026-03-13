use arrow::array::{Array, Float64Array, StringArray};
use arrow::datatypes::DataType;
use arrow::record_batch::RecordBatch;
use thiserror::Error;

#[derive(Debug, Clone)]
pub(super) struct QuantumAnchorBatchView<'a> {
    batch: &'a RecordBatch,
    anchor_ids: &'a StringArray,
    vector_scores: &'a Float64Array,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct QuantumAnchorBatchRow<'a> {
    pub(super) row: usize,
    pub(super) anchor_id: &'a str,
    pub(super) vector_score: f64,
}

#[derive(Debug, Clone, Error)]
pub(super) enum QuantumAnchorBatchError {
    #[error("missing required batch column `{column}`")]
    MissingColumn { column: String },
    #[error("batch column `{column}` must be Utf8, found `{data_type:?}`")]
    InvalidUtf8Column { column: String, data_type: DataType },
    #[error("batch column `{column}` must be Float64, found `{data_type:?}`")]
    InvalidFloat64Column { column: String, data_type: DataType },
    #[error("batch column `{column}` contains null at row {row}")]
    NullValue { column: String, row: usize },
}

impl<'a> QuantumAnchorBatchView<'a> {
    pub(super) fn new(
        batch: &'a RecordBatch,
        id_col: &str,
        score_col: &str,
    ) -> Result<Self, QuantumAnchorBatchError> {
        let anchor_ids_column =
            batch
                .column_by_name(id_col)
                .ok_or_else(|| QuantumAnchorBatchError::MissingColumn {
                    column: id_col.to_string(),
                })?;
        let anchor_ids = anchor_ids_column
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| QuantumAnchorBatchError::InvalidUtf8Column {
                column: id_col.to_string(),
                data_type: anchor_ids_column.data_type().clone(),
            })?;

        let vector_scores_column = batch.column_by_name(score_col).ok_or_else(|| {
            QuantumAnchorBatchError::MissingColumn {
                column: score_col.to_string(),
            }
        })?;
        let vector_scores = vector_scores_column
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| QuantumAnchorBatchError::InvalidFloat64Column {
                column: score_col.to_string(),
                data_type: vector_scores_column.data_type().clone(),
            })?;

        validate_required_values(anchor_ids, id_col)?;
        validate_required_values(vector_scores, score_col)?;

        Ok(Self {
            batch,
            anchor_ids,
            vector_scores,
        })
    }

    pub(super) const fn batch(&self) -> &'a RecordBatch {
        self.batch
    }

    pub(super) fn rows(&self) -> impl Iterator<Item = QuantumAnchorBatchRow<'_>> + '_ {
        (0..self.batch.num_rows()).map(|row| QuantumAnchorBatchRow {
            row,
            anchor_id: self.anchor_ids.value(row),
            vector_score: self.vector_scores.value(row),
        })
    }
}

fn validate_required_values(
    array: &dyn Array,
    column: &str,
) -> Result<(), QuantumAnchorBatchError> {
    if array.null_count() == 0 {
        return Ok(());
    }

    for row in 0..array.len() {
        if array.is_null(row) {
            return Err(QuantumAnchorBatchError::NullValue {
                column: column.to_string(),
                row,
            });
        }
    }

    Ok(())
}
