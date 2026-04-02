use std::collections::HashSet;

use crate::gateway::studio::pathing;
use crate::gateway::studio::types::{UiProjectConfig, UiRepoProjectConfig};

use super::types::WendaoTomlPluginEntry;

pub(crate) fn sanitize_projects(raw: Vec<UiProjectConfig>) -> Vec<UiProjectConfig> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::new();
    for project in raw {
        let name = project.name.trim();
        if name.is_empty() {
            continue;
        }
        if !seen.insert(name.to_string()) {
            continue;
        }

        let Some(root) = sanitize_path_like(project.root.as_str()) else {
            continue;
        };

        out.push(UiProjectConfig {
            name: name.to_string(),
            root,
            dirs: sanitize_path_list(&project.dirs),
        });
    }
    out
}

pub(crate) fn sanitize_path_list(raw: &[String]) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::new();
    for path in raw {
        let Some(normalized) = pathing::normalize_project_dir_root(path.as_str()) else {
            continue;
        };
        if seen.insert(normalized.clone()) {
            out.push(normalized);
        }
    }
    out
}

pub(crate) fn sanitize_path_like(raw: &str) -> Option<String> {
    pathing::normalize_path_like(raw)
}

pub(crate) fn merge_repo_plugins(
    existing: Vec<WendaoTomlPluginEntry>,
    plugin_ids: &[String],
) -> Vec<WendaoTomlPluginEntry> {
    let desired = sanitize_plugin_ids(plugin_ids);
    if desired.is_empty() {
        return Vec::new();
    }

    let allowed = desired.iter().cloned().collect::<HashSet<_>>();
    let mut preserved = Vec::new();
    let mut seen = HashSet::<String>::new();
    for entry in existing
        .into_iter()
        .filter_map(WendaoTomlPluginEntry::into_normalized)
    {
        let Some(id) = entry.normalized_id() else {
            continue;
        };
        if allowed.contains(id.as_str()) {
            seen.insert(id);
            preserved.push(entry);
        }
    }

    for plugin in desired {
        if seen.insert(plugin.clone()) {
            preserved.push(WendaoTomlPluginEntry::Id(plugin));
        }
    }

    preserved
}

pub(crate) fn sanitize_plugin_ids(raw: &[String]) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    raw.iter()
        .filter_map(|plugin| normalize_plugin_id(plugin))
        .filter(|plugin| seen.insert(plugin.clone()))
        .collect()
}

pub(crate) fn sanitize_repo_projects(raw: Vec<UiRepoProjectConfig>) -> Vec<UiRepoProjectConfig> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::new();
    for project in raw {
        let id = project.id.trim();
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        let root = project
            .root
            .as_deref()
            .and_then(sanitize_path_like)
            .filter(|value| !value.is_empty());
        let url = project
            .url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
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
        let mut plugin_seen = HashSet::<String>::new();
        let plugins = project
            .plugins
            .into_iter()
            .filter_map(|plugin| normalize_plugin_id(plugin.as_str()))
            .filter(|plugin| plugin_seen.insert(plugin.clone()))
            .collect::<Vec<_>>();
        if plugins.is_empty() {
            continue;
        }
        if root.is_none() && url.is_none() {
            continue;
        }
        out.push(UiRepoProjectConfig {
            id: id.to_string(),
            root,
            url,
            git_ref,
            refresh,
            plugins,
        });
    }
    out
}

fn normalize_plugin_id(raw: &str) -> Option<String> {
    let plugin = raw.trim();
    if plugin.is_empty() {
        None
    } else {
        Some(plugin.to_string())
    }
}
