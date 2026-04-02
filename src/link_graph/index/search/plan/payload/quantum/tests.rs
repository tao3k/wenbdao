use super::rerank::{
    apply_plugin_rerank_scores, attach_plugin_rerank_request_metadata,
    build_plugin_rerank_telemetry, build_plugin_rerank_transport_client,
    collect_plugin_rerank_trace_ids, plugin_rerank_request_trace_id,
};
use crate::analyzers::PluginArrowScoreRow;
use crate::link_graph::models::QuantumContext;
use crate::link_graph::plugin_runtime::{
    NegotiatedTransportSelection, build_rerank_provider_binding,
};
use crate::link_graph::runtime_config::models::retrieval::LinkGraphCompatRerankRuntimeConfig;
use arrow::array::StringArray;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::collections::BTreeMap;
use std::sync::Arc;
use xiuxian_wendao_core::transport::PluginTransportKind;

#[test]
fn apply_plugin_rerank_scores_overwrites_saliency_and_resorts_contexts() {
    let mut contexts = vec![
        QuantumContext {
            anchor_id: "doc-1#a".to_string(),
            doc_id: "doc-1".to_string(),
            path: "notes/doc-1.md".to_string(),
            semantic_path: vec![],
            trace_label: None,
            related_clusters: vec![],
            saliency_score: 0.2,
            vector_score: 0.6,
            topology_score: 0.1,
        },
        QuantumContext {
            anchor_id: "doc-2#b".to_string(),
            doc_id: "doc-2".to_string(),
            path: "notes/doc-2.md".to_string(),
            semantic_path: vec![],
            trace_label: None,
            related_clusters: vec![],
            saliency_score: 0.9,
            vector_score: 0.5,
            topology_score: 0.2,
        },
    ];
    let response_rows = BTreeMap::from([
        (
            "doc-1#a".to_string(),
            PluginArrowScoreRow {
                doc_id: "doc-1#a".to_string(),
                analyzer_score: 0.7,
                final_score: 0.95,
                trace_id: None,
            },
        ),
        (
            "doc-2#b".to_string(),
            PluginArrowScoreRow {
                doc_id: "doc-2#b".to_string(),
                analyzer_score: 0.3,
                final_score: 0.4,
                trace_id: None,
            },
        ),
    ]);

    let updated = apply_plugin_rerank_scores(&mut contexts, &response_rows);

    assert_eq!(updated, 2);
    assert_eq!(contexts[0].anchor_id, "doc-1#a");
    assert!((contexts[0].saliency_score - 0.95).abs() < f64::EPSILON);
    assert_eq!(contexts[1].anchor_id, "doc-2#b");
    assert!((contexts[1].saliency_score - 0.4).abs() < f64::EPSILON);
}

#[test]
fn build_plugin_rerank_transport_client_honors_runtime_overrides() {
    let binding = build_rerank_provider_binding(&LinkGraphCompatRerankRuntimeConfig {
        base_url: Some("http://127.0.0.1:8090".to_string()),
        route: Some("/custom-rerank".to_string()),
        health_route: Some("/healthz".to_string()),
        schema_version: Some("v1".to_string()),
        timeout_secs: Some(15),
        service_mode: None,
        analyzer_config_path: None,
        analyzer_strategy: None,
        vector_weight: None,
        similarity_weight: None,
    });
    let client = build_plugin_rerank_transport_client(&binding)
        .expect("config should be valid")
        .expect("base url should enable transport");

    assert_eq!(
        client.selection().selected_transport,
        PluginTransportKind::ArrowFlight
    );
    assert_eq!(client.flight_base_url(), Some("http://127.0.0.1:8090"));
    assert_eq!(client.flight_route(), Some("/custom-rerank"));
}

