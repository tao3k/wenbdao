#[cfg(feature = "julia")]
use std::sync::Arc;

#[cfg(feature = "julia")]
use arrow::array::{FixedSizeListArray, Float32Array, Float64Array, StringArray};
#[cfg(feature = "julia")]
use arrow::datatypes::{DataType, Field};
#[cfg(feature = "julia")]
use arrow::record_batch::RecordBatch;

#[cfg(feature = "julia")]
use crate::analyzers::errors::RepoIntelligenceError;

#[cfg(feature = "julia")]
use super::errors::contract_request_error;
#[cfg(feature = "julia")]
use super::schema::julia_arrow_request_schema;

#[cfg(feature = "julia")]
fn julia_arrow_vector_item_field() -> Arc<Field> {
    Arc::new(Field::new("item", DataType::Float32, true))
}

/// One request row for the WendaoArrow `v1` plugin rerank contract.
#[cfg(feature = "julia")]
#[derive(Debug, Clone, PartialEq)]
pub struct PluginArrowRequestRow {
    /// Stable document identifier for the candidate row.
    pub doc_id: String,
    /// Coarse Rust-side retrieval score.
    pub vector_score: f64,
    /// Candidate embedding forwarded to Julia.
    pub embedding: Vec<f32>,
}

/// Build one WendaoArrow `v1` plugin request batch from typed Rust rows.
///
/// The request batch contains `doc_id`, `vector_score`, `embedding`, and
/// `query_embedding`, with the query vector repeated per row.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the request rows are empty, any row
/// carries an empty embedding, or the embedding dimensions do not match the
/// provided query vector dimension.
#[cfg(feature = "julia")]
pub fn build_plugin_arrow_request_batch(
    rows: &[PluginArrowRequestRow],
    query_vector: &[f32],
) -> Result<RecordBatch, RepoIntelligenceError> {
    if rows.is_empty() {
        return Err(contract_request_error(
            "WendaoArrow request batch requires at least one row",
        ));
    }
    if query_vector.is_empty() {
        return Err(contract_request_error(
            "WendaoArrow request batch requires a non-empty query vector",
        ));
    }

    let expected_dim = query_vector.len();
    let Some(vector_dim) = i32::try_from(expected_dim).ok() else {
        return Err(contract_request_error(format!(
            "query vector dimension {expected_dim} exceeds i32 range"
        )));
    };

    let mut doc_ids = Vec::with_capacity(rows.len());
    let mut vector_scores = Vec::with_capacity(rows.len());
    let mut embedding_values = Vec::with_capacity(rows.len() * expected_dim);
    let mut query_embedding_values = Vec::with_capacity(rows.len() * expected_dim);

    for row in rows {
        if row.doc_id.trim().is_empty() {
            return Err(contract_request_error(
                "WendaoArrow request row `doc_id` must be non-empty",
            ));
        }
        if row.embedding.len() != expected_dim {
            return Err(contract_request_error(format!(
                "embedding dimension mismatch for doc_id `{}`: expected {}, found {}",
                row.doc_id,
                expected_dim,
                row.embedding.len()
            )));
        }

        doc_ids.push(row.doc_id.as_str());
        vector_scores.push(row.vector_score);
        embedding_values.extend_from_slice(row.embedding.as_slice());
        query_embedding_values.extend_from_slice(query_vector);
    }

    let schema = julia_arrow_request_schema(vector_dim);

    let embedding = FixedSizeListArray::try_new(
        julia_arrow_vector_item_field(),
        vector_dim,
        Arc::new(Float32Array::from(embedding_values)),
        None,
    )
    .map_err(|error| contract_request_error(error.to_string()))?;
    let query_embedding = FixedSizeListArray::try_new(
        julia_arrow_vector_item_field(),
        vector_dim,
        Arc::new(Float32Array::from(query_embedding_values)),
        None,
    )
    .map_err(|error| contract_request_error(error.to_string()))?;

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(doc_ids)),
            Arc::new(Float64Array::from(vector_scores)),
            Arc::new(embedding),
            Arc::new(query_embedding),
        ],
    )
    .map_err(|error| contract_request_error(error.to_string()))
}

/// Compatibility alias for the legacy Julia-named request row.
#[cfg(feature = "julia")]
pub type JuliaArrowRequestRow = PluginArrowRequestRow;

/// Compatibility shim for the legacy Julia-named request-batch builder.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the request rows are empty, any row
/// carries an empty embedding, or the embedding dimensions do not match the
/// provided query vector dimension.
#[cfg(feature = "julia")]
pub fn build_julia_arrow_request_batch(
    rows: &[JuliaArrowRequestRow],
    query_vector: &[f32],
) -> Result<RecordBatch, RepoIntelligenceError> {
    build_plugin_arrow_request_batch(rows, query_vector)
}
