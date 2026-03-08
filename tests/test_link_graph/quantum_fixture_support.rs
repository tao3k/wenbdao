use serde_json::{Value, json};
use xiuxian_wendao::{LinkGraphIndex, QuantumContext, QuantumFusionOptions};

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::link_graph_hybrid_fixture::{EXPECTED_FIXTURE_ROOT, HybridFixture};

const SCORE_PRECISION: f64 = 1_000_000_000_000.0;

pub(super) fn build_hybrid_fixture() -> Result<HybridFixture, Box<dyn std::error::Error>> {
    HybridFixture::build()
}

pub(super) fn default_quantum_fusion_options() -> QuantumFusionOptions {
    QuantumFusionOptions {
        alpha: 0.6,
        max_distance: 2,
        related_limit: 2,
        ppr: None,
    }
}

pub(super) fn assert_quantum_fixture(relative: &str, actual: &Value) {
    assert_json_fixture_eq(EXPECTED_FIXTURE_ROOT, relative, actual);
}

pub(super) fn contexts_snapshot(contexts: &[QuantumContext]) -> Value {
    json!({
        "contexts": contexts
            .iter()
            .map(snapshot_context)
            .collect::<Vec<_>>(),
    })
}

pub(super) fn page_index_fallback_snapshot(
    index: &LinkGraphIndex,
    anchor_id: &str,
    contexts: &[QuantumContext],
) -> Value {
    json!({
        "semantic_path": index.page_index_semantic_path(anchor_id),
        "trace_label": index.page_index_trace_label(anchor_id),
        "contexts": contexts
            .iter()
            .map(snapshot_context)
            .collect::<Vec<_>>(),
    })
}

fn snapshot_context(context: &QuantumContext) -> Value {
    let mut related_clusters = context.related_clusters.clone();
    related_clusters.sort();

    json!({
        "anchor_id": context.anchor_id,
        "semantic_path": context.semantic_path,
        "trace_label": context.trace_label(),
        "related_clusters": related_clusters,
        "saliency_score": round_score(context.saliency_score),
        "vector_score": round_score(context.vector_score),
        "topology_score": round_score(context.topology_score),
    })
}

fn round_score(value: f64) -> f64 {
    (value * SCORE_PRECISION).round() / SCORE_PRECISION
}
