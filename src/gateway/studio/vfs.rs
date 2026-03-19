//! VFS (Virtual File System) operations for the studio API.

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{collections::HashSet, fs};

use super::pathing;
use super::router::StudioState;
use super::types::{
    UiProjectConfig, VfsCategory, VfsContentResponse, VfsEntry, VfsScanEntry, VfsScanResult,
};

#[derive(Debug, Clone)]
struct ResolvedVfsRoot {
    request_root: String,
    display_name: String,
    filesystem_path: PathBuf,
    project_name: Option<String>,
    root_label: Option<String>,
    project_root: Option<String>,
    project_dirs: Vec<String>,
    filter_prefix: String,
    file_filters: Vec<pathing::ProjectFileFilter>,
}

#[derive(Debug, Clone)]
struct ResolvedVfsPath {
    full_path: PathBuf,
    root: ResolvedVfsRoot,
    rest: String,
}

struct ScanDirectoryConfig<'a> {
    file_filters: &'a [pathing::ProjectFileFilter],
    project_name: Option<&'a str>,
    root_label: Option<&'a str>,
    project_root: Option<&'a str>,
    project_dirs: &'a [String],
}

struct ScanDirectoryCounters<'a> {
    entries: &'a mut Vec<VfsScanEntry>,
    file_count: &'a mut usize,
    dir_count: &'a mut usize,
}

/// VFS operation error type.
#[derive(Debug)]
pub(crate) enum VfsError {
    Io(io::Error),
    NotFound(String),
    UnknownRoot(String),
}

impl std::fmt::Display for VfsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VfsError::Io(e) => write!(f, "IO error: {e}"),
            VfsError::NotFound(path) => write!(f, "Path not found: {path}"),
            VfsError::UnknownRoot(root) => write!(f, "Unknown VFS root: {root}"),
        }
    }
}

impl std::error::Error for VfsError {}

impl From<io::Error> for VfsError {
    fn from(e: io::Error) -> Self {
        VfsError::Io(e)
    }
}

/// List root entries for the VFS.
pub(crate) fn list_root_entries(state: &StudioState) -> Vec<VfsEntry> {
    resolved_vfs_roots(state)
        .into_iter()
        .map(|root| VfsEntry {
            path: root.request_root,
            name: root.display_name,
            is_dir: true,
            size: 0,
            modified: 0,
            content_type: None,
            project_name: root.project_name,
            root_label: root.root_label,
            project_root: root.project_root,
            project_dirs: (!root.project_dirs.is_empty()).then_some(root.project_dirs),
        })
        .collect()
}

/// Scan all VFS roots and return a summary.
pub(crate) fn scan_roots(state: &StudioState) -> VfsScanResult {
    let start = Instant::now();
    let mut entries = Vec::new();
    let mut file_count = 0;
    let mut dir_count = 0;

    for root in resolved_vfs_roots(state) {
        dir_count += 1;
        let modified = fs::metadata(&root.filesystem_path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|timestamp| timestamp.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_secs());
        entries.push(VfsScanEntry {
            path: root.request_root.clone(),
            name: root.display_name.clone(),
            is_dir: true,
            category: VfsCategory::Folder,
            size: 0,
            modified,
            content_type: None,
            has_frontmatter: false,
            wendao_id: None,
            project_name: root.project_name.clone(),
            root_label: root.root_label.clone(),
            project_root: root.project_root.clone(),
            project_dirs: (!root.project_dirs.is_empty()).then(|| root.project_dirs.clone()),
        });
        let config = ScanDirectoryConfig {
            file_filters: root.file_filters.as_slice(),
            project_name: root.project_name.as_deref(),
            root_label: root.root_label.as_deref(),
            project_root: root.project_root.as_deref(),
            project_dirs: root.project_dirs.as_slice(),
        };
        let mut counters = ScanDirectoryCounters {
            entries: &mut entries,
            file_count: &mut file_count,
            dir_count: &mut dir_count,
        };
        scan_directory(
            root.filesystem_path.as_path(),
            root.request_root.as_str(),
            root.filter_prefix.as_str(),
            &config,
            &mut counters,
        );
    }

    VfsScanResult {
        entries,
        file_count,
        dir_count,
        scan_duration_ms: elapsed_millis_u64(start.elapsed()),
    }
}

