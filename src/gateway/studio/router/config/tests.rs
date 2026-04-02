use std::fs;

use crate::gateway::studio::router::config::types::{WendaoTomlConfig, WendaoTomlPluginEntry};
use crate::gateway::studio::router::config::{
    load_ui_config_from_wendao_toml, persist_ui_config_to_wendao_toml,
};
use crate::gateway::studio::types::{UiConfig, UiRepoProjectConfig};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn load_ui_config_from_wendao_toml_accepts_inline_repo_plugin_config() -> TestResult {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("wendao.toml"),
        r#"[link_graph.projects.sample]
root = "."
plugins = [
  "julia",
  { id = "julia", flight_transport = { base_url = "http://127.0.0.1:8815" } }
]
"#,
    )?;

    let config = load_ui_config_from_wendao_toml(temp.path()).expect("ui config should load");
    assert_eq!(config.repo_projects.len(), 1);
    assert_eq!(config.repo_projects[0].id, "sample");
    assert_eq!(config.repo_projects[0].plugins, vec!["julia".to_string()]);
    Ok(())
}

#[test]
fn persist_ui_config_to_wendao_toml_preserves_inline_repo_plugin_config() -> TestResult {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.projects.sample]
root = "."
plugins = [
  "julia",
  { id = "julia", flight_transport = { base_url = "http://127.0.0.1:8815", route = "/rerank" } }
]
"#,
    )?;

    persist_ui_config_to_wendao_toml(
        temp.path(),
        &UiConfig {
            projects: Vec::new(),
            repo_projects: vec![UiRepoProjectConfig {
                id: "sample".to_string(),
                root: Some(".".to_string()),
                url: None,
                git_ref: None,
                refresh: None,
                plugins: vec!["julia".to_string()],
            }],
        },
    )?;

    let persisted: WendaoTomlConfig = toml::from_str(&fs::read_to_string(&config_path)?)?;
    let project = persisted
        .link_graph
        .projects
        .get("sample")
        .expect("sample project should persist");
    assert_eq!(project.plugins.len(), 2);
    assert!(matches!(
        &project.plugins[0],
        WendaoTomlPluginEntry::Id(id) if id == "julia"
    ));
    assert!(matches!(
        &project.plugins[1],
        WendaoTomlPluginEntry::Config(config)
            if config.id == "julia" && config.extra.contains_key("flight_transport")
    ));
    Ok(())
}
