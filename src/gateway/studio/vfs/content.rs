use std::fs;
use std::path::PathBuf;

use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::types::{VfsContentResponse, VfsEntry};

use super::filters::VfsError;
use super::roots::resolve_all_vfs_roots;

pub(crate) fn get_entry(state: &StudioState, path: &str) -> Result<VfsEntry, VfsError> {
    let resolved = resolve_vfs_path(state, path)?;
    let metadata = fs::metadata(&resolved.full_path)
        .map_err(|error| VfsError::internal("IO_ERROR", error.to_string(), None))?;

    Ok(VfsEntry {
        path: path.to_string(),
        name: resolved
            .full_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        is_dir: metadata.is_dir(),
        size: metadata.len(),
        modified: unix_timestamp_secs(&metadata),
        content_type: None,
        project_name: None,
        root_label: None,
        project_root: None,
        project_dirs: None,
    })
}

#[allow(clippy::unused_async)]
pub(crate) async fn read_content(
    state: &StudioState,
    path: &str,
) -> Result<VfsContentResponse, VfsError> {
    let resolved = resolve_vfs_path(state, path)?;
    let content = fs::read_to_string(&resolved.full_path)
        .map_err(|error| VfsError::internal("IO_ERROR", error.to_string(), None))?;
    let metadata = fs::metadata(&resolved.full_path)
        .map_err(|error| VfsError::internal("IO_ERROR", error.to_string(), None))?;

    Ok(VfsContentResponse {
        path: path.to_string(),
        content_type: "text/plain".to_string(),
        content,
        modified: unix_timestamp_secs(&metadata),
    })
}

pub(super) struct ResolvedVfsPath {
    pub(super) full_path: PathBuf,
}

pub(crate) fn resolve_vfs_file_path(state: &StudioState, path: &str) -> Result<PathBuf, VfsError> {
    resolve_vfs_path(state, path).map(|resolved| resolved.full_path)
}

pub(super) fn resolve_vfs_path(
    state: &StudioState,
    path: &str,
) -> Result<ResolvedVfsPath, VfsError> {
    for root in resolve_all_vfs_roots(state) {
        if path == root.request_root {
            return Ok(ResolvedVfsPath {
                full_path: root.full_path,
            });
        }
        let prefix = format!("{}/", root.request_root);
        if path.starts_with(&prefix) {
            let rel = &path[prefix.len()..];
            return Ok(ResolvedVfsPath {
                full_path: root.full_path.join(rel),
            });
        }
    }
    Err(VfsError::not_found(format!("VFS path not found: {path}")))
}

pub(super) fn unix_timestamp_secs(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs())
}
