use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::gateway::studio::types::UiProjectConfig;

#[derive(Debug, Clone, Default)]
pub(super) struct SearchProjectMetadata {
    pub(super) project_name: Option<String>,
    pub(super) root_label: Option<String>,
}

pub(super) fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(super) fn normalize_config_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed == "." {
        return Some(".".to_string());
    }

    let normalized = trimmed
        .replace('\\', "/")
        .trim_end_matches('/')
        .trim_start_matches("./")
        .to_string();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub(super) fn configured_project_scan_roots(
    project_root: &Path,
    projects: &[UiProjectConfig],
) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut seen = HashSet::new();

    for project in projects {
        for configured_path in &project.paths {
            let Some(scope_path) = resolve_project_scope_path(
                project_root,
                project.root.as_str(),
                configured_path.as_str(),
            ) else {
                continue;
            };
            if !scope_path.exists() {
                continue;
            }
            let normalized = normalize_path(scope_path.as_path());
            if seen.insert(normalized) {
                roots.push(scope_path);
            }
        }
    }

    roots
}

pub(super) fn resolve_project_root_path(
    project_root: &Path,
    configured_root: &str,
) -> Option<PathBuf> {
    let normalized_root = normalize_config_path(configured_root)?;
    if Path::new(normalized_root.as_str()).is_absolute() {
        Some(PathBuf::from(normalized_root))
    } else if normalized_root == "." {
        Some(project_root.to_path_buf())
    } else {
        Some(project_root.join(normalized_root))
    }
}

pub(super) fn resolve_project_scope_path(
    project_root: &Path,
    configured_root: &str,
    configured_path: &str,
) -> Option<PathBuf> {
    let project_base = resolve_project_root_path(project_root, configured_root)?;
    let normalized_path = normalize_config_path(configured_path)?;
    if Path::new(normalized_path.as_str()).is_absolute() {
        Some(PathBuf::from(normalized_path))
    } else if normalized_path == "." {
        Some(project_base)
    } else {
        Some(project_base.join(normalized_path))
    }
}

pub(super) fn index_path_for_entry(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .map_or_else(|_| normalize_path(path), normalize_path)
}

pub(super) fn project_metadata_for_path(
    project_root: &Path,
    projects: &[UiProjectConfig],
    hit_path: &str,
) -> SearchProjectMetadata {
    let absolute_hit = if Path::new(hit_path).is_absolute() {
        PathBuf::from(hit_path)
    } else {
        project_root.join(hit_path)
    };
    let mut best_path_match: Option<(usize, SearchProjectMetadata)> = None;
    let mut best_root_match: Option<(usize, SearchProjectMetadata)> = None;

    for project in projects {
        let Some(project_root_path) =
            resolve_project_root_path(project_root, project.root.as_str())
        else {
            continue;
        };

        if !path_within_scope(absolute_hit.as_path(), project_root_path.as_path()) {
            continue;
        }

        update_best_match(
            &mut best_root_match,
            path_specificity(normalize_path(project_root_path.as_path()).as_str()),
            SearchProjectMetadata {
                project_name: Some(project.name.clone()),
                root_label: None,
            },
        );

        for configured_path in &project.paths {
            let Some(normalized_path) = normalize_config_path(configured_path.as_str()) else {
                continue;
            };
            let Some(candidate_scope) = resolve_project_scope_path(
                project_root,
                project.root.as_str(),
                normalized_path.as_str(),
            ) else {
                continue;
            };
            if !path_within_scope(absolute_hit.as_path(), candidate_scope.as_path()) {
                continue;
            }

            update_best_match(
                &mut best_path_match,
                path_specificity(normalize_path(candidate_scope.as_path()).as_str()),
                SearchProjectMetadata {
                    project_name: Some(project.name.clone()),
                    root_label: configured_root_label(
                        normalized_path.as_str(),
                        project.name.as_str(),
                    ),
                },
            );
        }
    }

    best_path_match
        .map(|(_, metadata)| metadata)
        .or_else(|| best_root_match.map(|(_, metadata)| metadata))
        .unwrap_or_default()
}

fn configured_root_label(configured_path: &str, project_name: &str) -> Option<String> {
    if configured_path == "." {
        return Some(project_name.to_string());
    }

    Path::new(configured_path)
        .file_name()
        .map(|segment| segment.to_string_lossy().into_owned())
        .or_else(|| Some(project_name.to_string()))
}

fn path_within_scope(path: &Path, scope: &Path) -> bool {
    path == scope || path.strip_prefix(scope).is_ok()
}

fn path_specificity(path: &str) -> usize {
    if path == "." {
        0
    } else {
        path.split('/').count()
    }
}

fn update_best_match(
    slot: &mut Option<(usize, SearchProjectMetadata)>,
    specificity: usize,
    metadata: SearchProjectMetadata,
) {
    match slot {
        Some((current_specificity, _)) if *current_specificity >= specificity => {}
        _ => *slot = Some((specificity, metadata)),
    }
}
