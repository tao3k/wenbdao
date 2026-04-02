use crate::link_graph::plugin_runtime::{
    CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER, build_rerank_provider_binding,
};
use crate::link_graph::runtime_config::models::retrieval::{
    LinkGraphCompatRerankRuntimeConfig, julia_rerank_provider_selector,
};
use xiuxian_wendao_core::transport::PluginTransportKind;

#[test]
fn julia_rerank_runtime_converts_into_generic_binding() {
    let binding = build_rerank_provider_binding(&LinkGraphCompatRerankRuntimeConfig {
        base_url: Some("http://127.0.0.1:8088".to_string()),
        route: Some("/rerank".to_string()),
        health_route: Some("/healthz".to_string()),
        schema_version: Some("v2".to_string()),
        timeout_secs: Some(15),
        service_mode: Some("stream".to_string()),
        analyzer_config_path: Some("config/analyzer.toml".to_string()),
        analyzer_strategy: Some("linear_blend".to_string()),
        vector_weight: Some(0.7),
        similarity_weight: Some(0.3),
    });
    let selector = julia_rerank_provider_selector();

    assert_eq!(binding.selector, selector);
    assert_eq!(binding.transport, PluginTransportKind::ArrowFlight);
    assert_eq!(binding.contract_version.0, "v2");
    assert_eq!(
        binding.endpoint.base_url.as_deref(),
        Some("http://127.0.0.1:8088")
    );
    assert_eq!(binding.endpoint.route.as_deref(), Some("/rerank"));
    assert_eq!(binding.endpoint.health_route.as_deref(), Some("/healthz"));
    assert_eq!(binding.endpoint.timeout_secs, Some(15));
    let launch = binding.launch.expect("launch");
    assert!(launch.args.iter().any(|value| value == "--service-mode"));
    assert!(launch.args.iter().any(|value| value == "linear_blend"));
}

#[test]
fn plugin_runtime_transport_preference_order_is_flight_first() {
    assert_eq!(
        CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER,
        [PluginTransportKind::ArrowFlight]
    );
}
