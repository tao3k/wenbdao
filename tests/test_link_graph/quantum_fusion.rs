use super::quantum_fixture_support::{
    assert_quantum_fixture, build_hybrid_fixture, contexts_snapshot,
    default_quantum_fusion_options, page_index_fallback_snapshot,
};
use xiuxian_wendao::{QuantumAnchorHit, QuantumFusionOptions};

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
