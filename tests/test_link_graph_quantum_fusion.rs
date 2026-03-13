//! Integration tests for quantum-fusion context construction.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;
#[path = "support/link_graph_fixture_tree.rs"]
mod link_graph_fixture_tree;
#[path = "support/link_graph_hybrid_fixture.rs"]
mod link_graph_hybrid_fixture;
#[path = "support/quantum_fixture_support.rs"]
mod quantum_fixture_support;

use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use arrow::array::{ArrayRef, Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use quantum_fixture_support::{
    assert_quantum_fixture, build_hybrid_fixture, contexts_snapshot,
    default_quantum_fusion_options, page_index_fallback_snapshot,
};
use serde_json::json;
use xiuxian_vector::VectorStore;
use xiuxian_wendao::{
    BatchQuantumScorer, QUANTUM_SALIENCY_COLUMN, QuantumAnchorHit, QuantumContextBuildError,
    QuantumFusionOptions, QuantumSemanticIgnition, QuantumSemanticIgnitionFuture,
    QuantumSemanticSearchRequest, VectorStoreSemanticIgnition,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct StubIgnitionError(&'static str);

impl Display for StubIgnitionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for StubIgnitionError {}

struct AssertingIgnition {
    anchor_id: String,
    calls: Arc<AtomicUsize>,
}

impl QuantumSemanticIgnition for AssertingIgnition {
    type Error = StubIgnitionError;

    fn backend_name(&self) -> &str {
        "stub-semantic"
    }

    fn search_anchors<'a>(
        &'a self,
        request: QuantumSemanticSearchRequest<'a>,
    ) -> QuantumSemanticIgnitionFuture<'a, Self::Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(request.query_text, Some("leaf topic"));
        assert_eq!(request.query_vector, [0.1_f32, 0.2, 0.3].as_slice());
        assert_eq!(request.candidate_limit, 1);
        assert_eq!(request.min_vector_score, None);
        assert_eq!(request.max_vector_score, None);

        let anchor_id = self.anchor_id.clone();
        Box::pin(async move {
            Ok(vec![QuantumAnchorHit {
                anchor_id,
                vector_score: 0.75,
            }])
        })
    }
}

#[test]
fn test_quantum_contexts_from_anchors_recover_semantic_path_and_fuse_scores()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = build_hybrid_fixture()?;
    let anchor_id = fixture.alpha_leaf_anchor_id()?;

    let contexts = fixture.index().quantum_contexts_from_anchors(
        &[QuantumAnchorHit {
            anchor_id,
            vector_score: 0.8,
        }],
        &default_quantum_fusion_options(),
    )?;

    let actual = contexts_snapshot(&contexts);
    assert_quantum_fixture("quantum_fusion/contexts_from_anchors.json", &actual);
    Ok(())
}

#[test]
fn test_page_index_semantic_path_supports_anchor_and_doc_fallbacks()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = build_hybrid_fixture()?;
    let contexts = fixture.index().quantum_contexts_from_anchors(
        &[
            QuantumAnchorHit {
                anchor_id: "plain".to_string(),
                vector_score: 0.4,
            },
            QuantumAnchorHit {
                anchor_id: "missing#anchor".to_string(),
                vector_score: 0.9,
            },
        ],
        &QuantumFusionOptions {
            alpha: 0.5,
            max_distance: 1,
            related_limit: 4,
            ppr: None,
        },
    )?;

    let actual = page_index_fallback_snapshot(fixture.index(), "plain", &contexts);
    assert_quantum_fixture("quantum_fusion/page_index_doc_fallback.json", &actual);
    Ok(())
}

