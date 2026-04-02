use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::gateway::studio::types::UiConfig;

use super::paths::studio_wendao_toml_path;
use super::sanitize::merge_repo_plugins;
use super::types::{WendaoTomlConfig, WendaoTomlProjectConfig};

/// Persists UI config to `wendao.toml`.
///
/// # Errors
///
/// Returns an error string if reading, parsing, or writing fails.
pub fn persist_ui_config_to_wendao_toml(
    config_root: &Path,
    config: &UiConfig,
) -> Result<(), String> {
    let config_path = studio_wendao_toml_path(config_root);
    let mut parsed = if config_path.is_file() {
        let existing = fs::read_to_string(config_path.as_path()).map_err(|error| {
            format!(
                "failed to read `{}` before persisting UI config: {error}",
                config_path.display()
            )
        })?;
        toml::from_str::<WendaoTomlConfig>(&existing).unwrap_or_default()
    } else {
        WendaoTomlConfig::default()
    };

    let mut existing_projects = std::mem::take(&mut parsed.link_graph.projects);
    let mut projects = BTreeMap::<String, WendaoTomlProjectConfig>::new();
    for project in &config.projects {
        let mut entry = existing_projects.remove(&project.name).unwrap_or_default();
        entry.root = Some(project.root.clone());
        entry.dirs = project.dirs.clone();
        projects.insert(project.name.clone(), entry);
    }
    for repo in &config.repo_projects {
        let mut entry = projects
            .remove(&repo.id)
            .or_else(|| existing_projects.remove(&repo.id))
            .unwrap_or_default();
        if let Some(root) = repo.root.clone() {
            entry.root = Some(root);
        }
        entry.url.clone_from(&repo.url);
        entry.git_ref.clone_from(&repo.git_ref);
        entry.refresh.clone_from(&repo.refresh);
        entry.plugins = merge_repo_plugins(entry.plugins, &repo.plugins);
        projects.insert(repo.id.clone(), entry);
    }
    parsed.link_graph.projects = projects;

    let serialized = toml::to_string_pretty(&parsed).map_err(|error| {
        format!(
            "failed to serialize UI config into TOML `{}`: {error}",
            config_path.display()
        )
    })?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create config dir `{}`: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(config_path.as_path(), serialized).map_err(|error| {
        format!(
            "failed to write persisted UI config `{}`: {error}",
            config_path.display()
        )
    })
}
