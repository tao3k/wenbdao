use crate::link_graph::plugin_runtime::render_plugin_artifact_toml_for_selector;
use crate::link_graph::runtime_config::julia_deployment_artifact_selector;
use crate::link_graph::set_link_graph_wendao_config_override;
use serial_test::serial;
use std::fs;

#[test]
#[serial]
fn render_plugin_artifact_toml_renders_julia_deployment_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/rerank"
schema_version = "v1"
"#,
    )?;
    set_link_graph_wendao_config_override(&config_path.to_string_lossy());

    let rendered = render_plugin_artifact_toml_for_selector(&julia_deployment_artifact_selector())?
        .expect("rendered artifact");
    assert!(rendered.contains("plugin_id = \"xiuxian-wendao-julia\""));
    assert!(rendered.contains("artifact_id = \"deployment\""));
    assert!(rendered.contains("route = \"/rerank\""));
    assert!(rendered.contains("selected_transport = \"arrow_flight\""));

    Ok(())
}