/// Get a single VFS entry by path.
pub(crate) fn get_entry(state: &StudioState, path: &str) -> Result<VfsEntry, VfsError> {
    let resolved = resolve_vfs_path(state, path)?;
    let full_path = resolved.full_path;

    if !full_path.exists() {
        return Err(VfsError::NotFound(path.to_string()));
    }

    let metadata = std::fs::metadata(&full_path)?;
    if metadata.is_file() {
        let filter_path =
            join_filter_path(resolved.root.filter_prefix.as_str(), resolved.rest.as_str());
        if !matches_file_filters(filter_path.as_str(), resolved.root.file_filters.as_slice()) {
            return Err(VfsError::NotFound(path.to_string()));
        }
    }
    let is_dir = metadata.is_dir();
    let name = full_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(VfsEntry {
        path: path.to_string(),
        name,
        is_dir,
        size: metadata.len(),
        modified: metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs()),
        content_type: if is_dir {
            None
        } else {
            Some(guess_content_type(&full_path))
        },
        project_name: resolved.root.project_name.clone(),
        root_label: resolved.root.root_label.clone(),
        project_root: resolved.root.project_root.clone(),
        project_dirs: (!resolved.root.project_dirs.is_empty())
            .then(|| resolved.root.project_dirs.clone()),
    })
}

/// Read file content from VFS.
pub(crate) async fn read_content(
    state: &StudioState,
    path: &str,
) -> Result<VfsContentResponse, VfsError> {
    let resolved = resolve_vfs_path(state, path)?;
    let full_path = resolved.full_path;

    if !full_path.exists() {
        return Err(VfsError::NotFound(path.to_string()));
    }
    let metadata = std::fs::metadata(&full_path)?;
    if metadata.is_file() {
        let filter_path =
            join_filter_path(resolved.root.filter_prefix.as_str(), resolved.rest.as_str());
        if !matches_file_filters(filter_path.as_str(), resolved.root.file_filters.as_slice()) {
            return Err(VfsError::NotFound(path.to_string()));
        }
    }

    let content = tokio::fs::read_to_string(&full_path)
        .await
        .map_err(VfsError::Io)?;
    let content_type = guess_content_type(&full_path);

    Ok(VfsContentResponse {
        path: path.to_string(),
        content,
        content_type,
    })
}

fn scan_directory(
    base: &Path,
    prefix: &str,
    filter_prefix: &str,
    config: &ScanDirectoryConfig<'_>,
    counters: &mut ScanDirectoryCounters<'_>,
) {
    if let Ok(dir_entries) = std::fs::read_dir(base) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            let relative = format!("{}/{}", prefix, entry.file_name().to_string_lossy());
            let filter_relative =
                join_filter_path(filter_prefix, entry.file_name().to_string_lossy().as_ref());
            let metadata = entry.metadata().ok();

            if path.is_dir() {
                *counters.dir_count += 1;
                counters.entries.push(VfsScanEntry {
                    path: relative.clone(),
                    name: entry.file_name().to_string_lossy().to_string(),
                    is_dir: true,
                    category: VfsCategory::Folder,
                    size: 0,
                    modified: metadata
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_secs()),
                    content_type: None,
                    has_frontmatter: false,
                    wendao_id: None,
                    project_name: config.project_name.map(ToOwned::to_owned),
                    root_label: config.root_label.map(ToOwned::to_owned),
                    project_root: config.project_root.map(ToOwned::to_owned),
                    project_dirs: (!config.project_dirs.is_empty())
                        .then(|| config.project_dirs.to_vec()),
                });
                scan_directory(&path, &relative, filter_relative.as_str(), config, counters);
            } else {
                if !matches_file_filters(filter_relative.as_str(), config.file_filters) {
                    continue;
                }
                *counters.file_count += 1;
                let has_frontmatter = is_markdown_with_frontmatter(&path);
                counters.entries.push(VfsScanEntry {
                    path: relative,
                    name: entry.file_name().to_string_lossy().to_string(),
                    is_dir: false,
                    category: classify_file_category(prefix, &path),
                    size: metadata.as_ref().map_or(0, std::fs::Metadata::len),
                    modified: metadata
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_secs()),
                    content_type: Some(guess_content_type(&path)),
                    has_frontmatter,
                    wendao_id: None,
                    project_name: config.project_name.map(ToOwned::to_owned),
                    root_label: config.root_label.map(ToOwned::to_owned),
                    project_root: config.project_root.map(ToOwned::to_owned),
                    project_dirs: (!config.project_dirs.is_empty())
                        .then(|| config.project_dirs.to_vec()),
                });
            }
        }
    }
}

