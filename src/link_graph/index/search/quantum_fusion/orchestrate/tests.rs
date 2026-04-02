use crate::link_graph::index::search::quantum_fusion::orchestrate::candidates::QuantumContextCandidate;
use crate::link_graph::index::search::quantum_fusion::orchestrate::scoring::{
    build_quantum_context_batch, extract_saliency_scores,
};
use crate::link_graph::index::search::quantum_fusion::scoring::QUANTUM_SALIENCY_COLUMN;
use arrow::array::{ArrayRef, Float64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

#[test]
fn build_quantum_context_batch_uses_expected_schema() {
    let candidates = vec![QuantumContextCandidate {
        anchor_id: "doc#1".to_string(),
        semantic_path: vec!["root".to_string()],
        related_clusters: vec!["doc".to_string()],
        vector_score: 0.5,
        topology_score: 0.4,
    }];

    let batch = build_quantum_context_batch(&candidates).expect("batch should build");
    assert_eq!(batch.num_rows(), 1);
    assert_eq!(batch.schema().field(0).name(), "anchor_id");
    assert_eq!(batch.schema().field(0).data_type(), &DataType::Utf8);
    assert_eq!(batch.schema().field(1).name(), "vector_score");
    assert_eq!(batch.schema().field(1).data_type(), &DataType::Float64);
}

#[test]
fn extract_saliency_scores_reads_expected_values() {
    let schema = Arc::new(Schema::new(vec![Field::new(
        QUANTUM_SALIENCY_COLUMN,
        DataType::Float64,
        false,
    )]));
    let values: ArrayRef = Arc::new(Float64Array::from(vec![0.1, 0.7]));
    let batch = RecordBatch::try_new(schema, vec![values]).expect("batch should build");

    assert_eq!(
        extract_saliency_scores(&batch).expect("scores should extract"),
        vec![0.1, 0.7]
    );
}
