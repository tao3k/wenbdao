#![cfg(feature = "julia")]

//! Integration tests for planned-search Julia rerank on the vector-store backend.

use serial_test::serial;
use std::fs;
use xiuxian_vector::VectorStore;
use xiuxian_wendao::{
    LinkGraphIndex, LinkGraphSearchOptions, set_link_graph_wendao_config_override,
};
use xiuxian_wendao_julia::compatibility::link_graph::DEFAULT_JULIA_RERANK_FLIGHT_ROUTE;

use crate::support::wendaoarrow_custom_service::{
    WendaoArrowScoreRow, spawn_wendaoarrow_custom_scoring_service,
};

#[test]
#[serial(link_graph_runtime_config)]
fn test_planned_search_payload_with_query_vector_applies_julia_rerank_on_vector_store()
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
        runtime.block_on(spawn_wendaoarrow_custom_scoring_service(&[
            WendaoArrowScoreRow {
                doc_id: "alpha",
                analyzer_score: 0.3,
                final_score: 0.2,
            },
            WendaoArrowScoreRow {
                doc_id: "beta",
                analyzer_score: 0.95,
                final_score: 0.99,
            },
        ]));
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
    assert_eq!(payload.quantum_contexts.len(), 2);
    assert_eq!(payload.quantum_contexts[0].doc_id, "beta");
    assert!((payload.quantum_contexts[0].saliency_score - 0.99).abs() < f64::EPSILON);
    assert_eq!(payload.quantum_contexts[1].doc_id, "alpha");
    assert!((payload.quantum_contexts[1].saliency_score - 0.2).abs() < f64::EPSILON);

    Ok(())
}
