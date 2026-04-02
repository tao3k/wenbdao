use std::fs;

use serde_json::json;

use super::{RepositoryPluginConfig, RepositoryRefreshPolicy, load_repo_intelligence_config};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn load_repo_intelligence_config_parses_inline_plugin_config() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = temp.path().join("repos").join("sample");
    fs::create_dir_all(&repo_dir)?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.projects.sample]
root = "repos/sample"
refresh = "manual"
plugins = [
  "julia",
  { id = "julia", flight_transport = { base_url = "http://127.0.0.1:8815", route = "/rerank", timeout_secs = 15 } }
]
"#,
    )?;

    let config = load_repo_intelligence_config(Some(&config_path), temp.path())?;
    assert_eq!(config.repos.len(), 1);
    let repository = &config.repos[0];
    assert_eq!(repository.id, "sample");
    assert_eq!(repository.refresh, RepositoryRefreshPolicy::Manual);
    assert_eq!(repository.path.as_deref(), Some(repo_dir.as_path()));
    assert_eq!(
        repository.plugins,
        vec![
            RepositoryPluginConfig::Id("julia".to_string()),
            RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: json!({
                    "flight_transport": {
                        "base_url": "http://127.0.0.1:8815",
                        "route": "/rerank",
                        "timeout_secs": 15,
                    }
                }),
            },
        ]
    );
    Ok(())
}

#[test]
fn load_repo_intelligence_config_rejects_empty_inline_plugin_id() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = temp.path().join("repos").join("sample");
    fs::create_dir_all(&repo_dir)?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.projects.sample]
root = "repos/sample"
plugins = [{ id = "   ", flight_transport = { base_url = "http://127.0.0.1:8815" } }]
"#,
    )?;

    let error = load_repo_intelligence_config(Some(&config_path), temp.path())
        .expect_err("expected empty plugin id to be rejected");
    assert_eq!(
        error.to_string(),
        format!(
            "repo intelligence config load failed: failed to parse `{}`: repo `sample` plugin id cannot be empty",
            config_path.display()
        )
    );
    Ok(())
}
