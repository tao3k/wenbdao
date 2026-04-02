//! Unit tests for `zhenfa_router/rpc` module.

use super::*;
use crate::zhenfa_router::models::WendaoSearchRequest;

use crate::link_graph::{
    LinkGraphConfidenceLevel, LinkGraphJuliaRerankTelemetry, LinkGraphRetrievalMode,
    LinkGraphSemanticIgnitionTelemetry, QuantumContext,
};
use crate::set_link_graph_wendao_config_override;
use std::fs;
use xiuxian_wendao_julia::compatibility::link_graph::{
    DEFAULT_JULIA_RERANK_FLIGHT_ROUTE, JULIA_DEPLOYMENT_ARTIFACT_ID, JULIA_PLUGIN_ID,
};

#[test]
fn normalize_limit_clamps_range() {
    assert_eq!(normalize_limit(None), DEFAULT_SEARCH_LIMIT);
    assert_eq!(normalize_limit(Some(0)), 1);
    assert_eq!(normalize_limit(Some(3)), 3);
    assert_eq!(
        normalize_limit(Some(MAX_SEARCH_LIMIT + 10)),
        MAX_SEARCH_LIMIT
    );
}

#[test]
fn render_markdown_includes_hits() {
    let payload = LinkGraphPlannedSearchPayload {
        query: "router".to_string(),
        options: crate::link_graph::LinkGraphSearchOptions::default(),
        hits: vec![crate::link_graph::LinkGraphDisplayHit {
            stem: "alpha".to_string(),
            title: "Alpha Note".to_string(),
            path: "notes/alpha.md".to_string(),
            doc_type: None,
            tags: Vec::new(),
            score: 0.9,
            best_section: "Design".to_string(),
            match_reason: String::new(),
        }],
        hit_count: 1,
        section_hit_count: 1,
        requested_mode: LinkGraphRetrievalMode::Hybrid,
        selected_mode: LinkGraphRetrievalMode::Hybrid,
        reason: "graph_sufficient".to_string(),
        graph_hit_count: 1,
        source_hint_count: 1,
        graph_confidence_score: 0.9,
        graph_confidence_level: LinkGraphConfidenceLevel::High,
        retrieval_plan: None,
        semantic_ignition: Some(LinkGraphSemanticIgnitionTelemetry {
            backend: "openai_compatible".to_string(),
            backend_name: Some("openai-compatible+xiuxian-vector".to_string()),
            context_count: 1,
            error: None,
        }),
        julia_rerank: Some(LinkGraphJuliaRerankTelemetry {
            applied: false,
            response_row_count: 0,
            selected_transport: None,
            fallback_from: None,
            fallback_reason: None,
            trace_ids: Vec::new(),
            error: Some("not configured".to_string()),
        }),
        query_vector: None,
        quantum_contexts: vec![QuantumContext {
            anchor_id: "alpha".to_string(),
            doc_id: "alpha".to_string(),
            path: "notes/alpha.md".to_string(),
            semantic_path: vec!["Alpha".to_string(), "Design".to_string()],
            trace_label: None,
            related_clusters: Vec::new(),
            saliency_score: 0.88,
            vector_score: 0.91,
            topology_score: 0.42,
        }],
        results: vec![],
        provisional_suggestions: vec![],
        provisional_error: None,
        promoted_overlay: None,
        ccs_audit: None,
    };

    let rendered = render_markdown(&payload);
    assert!(rendered.contains("Wendao Search Results"));
    assert!(rendered.contains("Alpha Note"));
    assert!(rendered.contains("section: Design"));
    assert!(rendered.contains("semantic_ignition: openai-compatible+xiuxian-vector"));
    assert!(rendered.contains("Quantum Contexts"));
}

#[test]
fn wendao_search_request_deserializes_query_vector() {
    let request: WendaoSearchRequest = serde_json::from_value(serde_json::json!({
        "query": "alpha signal",
        "query_vector": [1.0, 0.0, 0.0]
    }))
    .expect("request should deserialize");

    assert_eq!(request.query, "alpha signal");
    assert_eq!(request.query_vector, Some(vec![1.0, 0.0, 0.0]));
}

#[test]
fn export_plugin_artifact_from_rpc_params_returns_toml() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:18080"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let rendered = export_plugin_artifact_from_rpc_params(serde_json::json!({
        "plugin_id": JULIA_PLUGIN_ID,
        "artifact_id": JULIA_DEPLOYMENT_ARTIFACT_ID,
    }))
    .expect("export generic toml");
    assert!(rendered.contains("artifact_schema_version = \"v1\""));
    assert!(rendered.contains(&format!("route = \"{DEFAULT_JULIA_RERANK_FLIGHT_ROUTE}\"")));
}

#[test]
fn export_plugin_artifact_from_rpc_params_returns_json() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:18080"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let rendered = export_plugin_artifact_from_rpc_params(serde_json::json!({
        "plugin_id": JULIA_PLUGIN_ID,
        "artifact_id": JULIA_DEPLOYMENT_ARTIFACT_ID,
        "output_format": "json"
    }))
    .expect("export generic json");
    assert!(rendered.contains("\"artifact_schema_version\": \"v1\""));
    assert!(rendered.contains(&format!(
        "\"route\": \"{DEFAULT_JULIA_RERANK_FLIGHT_ROUTE}\""
    )));
}

#[test]
fn export_plugin_artifact_from_rpc_params_writes_json_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:18080"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let output_path = temp.path().join("plugin-artifact.json");
    let rendered = export_plugin_artifact_from_rpc_params(serde_json::json!({
        "plugin_id": JULIA_PLUGIN_ID,
        "artifact_id": JULIA_DEPLOYMENT_ARTIFACT_ID,
        "output_format": "json",
        "output_path": output_path.to_string_lossy().to_string()
    }))
    .expect("export generic json file");

    assert!(rendered.contains("Wrote plugin artifact"));
    let written = fs::read_to_string(&output_path).expect("read written json");
    assert!(written.contains("\"base_url\": \"http://127.0.0.1:18080\""));
}
