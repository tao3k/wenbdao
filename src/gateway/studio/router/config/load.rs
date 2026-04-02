use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::gateway::studio::types::{UiConfig, UiProjectConfig, UiRepoProjectConfig};

use super::paths::studio_wendao_toml_path;
use super::sanitize::{
    sanitize_path_like, sanitize_path_list, sanitize_projects, sanitize_repo_projects,
};
use super::types::WendaoTomlConfig;

/// Loads UI config from `wendao.toml` if it exists.
#[must_use]
pub fn load_ui_config_from_wendao_toml(config_root: &Path) -> Option<UiConfig> {
    let config_path = studio_wendao_toml_path(config_root);
    let contents = fs::read_to_string(config_path).ok()?;
    let parsed: WendaoTomlConfig = toml::from_str(&contents).ok()?;
    Some(ui_config_from_wendao_toml(parsed))
}

fn ui_config_from_wendao_toml(parsed: WendaoTomlConfig) -> UiConfig {
    let mut projects = Vec::new();
    let mut repo_projects = Vec::new();

    for (id, project) in parsed.link_graph.projects {
        let dirs = sanitize_path_list(&project.dirs);
        let root = project
            .root
            .as_deref()
            .and_then(sanitize_path_like)
            .unwrap_or_else(|| ".".to_string());
        if !dirs.is_empty() {
            projects.push(UiProjectConfig {
                name: id.clone(),
                root,
                dirs,
            });
        }

        let mut plugin_seen = HashSet::<String>::new();
        let plugins = project
            .plugins
            .into_iter()
            .filter_map(|plugin| plugin.normalized_id())
            .filter(|plugin| plugin_seen.insert(plugin.clone()))
            .collect::<Vec<_>>();
        if plugins.is_empty() {
            continue;
        }

        let repo_root = project.root.as_deref().and_then(sanitize_path_like);
        let url = project
            .url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if repo_root.is_none() && url.is_none() {
            continue;
        }
        let git_ref = project
            .git_ref
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let refresh = project
            .refresh
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        repo_projects.push(UiRepoProjectConfig {
            id,
            root: repo_root,
            url,
            git_ref,
            refresh,
            plugins,
        });
    }

    UiConfig {
        projects: sanitize_projects(projects),
        repo_projects: sanitize_repo_projects(repo_projects),
    }
}
