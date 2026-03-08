use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use xiuxian_wendao::QuantumFusionOptions;
use xiuxian_wendao::link_graph::{
    BatchQuantumScorer, BatchQuantumScorerError, QUANTUM_SALIENCY_COLUMN,
};

#[test]
fn test_batch_quantum_scorer_appends_fused_saliency_column()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new_with_metadata(
        vec![
            Field::new("doc_id", DataType::Utf8, false),
            Field::new("semantic_score", DataType::Float64, false),
        ],
        [("source".to_string(), "unit-test".to_string())]
            .into_iter()
            .collect(),
    ));
    let batch = RecordBatch::try_new(
        Arc::clone(&schema),
        vec![
            Arc::new(StringArray::from(vec!["docs/alpha", "docs/beta"])),
            Arc::new(Float64Array::from(vec![0.8, 0.25])),
        ],
    )?;

    let ppr_map = HashMap::from([
        ("docs/alpha".to_string(), 0.1),
        ("docs/beta".to_string(), 0.6),
    ]);
    let batch_scorer = BatchQuantumScorer::new(&QuantumFusionOptions {
        alpha: 0.6,
        max_distance: 2,
        related_limit: 3,
        ppr: None,
    });

    let scored_batch = batch_scorer.score_batch(&batch, &ppr_map, "doc_id", "semantic_score")?;
    let saliency = scored_batch
        .column_by_name(QUANTUM_SALIENCY_COLUMN)
        .ok_or("missing saliency column")?
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or("saliency column has wrong type")?;

    assert_eq!(scored_batch.num_columns(), 3);
    assert_eq!(
        scored_batch.schema_ref().metadata().get("source"),
        Some(&"unit-test".to_string())
    );
    assert!((saliency.value(0) - 0.52).abs() < 1e-12);
    assert!((saliency.value(1) - 0.39).abs() < 1e-12);

    Ok(())
}

#[test]
fn test_batch_quantum_scorer_rejects_wrong_similarity_column_type()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("doc_id", DataType::Utf8, false),
        Field::new("semantic_score", DataType::Utf8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec!["docs/alpha"])),
            Arc::new(StringArray::from(vec!["0.8"])),
        ],
    )?;

    let batch_scorer = BatchQuantumScorer::new(&QuantumFusionOptions::default());
    let error = match batch_scorer.score_batch(&batch, &HashMap::new(), "doc_id", "semantic_score")
    {
        Err(error) => error,
        Ok(scored_batch) => panic!(
            "wrong similarity type should fail, but returned {} columns",
            scored_batch.num_columns()
        ),
    };

    assert!(matches!(
        error,
        BatchQuantumScorerError::InvalidFloat64Column { .. }
    ));

    Ok(())
}

#[test]
fn test_batch_quantum_scorer_rejects_null_identifier_values()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("doc_id", DataType::Utf8, true),
        Field::new("semantic_score", DataType::Float64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec![Some("docs/alpha"), None])),
            Arc::new(Float64Array::from(vec![0.8, 0.2])),
        ],
    )?;

    let batch_scorer = BatchQuantumScorer::new(&QuantumFusionOptions::default());
    let error = match batch_scorer.score_batch(&batch, &HashMap::new(), "doc_id", "semantic_score")
    {
        Err(error) => error,
        Ok(scored_batch) => panic!(
            "null id rows should fail, but returned {} columns",
            scored_batch.num_columns()
        ),
    };

    assert!(matches!(
        error,
        BatchQuantumScorerError::NullValue { row: 1, .. }
    ));

    Ok(())
}
