use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

pub type TestResultPath = Result<PathBuf, Box<dyn std::error::Error>>;

pub fn assert_repo_json_snapshot(name: &str, value: impl Serialize) {
    insta::with_settings!({
        snapshot_path => "../snapshots/repo_intelligence",
        prepend_module_to_snapshot => false,
        sort_maps => true,
    }, {
        insta::assert_json_snapshot!(name, value);
    });
}

pub fn write_repo_config(base: &Path, repo_dir: &Path, repo_id: &str) -> TestResultPath {
    let config_path = base.join(format!("{repo_id}.wendao.toml"));
    fs::write(
        &config_path,
        format!(
            r#"[link_graph.projects.{repo_id}]
root = "{}"
plugins = ["julia"]
"#,
            repo_dir.display()
        ),
    )?;
    Ok(config_path)
}