#[test]
fn build_plugin_rerank_transport_client_accepts_arrow_flight_bindings() {
    let mut binding = build_rerank_provider_binding(&LinkGraphCompatRerankRuntimeConfig {
        base_url: Some("http://127.0.0.1:18080".to_string()),
        route: Some("/rerank".to_string()),
        health_route: Some("/healthz".to_string()),
        schema_version: Some("v2".to_string()),
        timeout_secs: Some(20),
        service_mode: None,
        analyzer_config_path: None,
        analyzer_strategy: None,
        vector_weight: None,
        similarity_weight: None,
    });
    binding.transport = PluginTransportKind::ArrowFlight;

    let client = build_plugin_rerank_transport_client(&binding)
        .expect("flight binding should be negotiable")
        .expect("flight binding should materialize a lazy Flight client");

    assert_eq!(
        client.selection().selected_transport,
        PluginTransportKind::ArrowFlight
    );
    assert_eq!(client.flight_base_url(), Some("http://127.0.0.1:18080"));
    assert_eq!(client.flight_route(), Some("/rerank"));
}

#[test]
fn collect_plugin_rerank_trace_ids_deduplicates_non_empty_values() {
    let response_rows = BTreeMap::from([
        (
            "doc-1#a".to_string(),
            PluginArrowScoreRow {
                doc_id: "doc-1#a".to_string(),
                analyzer_score: 0.7,
                final_score: 0.95,
                trace_id: Some("trace-123".to_string()),
            },
        ),
        (
            "doc-2#b".to_string(),
            PluginArrowScoreRow {
                doc_id: "doc-2#b".to_string(),
                analyzer_score: 0.3,
                final_score: 0.4,
                trace_id: Some("trace-123".to_string()),
            },
        ),
        (
            "doc-3#c".to_string(),
            PluginArrowScoreRow {
                doc_id: "doc-3#c".to_string(),
                analyzer_score: 0.2,
                final_score: 0.1,
                trace_id: Some("trace-456".to_string()),
            },
        ),
    ]);

    let trace_ids = collect_plugin_rerank_trace_ids(&response_rows);

    assert_eq!(
        trace_ids,
        vec!["trace-123".to_string(), "trace-456".to_string()]
    );
}

#[test]
fn build_plugin_rerank_telemetry_carries_transport_selection_and_fallback() {
    let telemetry = build_plugin_rerank_telemetry(
        Some(&NegotiatedTransportSelection {
            selected_transport: PluginTransportKind::ArrowFlight,
            fallback_from: None,
            fallback_reason: None,
        }),
        true,
        2,
        vec!["trace-123".to_string()],
        None,
    );

    assert!(telemetry.applied);
    assert_eq!(telemetry.response_row_count, 2);
    assert_eq!(
        telemetry.selected_transport,
        Some(PluginTransportKind::ArrowFlight)
    );
    assert_eq!(telemetry.fallback_from, None);
    assert_eq!(telemetry.fallback_reason, None);
    assert_eq!(telemetry.trace_ids, vec!["trace-123".to_string()]);
    assert_eq!(telemetry.error, None);
}

#[test]
fn plugin_rerank_request_trace_id_normalizes_query_text() {
    assert_eq!(
        plugin_rerank_request_trace_id("  alpha   signal "),
        "julia-rerank:alpha_signal"
    );
    assert_eq!(plugin_rerank_request_trace_id(""), "julia-rerank:query");
}

#[test]
fn attach_plugin_rerank_request_metadata_sets_schema_metadata() {
    let batch = RecordBatch::try_new(
        Arc::new(Schema::new(vec![Field::new(
            "doc_id",
            DataType::Utf8,
            false,
        )])),
        vec![Arc::new(StringArray::from(vec!["doc-1"]))],
    )
    .expect("batch");

    let traced_batch =
        attach_plugin_rerank_request_metadata(batch, "alpha signal", "v1").expect("metadata");

    assert_eq!(
        traced_batch.schema().metadata().get("trace_id"),
        Some(&"julia-rerank:alpha_signal".to_string())
    );
    assert_eq!(
        traced_batch
            .schema()
            .metadata()
            .get("wendao.schema_version"),
        Some(&"v1".to_string())
    );
}
