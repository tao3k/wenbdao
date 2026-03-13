//! VFS (Virtual File System) operations for the studio API.

use std::io;
use std::path::Path;
use std::time::Instant;

use super::router::StudioState;
use super::types::{VfsCategory, VfsContentResponse, VfsEntry, VfsScanEntry, VfsScanResult};

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
pub(crate) fn list_root_entries(state: &StudioState) -> Result<Vec<VfsEntry>, VfsError> {
    let mut entries = Vec::new();

    // Add skills root
    let skills_root = &state.internal_skill_root;
    if skills_root.exists() {
        entries.push(VfsEntry {
            path: "skills".to_string(),
            name: "skills".to_string(),
            is_dir: true,
            size: 0,
            modified: 0,
            content_type: None,
        });
    }

    // Add knowledge root
    let knowledge_root = &state.knowledge_root;
    if knowledge_root.exists() {
        entries.push(VfsEntry {
            path: "knowledge".to_string(),
            name: "knowledge".to_string(),
            is_dir: true,
            size: 0,
            modified: 0,
            content_type: None,
        });
    }

    Ok(entries)
}

/// Scan all VFS roots and return a summary.
pub(crate) fn scan_roots(state: &StudioState) -> Result<VfsScanResult, VfsError> {
    let start = Instant::now();
    let mut entries = Vec::new();
    let mut file_count = 0;
    let mut dir_count = 0;

    // Scan skills root
    let skills_root = &state.internal_skill_root;
    if skills_root.exists() {
        scan_directory(
            skills_root,
            "skills",
            &mut entries,
            &mut file_count,
            &mut dir_count,
        );
    }

    // Scan knowledge root
    let knowledge_root = &state.knowledge_root;
    if knowledge_root.exists() {
        scan_directory(
            knowledge_root,
            "knowledge",
            &mut entries,
            &mut file_count,
            &mut dir_count,
        );
    }

    Ok(VfsScanResult {
        entries,
        file_count,
        dir_count,
        scan_duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Get a single VFS entry by path.
pub(crate) fn get_entry(state: &StudioState, path: &str) -> Result<VfsEntry, VfsError> {
    let (root, rest) = path.split_once('/').unwrap_or((path, ""));

    let full_path = match root {
        "skills" => state.internal_skill_root.join(rest),
        "knowledge" => state.knowledge_root.join(rest),
        _ => return Err(VfsError::UnknownRoot(root.to_string())),
    };

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
            .map(|d| d.as_secs())
            .unwrap_or(0),
        content_type: if !is_dir {
            guess_content_type(&full_path)
        } else {
            None
        },
    })
}

/// Read file content from VFS.
pub(crate) async fn read_content(
    state: &StudioState,
    path: &str,
) -> Result<VfsContentResponse, VfsError> {
    let (root, rest) = path.split_once('/').unwrap_or((path, ""));

    let full_path = match root {
        "skills" => state.internal_skill_root.join(rest),
        "knowledge" => state.knowledge_root.join(rest),
        _ => return Err(VfsError::UnknownRoot(root.to_string())),
    };

    if !full_path.exists() {
        return Err(VfsError::NotFound(path.to_string()));
    }

    let content = tokio::fs::read_to_string(&full_path)
        .await
        .map_err(VfsError::Io)?;
    let content_type = guess_content_type(&full_path).unwrap_or_else(|| "text/plain".to_string());

    Ok(VfsContentResponse {
        path: path.to_string(),
        content,
        content_type,
    })
}

fn scan_directory(
    base: &Path,
    prefix: &str,
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
                    category: if prefix == "skills" {
                        VfsCategory::Skill
                    } else {
                        VfsCategory::Knowledge
                    },
                    size: 0,
                    modified: metadata
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    content_type: None,
                    has_frontmatter: false,
                    wendao_id: None,
                });
                scan_directory(&path, &relative, entries, file_count, dir_count);
            } else {
                *file_count += 1;
                let has_frontmatter = is_markdown_with_frontmatter(&path);
                entries.push(VfsScanEntry {
                    path: relative,
                    name: entry.file_name().to_string_lossy().to_string(),
                    is_dir: false,
                    category: if prefix == "skills" {
                        VfsCategory::Skill
                    } else {
                        VfsCategory::Knowledge
                    },
                    size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                    modified: metadata
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    content_type: guess_content_type(&path),
                    has_frontmatter,
                    wendao_id: None,
                });
            }
        }
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

fn guess_content_type(path: &Path) -> Option<String> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("md") => Some("text/markdown".to_string()),
        Some("py") => Some("text/x-python".to_string()),
        Some("rs") => Some("text/x-rust".to_string()),
        Some("toml") => Some("application/toml".to_string()),
        Some("json") => Some("application/json".to_string()),
        Some("yaml") | Some("yml") => Some("application/yaml".to_string()),
        _ => Some("text/plain".to_string()),
    }
}
