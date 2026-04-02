use crate::link_graph::plugin_runtime::resolve_plugin_artifact_for_selector;
use crate::link_graph::runtime_config::julia_deployment_artifact_selector;
use crate::link_graph::set_link_graph_wendao_config_override;
use serial_test::serial;
use std::fs;
use xiuxian_wendao_core::transport::PluginTransportKind;

#[test]
#[serial]
fn resolve_plugin_artifact_resolves_julia_deployment_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/rerank"
health_route = "/healthz"
schema_version = "v1"
timeout_secs = 15
service_mode = "stream"
"#,
    )?;
    set_link_graph_wendao_config_override(&config_path.to_string_lossy());

    let selector = julia_deployment_artifact_selector();
    let artifact = resolve_plugin_artifact_for_selector(&selector).expect("artifact");
    assert_eq!(artifact.plugin_id, selector.plugin_id);
    assert_eq!(artifact.artifact_id, selector.artifact_id);
    assert_eq!(artifact.artifact_schema_version.0, "v1");
    assert_eq!(
        artifact
            .endpoint
            .as_ref()
            .and_then(|endpoint| endpoint.base_url.as_deref()),
        Some("http://127.0.0.1:8088")
    );
    assert_eq!(
        artifact.selected_transport,
        Some(PluginTransportKind::ArrowFlight)
    );
    assert_eq!(artifact.fallback_from, None);
    assert_eq!(artifact.fallback_reason, None);

    Ok(())
}
