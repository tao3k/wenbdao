//! VFS (Virtual File System) operations for the studio API.

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{collections::HashSet, fs};

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
        });
        scan_directory(
            root.filesystem_path.as_path(),
            root.request_root.as_str(),
            root.project_name.as_deref(),
            root.root_label.as_deref(),
            &mut entries,
            &mut file_count,
            &mut dir_count,
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
    let full_path = resolve_vfs_path(state, path)?;

    if !full_path.exists() {
        return Err(VfsError::NotFound(path.to_string()));
    }

    let metadata = std::fs::metadata(&full_path)?;
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
        project_name: None,
        root_label: None,
    })
}

/// Read file content from VFS.
pub(crate) async fn read_content(
    state: &StudioState,
    path: &str,
) -> Result<VfsContentResponse, VfsError> {
    let full_path = resolve_vfs_path(state, path)?;

    if !full_path.exists() {
        return Err(VfsError::NotFound(path.to_string()));
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
    project_name: Option<&str>,
    root_label: Option<&str>,
    entries: &mut Vec<VfsScanEntry>,
    file_count: &mut usize,
    dir_count: &mut usize,
) {
    if let Ok(dir_entries) = std::fs::read_dir(base) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            let relative = format!("{}/{}", prefix, entry.file_name().to_string_lossy());
            let metadata = entry.metadata().ok();

            if path.is_dir() {
                *dir_count += 1;
                entries.push(VfsScanEntry {
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
                    project_name: project_name.map(ToOwned::to_owned),
                    root_label: root_label.map(ToOwned::to_owned),
                });
                scan_directory(
                    &path,
                    &relative,
                    project_name,
                    root_label,
                    entries,
                    file_count,
                    dir_count,
                );
            } else {
                *file_count += 1;
                let has_frontmatter = is_markdown_with_frontmatter(&path);
                entries.push(VfsScanEntry {
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
                    project_name: project_name.map(ToOwned::to_owned),
                    root_label: root_label.map(ToOwned::to_owned),
                });
            }
        }
    }
}

fn resolved_vfs_roots(state: &StudioState) -> Vec<ResolvedVfsRoot> {
    let mut roots = Vec::new();
    let mut seen_fs_paths = HashSet::new();
    let mut seen_request_roots = HashSet::new();

    for project in state.configured_projects() {
        for configured in &project.paths {
            push_root(
                &mut roots,
                &mut seen_fs_paths,
                &mut seen_request_roots,
                resolve_project_root_candidate(state, &project, configured.as_str()),
            );
        }
    }

    roots
}

pub(crate) fn graph_lookup_candidates(state: &StudioState, requested_path: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    push_unique_candidate(&mut candidates, requested_path.trim().replace('\\', "/"));

    if let Ok(full_path) = resolve_vfs_path(state, requested_path) {
        push_unique_candidate(
            &mut candidates,
            normalize_graph_index_path(state, full_path.as_path()),
        );
        push_unique_candidate(&mut candidates, normalize_path_string(full_path.as_path()));
    }

    candidates
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
    seen_request_roots: &mut HashSet<String>,
    candidate: Option<ResolvedVfsRoot>,
) {
    let Some(mut candidate) = candidate else {
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

    if !seen_request_roots.insert(candidate.request_root.clone()) {
        let mut suffix = 2usize;
        loop {
            let alternative = format!("{}-{suffix}", candidate.request_root);
            if seen_request_roots.insert(alternative.clone()) {
                candidate.request_root.clone_from(&alternative);
                break;
            }
            suffix += 1;
        }
    }

    roots.push(candidate);
}

fn resolve_project_root_candidate(
    state: &StudioState,
    project: &UiProjectConfig,
    raw: &str,
) -> Option<ResolvedVfsRoot> {
    let normalized = normalize_configured_root(raw)?;
    let filesystem_path =
        resolve_project_filesystem_root(state, project.root.as_str(), normalized.as_str());
    let root_label = configured_root_label(
        normalized.as_str(),
        filesystem_path.as_path(),
        project.name.as_str(),
    )?;
    Some(ResolvedVfsRoot {
        display_name: root_label.clone(),
        request_root: root_label.clone(),
        filesystem_path,
        project_name: Some(project.name.clone()),
        root_label: Some(root_label),
    })
}

fn normalize_configured_root(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
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

fn resolve_project_filesystem_root(state: &StudioState, root: &str, normalized: &str) -> PathBuf {
    let project_root = if Path::new(root).is_absolute() {
        PathBuf::from(root)
    } else if root == "." {
        state.project_root.clone()
    } else {
        state.project_root.join(root)
    };

    let candidate = Path::new(normalized);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else if normalized == "." {
        project_root
    } else {
        project_root.join(normalized)
    }
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

fn resolve_vfs_path(state: &StudioState, requested_path: &str) -> Result<PathBuf, VfsError> {
    let (root, rest) = requested_path
        .split_once('/')
        .unwrap_or((requested_path, ""));
    let full_path = resolved_vfs_roots(state)
        .into_iter()
        .find(|candidate| candidate.request_root == root)
        .map(|candidate| candidate.filesystem_path.join(rest))
        .ok_or_else(|| VfsError::UnknownRoot(root.to_string()))?;
    Ok(full_path)
}

fn classify_file_category(root: &str, path: &Path) -> VfsCategory {
    if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") || root.contains("skill")
    {
        VfsCategory::Skill
    } else if root == "knowledge" {
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