fn resolved_vfs_roots(state: &StudioState) -> Vec<ResolvedVfsRoot> {
    let mut roots = Vec::new();
    let mut seen_fs_paths = HashSet::new();

    for project in state.configured_projects() {
        let file_filters = project_file_filters(&project);
        for configured in &project.dirs {
            push_root(
                &mut roots,
                &mut seen_fs_paths,
                resolve_project_root_candidate(
                    state,
                    &project,
                    configured.as_str(),
                    file_filters.clone(),
                ),
            );
        }
    }

    assign_request_roots(&mut roots);
    roots
}

pub(crate) fn graph_lookup_candidates(state: &StudioState, requested_path: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let normalized_requested_path = requested_path.trim().replace('\\', "/");
    push_unique_candidate(&mut candidates, normalized_requested_path.clone());

    if let Some(stripped_dollar) = normalized_requested_path.strip_prefix('$') {
        push_unique_candidate(&mut candidates, stripped_dollar.to_string());
    }

    let semantic_candidates = semantic_lookup_candidates(normalized_requested_path.as_str());
    for candidate in semantic_candidates {
        push_unique_candidate(&mut candidates, candidate);
    }

    let path_candidates = candidates.clone();
    for candidate in path_candidates {
        if let Ok(resolved) = resolve_vfs_path(state, candidate.as_str()) {
            let full_path = resolved.full_path;
            push_unique_candidate(
                &mut candidates,
                normalize_graph_index_path(state, full_path.as_path()),
            );
            push_unique_candidate(&mut candidates, normalize_path_string(full_path.as_path()));
        }
    }

    candidates
}

pub(crate) fn resolve_navigation_target(
    state: &StudioState,
    requested_path: &str,
) -> super::types::StudioNavigationTarget {
    let candidates = graph_lookup_candidates(state, requested_path);

    for candidate in &candidates {
        if let Ok(entry) = get_entry(state, candidate.as_str()) {
            let (project_name, root_label) =
                navigation_target_project_metadata(state, entry.path.as_str());
            return super::types::StudioNavigationTarget {
                path: entry.path.clone(),
                category: inferred_navigation_category(entry.path.as_str(), entry.is_dir),
                project_name: entry.project_name.or(project_name),
                root_label: entry.root_label.or(root_label),
                line: None,
                line_end: None,
                column: None,
            };
        }
    }

    let fallback_path = candidates
        .iter()
        .find(|candidate| !has_semantic_prefix(candidate.as_str()))
        .cloned()
        .unwrap_or_else(|| requested_path.trim().replace('\\', "/"));
    let (project_name, root_label) =
        navigation_target_project_metadata(state, fallback_path.as_str());

    super::types::StudioNavigationTarget {
        path: fallback_path.clone(),
        category: inferred_navigation_category(fallback_path.as_str(), false),
        project_name,
        root_label,
        line: None,
        line_end: None,
        column: None,
    }
}

fn navigation_target_project_metadata(
    state: &StudioState,
    path: &str,
) -> (Option<String>, Option<String>) {
    resolved_vfs_roots(state)
        .into_iter()
        .filter_map(|root| {
            if path == root.request_root {
                return Some((root.request_root.len(), root.project_name, root.root_label));
            }

            path.strip_prefix(root.request_root.as_str())
                .filter(|suffix| suffix.starts_with('/'))
                .map(|_| (root.request_root.len(), root.project_name, root.root_label))
        })
        .max_by_key(|(request_root_len, _, _)| *request_root_len)
        .map(|(_, project_name, root_label)| (project_name, root_label))
        .unwrap_or((None, None))
}

fn semantic_lookup_candidates(requested_path: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let without_dollar = requested_path.strip_prefix('$').unwrap_or(requested_path);
    if let Some(rest) = strip_ascii_prefix_case_insensitive(without_dollar, "wendao://") {
        let normalized = rest.trim_start_matches('/').to_string();
        push_unique_candidate(&mut candidates, normalized);
    }
    if let Some(rest) = strip_ascii_prefix_case_insensitive(without_dollar, "id:") {
        let normalized = rest.trim_start_matches('/').to_string();
        push_unique_candidate(&mut candidates, normalized);
    }
    candidates
}

fn strip_ascii_prefix_case_insensitive<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
        .map(|_| &value[prefix.len()..])
}

fn has_semantic_prefix(value: &str) -> bool {
    let trimmed = value.trim_start_matches('$');
    trimmed
        .get(..9)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("wendao://"))
        || trimmed
            .get(..3)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("id:"))
}

