use super::*;
use crate::set_link_graph_wendao_config_override;
use serial_test::serial;
use std::fs;
use xiuxian_wendao_julia::compatibility::link_graph::{
    DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH, DEFAULT_JULIA_RERANK_FLIGHT_ROUTE,
    JULIA_DEPLOYMENT_ARTIFACT_ID, JULIA_PLUGIN_ID,
};

#[test]
fn wendao_plugin_artifact_args_deserialize_selector_and_format() {
    let args: WendaoPluginArtifactArgs = serde_json::from_value(serde_json::json!({
        "plugin_id": JULIA_PLUGIN_ID,
        "artifact_id": JULIA_DEPLOYMENT_ARTIFACT_ID,
        "output_format": "json"
    }))
    .expect("generic plugin-artifact args should deserialize");

    assert_eq!(args.plugin_id, JULIA_PLUGIN_ID);
    assert_eq!(args.artifact_id, JULIA_DEPLOYMENT_ARTIFACT_ID);
    assert!(matches!(
        args.output_format,
        WendaoPluginArtifactOutputFormat::Json
    ));
}

#[test]
fn wendao_plugin_artifact_args_default_to_toml_output() {
    let args: WendaoPluginArtifactArgs = serde_json::from_value(serde_json::json!({
        "plugin_id": JULIA_PLUGIN_ID,
        "artifact_id": JULIA_DEPLOYMENT_ARTIFACT_ID
    }))
    .expect("generic plugin-artifact args should deserialize");

    assert!(matches!(
        args.output_format,
        WendaoPluginArtifactOutputFormat::Toml
    ));
}

#[test]
fn wendao_plugin_artifact_args_deserialize_output_path() {
    let args: WendaoPluginArtifactArgs = serde_json::from_value(serde_json::json!({
        "plugin_id": JULIA_PLUGIN_ID,
        "artifact_id": JULIA_DEPLOYMENT_ARTIFACT_ID,
        "output_format": "json",
        "output_path": ".run/julia/artifact.json"
    }))
    .expect("args with output_path should deserialize");

    assert_eq!(
        args.output_path.as_deref(),
        Some(".run/julia/artifact.json")
    );
}

#[test]
#[serial]
fn render_plugin_artifact_toml_uses_runtime_config() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
analyzer_strategy = "similarity_only"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let selector = build_plugin_artifact_selector(JULIA_PLUGIN_ID, JULIA_DEPLOYMENT_ARTIFACT_ID)
        .expect("build plugin selector");
    let rendered = render_plugin_artifact_toml(&selector).expect("render toml");
    assert!(rendered.contains("artifact_schema_version = \"v1\""));
    assert!(rendered.contains("generated_at = "));
    assert!(rendered.contains("base_url = \"http://127.0.0.1:8088\""));
    assert!(rendered.contains(&format!(
        "launcher_path = \"{DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH}\""
    )));
    assert!(rendered.contains("\"similarity_only\""));
}

#[test]
#[serial]
fn render_plugin_artifact_json_uses_runtime_config() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
analyzer_strategy = "similarity_only"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let selector = build_plugin_artifact_selector(JULIA_PLUGIN_ID, JULIA_DEPLOYMENT_ARTIFACT_ID)
        .expect("build plugin selector");
    let rendered = render_plugin_artifact_json(&selector).expect("render json");
    assert!(rendered.contains("\"artifact_schema_version\": \"v1\""));
    assert!(rendered.contains("\"generated_at\": "));
    assert!(rendered.contains("\"base_url\": \"http://127.0.0.1:8088\""));
    assert!(rendered.contains(&format!(
        "\"route\": \"{DEFAULT_JULIA_RERANK_FLIGHT_ROUTE}\""
    )));
    assert!(rendered.contains(&format!(
        "\"launcher_path\": \"{DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH}\""
    )));
}

#[test]
#[serial]
fn render_plugin_artifact_uses_selected_format() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let selector = build_plugin_artifact_selector(JULIA_PLUGIN_ID, JULIA_DEPLOYMENT_ARTIFACT_ID)
        .expect("build plugin selector");
    let rendered = render_plugin_artifact(&selector, WendaoPluginArtifactOutputFormat::Json)
        .expect("render generic plugin artifact");

    assert!(rendered.contains("\"artifact_schema_version\": \"v1\""));
    assert!(rendered.contains(&format!(
        "\"route\": \"{DEFAULT_JULIA_RERANK_FLIGHT_ROUTE}\""
    )));
}

#[test]
#[serial]
fn export_plugin_artifact_writes_json_file_when_requested() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let output_path = temp.path().join("exports").join("plugin-artifact.json");
    let message = export_plugin_artifact(WendaoPluginArtifactArgs {
        plugin_id: JULIA_PLUGIN_ID.to_string(),
        artifact_id: JULIA_DEPLOYMENT_ARTIFACT_ID.to_string(),
        output_format: WendaoPluginArtifactOutputFormat::Json,
        output_path: Some(output_path.to_string_lossy().to_string()),
    })
    .expect("export generic plugin artifact");

    assert!(message.contains("Wrote plugin artifact"));
    assert!(message.contains(JULIA_PLUGIN_ID));
    assert!(message.contains(JULIA_DEPLOYMENT_ARTIFACT_ID));
    let written = fs::read_to_string(&output_path).expect("read written json");
    assert!(written.contains("\"artifact_schema_version\": \"v1\""));
    assert!(written.contains(&format!(
        "\"route\": \"{DEFAULT_JULIA_RERANK_FLIGHT_ROUTE}\""
    )));
}
