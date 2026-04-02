#![cfg(feature = "julia")]

//! Integration test for planned-search Julia rerank against the official
//! WendaoArrow stream scoring example.

use serial_test::serial;
use std::fs;
use xiuxian_vector::VectorStore;
use xiuxian_wendao::{
    LinkGraphIndex, LinkGraphSearchOptions, set_link_graph_wendao_config_override,
};
use xiuxian_wendao_julia::compatibility::link_graph::DEFAULT_JULIA_RERANK_FLIGHT_ROUTE;

use crate::support::wendaoarrow_official_examples::spawn_wendaoarrow_stream_scoring_service;

#[test]
#[serial(link_graph_runtime_config)]
fn test_planned_search_payload_applies_official_stream_scoring_example()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("alpha.md"),
        "# Alpha\n\nalpha signal remains dominant.\n",
    )?;
    fs::write(
        temp.path().join("beta.md"),
        "# Beta\n\nbeta remains as the contrasting note.\n",
    )?;

    let vector_store_path = temp.path().join("vector-store");
    let store = runtime.block_on(VectorStore::new(
        vector_store_path.to_string_lossy().as_ref(),
        Some(3),
    ))?;
    runtime.block_on(store.add_documents(
        "wendao_semantic_docs",
        vec!["alpha".to_string(), "beta".to_string()],
        vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]],
        vec!["alpha anchor".to_string(), "beta anchor".to_string()],
        vec!["{}".to_string(), "{}".to_string()],
    ))?;

    let (server_base_url, mut server_guard) =
        runtime.block_on(spawn_wendaoarrow_stream_scoring_service());
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        format!(
            r#"[link_graph.retrieval]
mode = "hybrid"
candidate_multiplier = 2
max_sources = 2
graph_rows_per_source = 2

[link_graph.retrieval.semantic_ignition]
backend = "vector-store"
vector_store_path = "{}"
table_name = "wendao_semantic_docs"

[link_graph.retrieval.julia_rerank]
base_url = "{}"
route = "{}"
schema_version = "v1"
timeout_secs = 10
"#,
            vector_store_path.to_string_lossy(),
            server_base_url,
            DEFAULT_JULIA_RERANK_FLIGHT_ROUTE,
        ),
    )?;
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let index = LinkGraphIndex::build(temp.path())?;
    let payload = index.search_planned_payload_with_agentic_query_vector(
        "alpha signal",
        &[1.0, 0.0, 0.0],
        2,
        LinkGraphSearchOptions::default(),
        None,
        None,
    );
    server_guard.kill();

    assert_eq!(
        payload
            .julia_rerank
            .as_ref()
            .map(|telemetry| telemetry.applied),
        Some(true)
    );
    assert_eq!(
        payload
            .julia_rerank
            .as_ref()
            .map(|telemetry| telemetry.response_row_count),
        Some(2)
    );
    assert_eq!(payload.quantum_contexts.len(), 2);
    assert!(
        payload.quantum_contexts.iter().all(|context| {
            (context.saliency_score - context.vector_score).abs() < f64::EPSILON
        }),
        "official stream scoring example should preserve vector_score as final_score"
    );

    Ok(())
}