fn inferred_navigation_category(path: &str, is_dir: bool) -> String {
    if is_dir {
        return "folder".to_string();
    }
    if path.starts_with("knowledge/") {
        return "knowledge".to_string();
    }
    if path.ends_with("SKILL.md") {
        return "skill".to_string();
    }
    "doc".to_string()
}

pub(crate) fn studio_display_path(state: &StudioState, graph_path: &str) -> String {
    let full_path = graph_path_to_filesystem_path(state, graph_path);
    let mut best_match: Option<(usize, String)> = None;

    for root in resolved_vfs_roots(state) {
        let Ok(rest) = full_path.strip_prefix(&root.filesystem_path) else {
            continue;
        };
        let rest = normalize_relative_path(rest);
        let candidate = if rest.is_empty() {
            root.request_root
        } else {
            format!("{}/{}", root.request_root, rest)
        };
        let depth = root.filesystem_path.components().count();
        match &best_match {
            Some((best_depth, _)) if *best_depth >= depth => {}
            _ => best_match = Some((depth, candidate)),
        }
    }

    best_match.map_or_else(|| graph_path.replace('\\', "/"), |(_, path)| path)
}

fn push_root(
    roots: &mut Vec<ResolvedVfsRoot>,
    seen_fs_paths: &mut HashSet<String>,
    candidate: Option<ResolvedVfsRoot>,
) {
    let Some(candidate) = candidate else {
        return;
    };
    if !candidate.filesystem_path.exists() {
        return;
    }

    let normalized_fs_path = candidate
        .filesystem_path
        .to_string_lossy()
        .replace('\\', "/");
    if !seen_fs_paths.insert(normalized_fs_path) {
        return;
    }

    roots.push(candidate);
}

fn assign_request_roots(roots: &mut [ResolvedVfsRoot]) {
    let mut seen_labels = HashSet::new();
    let mut duplicate_labels = HashSet::new();

    for root in roots.iter() {
        if !seen_labels.insert(root.request_root.clone()) {
            duplicate_labels.insert(root.request_root.clone());
        }
    }

    let mut seen_request_roots = HashSet::new();
    for root in roots.iter_mut() {
        if duplicate_labels.contains(root.request_root.as_str())
            && let (Some(project_name), Some(root_label)) =
                (root.project_name.as_deref(), root.root_label.as_deref())
        {
            root.request_root = scoped_request_root(project_name, root_label);
        }

        if !seen_request_roots.insert(root.request_root.clone()) {
            let mut suffix = 2usize;
            loop {
                let alternative = format!("{}-{suffix}", root.request_root);
                if seen_request_roots.insert(alternative.clone()) {
                    root.request_root = alternative;
                    break;
                }
                suffix += 1;
            }
        }
    }
}

fn resolve_project_root_candidate(
    state: &StudioState,
    project: &UiProjectConfig,
    raw: &str,
    file_filters: Vec<pathing::ProjectFileFilter>,
) -> Option<ResolvedVfsRoot> {
    let normalized = normalize_configured_root(raw)?;
    let filesystem_path =
        resolve_project_filesystem_root(state, project.root.as_str(), normalized.as_str())?;
    let root_label = configured_root_label(
        normalized.as_str(),
        filesystem_path.as_path(),
        project.name.as_str(),
    )?;
    let filter_prefix = if normalized == "." {
        String::new()
    } else {
        normalized.clone()
    };
    Some(ResolvedVfsRoot {
        display_name: root_label.clone(),
        request_root: root_label.clone(),
        filesystem_path,
        project_name: Some(project.name.clone()),
        root_label: Some(root_label),
        project_root: Some(project.root.clone()),
        project_dirs: project.dirs.clone(),
        filter_prefix,
        file_filters,
    })
}

fn normalize_configured_root(raw: &str) -> Option<String> {
    pathing::normalize_project_dir_root(raw)
}

fn resolve_project_filesystem_root(
    state: &StudioState,
    root: &str,
    normalized: &str,
) -> Option<PathBuf> {
    let project_root = pathing::resolve_path_like(state.config_root.as_path(), root)?;
    pathing::resolve_path_like(project_root.as_path(), normalized)
}

fn configured_root_label(
    normalized: &str,
    filesystem_path: &Path,
    project_name: &str,
) -> Option<String> {
    if normalized == "." {
        return Some(project_name.to_string());
    }
    root_leaf_label(normalized, filesystem_path).or_else(|| Some(project_name.to_string()))
}

