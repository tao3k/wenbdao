use std::sync::Arc;

use super::*;
use arrow::array::{Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use xiuxian_wendao::{QuantumContextBuildError, QuantumFusionOptions};

#[test]
fn test_quantum_contexts_from_anchor_batch_supports_custom_columns_and_skips_unresolved_rows()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = build_hybrid_fixture()?;
    let anchor_id = fixture.alpha_leaf_anchor_id()?;

    let schema = Arc::new(Schema::new(vec![
        Field::new("anchor_ref", DataType::Utf8, false),
        Field::new("semantic_score", DataType::Float64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec![
                format!("  {anchor_id}  "),
                "missing#anchor".to_string(),
                "   ".to_string(),
            ])),
            Arc::new(Float64Array::from(vec![0.8, 0.9, 0.7])),
        ],
    )?;

    let contexts = fixture.index().quantum_contexts_from_anchor_batch(
        &batch,
        "anchor_ref",
        "semantic_score",
        &default_quantum_fusion_options(),
    )?;

    let actual = contexts_snapshot(&contexts);
    assert_quantum_fixture("quantum_anchor_batch/custom_columns.json", &actual);
    Ok(())
}

#[test]
fn test_quantum_contexts_from_anchor_batch_keeps_duplicate_rows_distinct()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = build_hybrid_fixture()?;
    let anchor_id = fixture.alpha_leaf_anchor_id()?;

    let schema = Arc::new(Schema::new(vec![
        Field::new("anchor_ref", DataType::Utf8, false),
        Field::new("semantic_score", DataType::Float64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec![
                anchor_id.clone(),
                anchor_id.clone(),
            ])),
            Arc::new(Float64Array::from(vec![0.9, 0.2])),
        ],
    )?;

    let contexts = fixture.index().quantum_contexts_from_anchor_batch(
        &batch,
        "anchor_ref",
        "semantic_score",
        &QuantumFusionOptions {
            alpha: 0.7,
            max_distance: 2,
            related_limit: 2,
            ppr: None,
        },
    )?;

    let actual = contexts_snapshot(&contexts);
    assert_quantum_fixture("quantum_anchor_batch/duplicate_rows.json", &actual);
    Ok(())
}

#[test]
fn test_quantum_contexts_from_anchor_batch_supports_whitespace_padded_doc_fallbacks()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = build_hybrid_fixture()?;
    let schema = Arc::new(Schema::new(vec![
        Field::new("anchor_ref", DataType::Utf8, false),
        Field::new("semantic_score", DataType::Float64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec![" plain "])),
            Arc::new(Float64Array::from(vec![0.4])),
        ],
    )?;

    let contexts = fixture.index().quantum_contexts_from_anchor_batch(
        &batch,
        "anchor_ref",
        "semantic_score",
        &QuantumFusionOptions {
            alpha: 0.5,
            max_distance: 1,
            related_limit: 2,
            ppr: None,
        },
    )?;

    let actual = contexts_snapshot(&contexts);
    assert_quantum_fixture("quantum_anchor_batch/doc_fallback.json", &actual);
    Ok(())
}

#[test]
fn test_quantum_contexts_from_anchor_batch_rejects_missing_identifier_column()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "semantic_score",
        DataType::Float64,
        false,
    )]));
    let batch = RecordBatch::try_new(schema, vec![Arc::new(Float64Array::from(vec![0.8]))])?;

    let fixture = build_hybrid_fixture()?;
    let error = match fixture.index().quantum_contexts_from_anchor_batch(
        &batch,
        "anchor_ref",
        "semantic_score",
        &QuantumFusionOptions::default(),
    ) {
        Err(error) => error,
        Ok(contexts) => panic!(
            "missing identifier column should fail, but returned {} contexts",
            contexts.len()
        ),
    };

    assert!(matches!(
        error,
        QuantumContextBuildError::MissingInputColumn { ref column } if column == "anchor_ref"
    ));

    Ok(())
}

#[test]
fn test_quantum_contexts_from_anchor_batch_rejects_wrong_identifier_column_type()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("anchor_ref", DataType::Float64, false),
        Field::new("semantic_score", DataType::Float64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Float64Array::from(vec![1.0])),
            Arc::new(Float64Array::from(vec![0.8])),
        ],
    )?;

    let fixture = build_hybrid_fixture()?;
    let error = match fixture.index().quantum_contexts_from_anchor_batch(
        &batch,
        "anchor_ref",
        "semantic_score",
        &QuantumFusionOptions::default(),
    ) {
        Err(error) => error,
        Ok(contexts) => panic!(
            "wrong identifier column type should fail, but returned {} contexts",
            contexts.len()
        ),
    };

    assert!(matches!(
        error,
        QuantumContextBuildError::InvalidInputUtf8Column { .. }
    ));

    Ok(())
}

#[test]
fn test_quantum_contexts_from_anchor_batch_rejects_wrong_score_column_type()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("anchor_ref", DataType::Utf8, false),
        Field::new("semantic_score", DataType::Utf8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec!["docs/alpha"])),
            Arc::new(StringArray::from(vec!["0.8"])),
        ],
    )?;

    let fixture = build_hybrid_fixture()?;
    let error = match fixture.index().quantum_contexts_from_anchor_batch(
        &batch,
        "anchor_ref",
        "semantic_score",
        &QuantumFusionOptions::default(),
    ) {
        Err(error) => error,
        Ok(contexts) => panic!(
            "wrong score column type should fail, but returned {} contexts",
            contexts.len()
        ),
    };

    assert!(matches!(
        error,
        QuantumContextBuildError::InvalidInputFloat64Column { .. }
    ));

    Ok(())
}

#[test]
fn test_quantum_contexts_from_anchor_batch_rejects_null_identifier_values()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("anchor_ref", DataType::Utf8, true),
        Field::new("semantic_score", DataType::Float64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec![Some("docs/alpha"), None])),
            Arc::new(Float64Array::from(vec![0.8, 0.2])),
        ],
    )?;

    let fixture = build_hybrid_fixture()?;
    let error = match fixture.index().quantum_contexts_from_anchor_batch(
        &batch,
        "anchor_ref",
        "semantic_score",
        &QuantumFusionOptions::default(),
    ) {
        Err(error) => error,
        Ok(contexts) => panic!(
            "null id rows should fail, but returned {} contexts",
            contexts.len()
        ),
    };

    assert!(matches!(
        error,
        QuantumContextBuildError::NullInputValue { row: 1, .. }
    ));

    Ok(())
}
