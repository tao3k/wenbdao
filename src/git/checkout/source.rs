use std::path::Path;

use git2::Repository;

use super::{
    RepositoryLifecycleState, RepositorySyncMode, ResolvedRepositorySource,
    ResolvedRepositorySourceKind,
};
use crate::analyzers::config::RegisteredRepository;
use crate::analyzers::errors::RepoIntelligenceError;

/// Resolves the source for a registered repository.
///
/// # Errors
///
/// Returns an error when the repository has no usable local path or remote
/// source, or when the selected source cannot be resolved.
pub fn resolve_repository_source(
    repository: &RegisteredRepository,
    cwd: &Path,
    mode: RepositorySyncMode,
) -> Result<ResolvedRepositorySource, RepoIntelligenceError> {
    if let Some(path) = repository.path.as_ref() {
        return resolve_local_checkout(repository, cwd, path, mode);
    }

    if repository.url.is_some() {
        return crate::git::checkout::managed::resolve_managed_checkout(repository, mode);
    }

    Err(RepoIntelligenceError::MissingRepositorySource {
        repo_id: repository.id.clone(),
    })
}

fn resolve_local_checkout(
    repository: &RegisteredRepository,
    cwd: &Path,
    path: &Path,
    mode: RepositorySyncMode,
) -> Result<ResolvedRepositorySource, RepoIntelligenceError> {
    let checkout_root = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    if checkout_root.exists() && !checkout_root.is_dir() {
        return Err(RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: repository.id.clone(),
            path: checkout_root.display().to_string(),
            reason: "path exists but is not a directory".to_string(),
        });
    }

    if !checkout_root.exists() && !matches!(mode, RepositorySyncMode::Status) {
        return Err(RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: repository.id.clone(),
            path: checkout_root.display().to_string(),
            reason: "directory does not exist".to_string(),
        });
    }

    let checkout_state = if checkout_root.is_dir() {
        Repository::open(&checkout_root).map_err(|error| {
            RepoIntelligenceError::InvalidRepositoryPath {
                repo_id: repository.id.clone(),
                path: checkout_root.display().to_string(),
                reason: format!("path is not a git checkout: {error}"),
            }
        })?;
        RepositoryLifecycleState::Validated
    } else {
        RepositoryLifecycleState::Missing
    };

    Ok(ResolvedRepositorySource {
        checkout_root,
        mirror_root: None,
        mirror_revision: None,
        tracking_revision: None,
        last_fetched_at: None,
        drift_state: crate::analyzers::query::RepoSyncDriftState::NotApplicable,
        mirror_state: RepositoryLifecycleState::NotApplicable,
        checkout_state,
        source_kind: ResolvedRepositorySourceKind::LocalCheckout,
    })
}