fn root_leaf_label(normalized: &str, filesystem_path: &Path) -> Option<String> {
    filesystem_path.file_name().map_or_else(
        || {
            normalized
                .rsplit('/')
                .find(|segment| !segment.trim().is_empty())
                .map(ToOwned::to_owned)
        },
        |component| Some(component.to_string_lossy().to_string()),
    )
}

fn scoped_request_root(project_name: &str, root_label: &str) -> String {
    if root_label == project_name {
        project_name.to_string()
    } else {
        format!("{project_name}/{root_label}")
    }
}

fn normalize_graph_index_path(state: &StudioState, full_path: &Path) -> String {
    full_path.strip_prefix(&state.project_root).map_or_else(
        |_| normalize_path_string(full_path),
        normalize_relative_path,
    )
}

fn graph_path_to_filesystem_path(state: &StudioState, graph_path: &str) -> PathBuf {
    let candidate = Path::new(graph_path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        state.project_root.join(candidate)
    }
}

fn normalize_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalize_relative_path(path: &Path) -> String {
    normalize_path_string(path)
        .trim_start_matches("./")
        .to_string()
}

fn push_unique_candidate(candidates: &mut Vec<String>, candidate: String) {
    if candidate.is_empty() || candidates.iter().any(|existing| existing == &candidate) {
        return;
    }
    candidates.push(candidate);
}

fn resolve_vfs_path(
    state: &StudioState,
    requested_path: &str,
) -> Result<ResolvedVfsPath, VfsError> {
    let normalized_request = requested_path.trim().replace('\\', "/");
    let Some((candidate, normalized_rest)) = resolved_vfs_roots(state)
        .into_iter()
        .filter_map(|candidate| {
            if normalized_request == candidate.request_root {
                return Some((candidate, String::new()));
            }

            normalized_request
                .strip_prefix(candidate.request_root.as_str())
                .filter(|suffix| suffix.starts_with('/'))
                .map(|suffix| (candidate, suffix.trim_start_matches('/').to_string()))
        })
        .max_by_key(|(candidate, _)| candidate.request_root.len())
    else {
        let unknown_root = normalized_request
            .split('/')
            .next()
            .unwrap_or(normalized_request.as_str())
            .to_string();
        return Err(VfsError::UnknownRoot(unknown_root));
    };
    let full_path = if normalized_rest.is_empty() {
        candidate.filesystem_path.clone()
    } else {
        candidate.filesystem_path.join(normalized_rest.as_str())
    };
    Ok(ResolvedVfsPath {
        full_path,
        root: candidate,
        rest: normalized_rest,
    })
}

fn project_file_filters(project: &UiProjectConfig) -> Vec<pathing::ProjectFileFilter> {
    project
        .dirs
        .iter()
        .filter_map(|entry| pathing::compile_project_dir_filter(entry.as_str()))
        .collect()
}

fn join_filter_path(prefix: &str, path: &str) -> String {
    let normalized_path = path.trim_start_matches('/').replace('\\', "/");
    if prefix.is_empty() || prefix == "." {
        normalized_path
    } else if normalized_path.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{normalized_path}")
    }
}

fn matches_file_filters(path: &str, filters: &[pathing::ProjectFileFilter]) -> bool {
    pathing::matches_project_file_filters(path, filters)
}

fn classify_file_category(root: &str, path: &Path) -> VfsCategory {
    let root_leaf = root.rsplit('/').next().unwrap_or(root);
    if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
        || root_leaf.contains("skill")
    {
        VfsCategory::Skill
    } else if root_leaf == "knowledge" {
        VfsCategory::Knowledge
    } else if matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md" | "markdown" | "bpmn")
    ) {
        VfsCategory::Doc
    } else {
        VfsCategory::Other
    }
}

fn is_markdown_with_frontmatter(path: &Path) -> bool {
    if path.extension().and_then(|e| e.to_str()) != Some("md") {
        return false;
    }
    if let Ok(content) = std::fs::read_to_string(path) {
        content.starts_with("---\n")
    } else {
        false
    }
}

fn elapsed_millis_u64(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

fn guess_content_type(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("md") => "text/markdown".to_string(),
        Some("py") => "text/x-python".to_string(),
        Some("rs") => "text/x-rust".to_string(),
        Some("toml") => "application/toml".to_string(),
        Some("json") => "application/json".to_string(),
        Some("yaml" | "yml") => "application/yaml".to_string(),
        _ => "text/plain".to_string(),
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/gateway/studio/vfs.rs"]
mod tests;