#[test]
fn test_quantum_contexts_from_anchors_keep_duplicate_anchor_rows_distinct()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = build_hybrid_fixture()?;
    let anchor_id = fixture.alpha_leaf_anchor_id()?;

    let contexts = fixture.index().quantum_contexts_from_anchors(
        &[
            QuantumAnchorHit {
                anchor_id: anchor_id.clone(),
                vector_score: 0.9,
            },
            QuantumAnchorHit {
                anchor_id,
                vector_score: 0.2,
            },
        ],
        &QuantumFusionOptions {
            alpha: 0.7,
            max_distance: 2,
            related_limit: 2,
            ppr: None,
        },
    )?;

    let actual = contexts_snapshot(&contexts);
    assert_quantum_fixture("quantum_fusion/duplicate_anchor_rows.json", &actual);
    Ok(())
}

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
fn test_quantum_scoring_preserves_arrow_columns_for_zero_copy()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("anchor_id", DataType::Utf8, false),
        Field::new("vector_score", DataType::Float64, false),
    ]));
    let anchor_ids: ArrayRef = Arc::new(StringArray::from(vec!["a", "b"]));
    let vector_scores: ArrayRef = Arc::new(Float64Array::from(vec![0.2, 0.4]));
    let batch = RecordBatch::try_new(schema, vec![anchor_ids.clone(), vector_scores.clone()])?;

    let scorer = BatchQuantumScorer::new(&QuantumFusionOptions::default());
    let scored = scorer.score_batch(&batch, &HashMap::new(), "anchor_id", "vector_score")?;

    let actual = json!({
        "reused_columns": {
            "anchor_id": Arc::ptr_eq(&anchor_ids, scored.column(0)),
            "vector_score": Arc::ptr_eq(&vector_scores, scored.column(1)),
        },
        "schema": {
            "input": ["anchor_id", "vector_score"],
            "output": scored
                .schema_ref()
                .fields()
                .iter()
                .map(|field| field.name())
                .collect::<Vec<_>>(),
        },
        "saliency_column": QUANTUM_SALIENCY_COLUMN,
    });

    assert_quantum_fixture("quantum_fusion/arrow_zero_copy.json", &actual);
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

#[tokio::test]
async fn test_quantum_contexts_from_semantic_ignition_delegates_and_recovers_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = build_hybrid_fixture()?;
    let anchor_id = fixture.alpha_leaf_anchor_id()?;
    let calls = Arc::new(AtomicUsize::new(0));
    let ignition = AssertingIgnition {
        anchor_id,
        calls: Arc::clone(&calls),
    };

    let contexts = fixture
        .index()
        .quantum_contexts_from_semantic_ignition(
            &ignition,
            QuantumSemanticSearchRequest {
                query_text: Some("  leaf topic  "),
                query_vector: &[0.1_f32, 0.2, 0.3],
                candidate_limit: 0,
                min_vector_score: None,
                max_vector_score: None,
            },
            &QuantumFusionOptions {
                alpha: 0.7,
                max_distance: 2,
                related_limit: 2,
                ppr: None,
            },
        )
        .await?;

    let actual = json!({
        "calls": calls.load(Ordering::SeqCst),
        "result": contexts_snapshot(&contexts),
    });
    assert_quantum_fixture(
        "semantic_ignition/delegates_and_recovers_trace.json",
        &actual,
    );
    Ok(())
}

#[tokio::test]
async fn test_vector_store_semantic_ignition_builds_contexts()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = build_hybrid_fixture()?;
    let anchor_id = fixture.alpha_leaf_anchor_id()?;

    let temp_dir = tempfile::TempDir::new()?;
    let store = VectorStore::new(temp_dir.path().to_str().unwrap(), Some(2)).await?;
    store
        .add_documents(
            "anchors",
            vec![anchor_id.clone(), "docs/beta".to_string()],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec!["alpha".to_string(), "beta".to_string()],
            vec!["{}".to_string(), "{}".to_string()],
        )
        .await?;

    let ignition = VectorStoreSemanticIgnition::new(store, "anchors");
    let query_vector = vec![1.0_f32, 0.0];
    let request = QuantumSemanticSearchRequest {
        query_text: Some("alpha"),
        query_vector: &query_vector,
        candidate_limit: 2,
        min_vector_score: None,
        max_vector_score: None,
    };

    let contexts = fixture
        .index()
        .quantum_contexts_from_semantic_ignition(
            &ignition,
            request,
            &default_quantum_fusion_options(),
        )
        .await?;

    assert!(!contexts.is_empty());
    assert_eq!(contexts[0].anchor_id, anchor_id);
    assert!(contexts[0].vector_score > 0.9);
    Ok(())
}
