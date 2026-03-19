use super::{resolve_link_graph_coactivation_runtime, resolve_link_graph_retrieval_policy_runtime};
use crate::link_graph::runtime_config::constants::{
    DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE, DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS,
    DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION,
    DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH,
};
use crate::link_graph::runtime_config::models::LinkGraphSemanticIgnitionBackend;
use crate::link_graph::set_link_graph_wendao_config_override;
use std::fs;

#[test]
fn test_coactivation_touch_queue_depth_default() {
    let runtime = resolve_link_graph_coactivation_runtime();
    assert_eq!(runtime.max_hops, DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS);
    assert_eq!(
        runtime.max_total_propagations,
        DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION.saturating_mul(2)
    );
    assert!(
        (runtime.hop_decay_scale - DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE).abs()
            <= f64::EPSILON,
        "unexpected hop_decay_scale: {}",
        runtime.hop_decay_scale
    );
    assert_eq!(
        runtime.touch_queue_depth,
        DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH
    );
}

#[test]
fn test_retrieval_runtime_resolves_semantic_ignition_config()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval]
mode = "hybrid"
candidate_multiplier = 3
max_sources = 5
graph_rows_per_source = 4

[link_graph.retrieval.semantic_ignition]
backend = "openai-compatible"
vector_store_path = ".cache/glm-anchor-store"
table_name = "glm_anchor_index"
embedding_base_url = "http://127.0.0.1:11434"
embedding_model = "glm-5"
"#,
    )?;
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let runtime = resolve_link_graph_retrieval_policy_runtime();
    assert_eq!(
        runtime.semantic_ignition.backend,
        LinkGraphSemanticIgnitionBackend::OpenAiCompatible
    );
    assert_eq!(runtime.candidate_multiplier, 3);
    assert_eq!(runtime.max_sources, 5);
    assert_eq!(runtime.graph_rows_per_source, 4);
    assert_eq!(
        runtime.semantic_ignition.vector_store_path.as_deref(),
        Some(".cache/glm-anchor-store")
    );
    assert_eq!(
        runtime.semantic_ignition.table_name.as_deref(),
        Some("glm_anchor_index")
    );
    assert_eq!(
        runtime.semantic_ignition.embedding_base_url.as_deref(),
        Some("http://127.0.0.1:11434")
    );
    assert_eq!(
        runtime.semantic_ignition.embedding_model.as_deref(),
        Some("glm-5")
    );

    Ok(())
}
