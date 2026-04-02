use std::fs;

use chrono::Utc;
use git2::Repository;

use crate::analyzers::config::RegisteredRepository;
use crate::analyzers::config::RepositoryRefreshPolicy;
use crate::analyzers::errors::RepoIntelligenceError;
use crate::git::checkout::managed::retry::{
    clone_bare_with_retry, ensure_remote_url, fetch_origin_with_retry, open_bare_with_retry,
    open_checkout_with_retry, should_fetch,
};
use crate::git::checkout::{
    RepositoryLifecycleState, RepositorySyncMode, ResolvedRepositorySource,
    ResolvedRepositorySourceKind, lock, metadata, namespace, refs,
};

#[allow(clippy::too_many_lines)]
pub(crate) fn resolve_managed_checkout(
    repository: &RegisteredRepository,
    mode: RepositorySyncMode,
) -> Result<ResolvedRepositorySource, RepoIntelligenceError> {
    let upstream_url = repository
        .url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| RepoIntelligenceError::MissingRepositorySource {
            repo_id: repository.id.clone(),
        })?;
    let mirror_root = namespace::managed_mirror_root_for(repository);
    let checkout_root = namespace::managed_checkout_root_for(repository);

    if (!mirror_root.exists() || !checkout_root.exists())
        && matches!(mode, RepositorySyncMode::Status)
    {
        return Ok(ResolvedRepositorySource {
            checkout_root,
            mirror_root: Some(mirror_root),
            mirror_revision: None,
            tracking_revision: None,
            last_fetched_at: None,
            drift_state: crate::analyzers::query::RepoSyncDriftState::Unknown,
            mirror_state: RepositoryLifecycleState::Missing,
            checkout_state: RepositoryLifecycleState::Missing,
            source_kind: ResolvedRepositorySourceKind::ManagedRemote,
        });
    }

    let _checkout_lock = (!matches!(mode, RepositorySyncMode::Status))
        .then(|| lock::acquire_managed_checkout_lock(repository))
        .transpose()?;

    if let Some(parent) = mirror_root.parent() {
        fs::create_dir_all::<&std::path::Path>(parent).map_err(|error| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to create managed mirror dir `{}`: {error}",
                    parent.display()
                ),
            }
        })?;
    }
    if let Some(parent) = checkout_root.parent() {
        fs::create_dir_all::<&std::path::Path>(parent).map_err(|error| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to create managed checkout dir `{}`: {error}",
                    parent.display()
                ),
            }
        })?;
    }

    let mirror_existed = mirror_root.exists();
    let (mirror_repository, mirror_remote_updated) = if mirror_existed {
        let repo = open_bare_with_retry(&mirror_root).map_err(|error| {
            RepoIntelligenceError::InvalidRepositoryPath {
                repo_id: repository.id.clone(),
                path: mirror_root.display().to_string(),
                reason: format!("failed to open managed mirror as bare git repository: {error}"),
            }
        })?;
        let remote_updated = if matches!(mode, RepositorySyncMode::Status) {
            false
        } else {
            ensure_remote_url(&repo, "origin", upstream_url).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to align managed mirror `{}` remote with `{upstream_url}`: {error}",
                        repository.id
                    ),
                }
            })?
        };
        if remote_updated || should_fetch(repository.refresh, mode) {
            fetch_origin_with_retry(&repo).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to refresh managed mirror `{}` from `{upstream_url}`: {error}",
                        repository.id
                    ),
                }
            })?;
        }
        (repo, remote_updated)
    } else {
        (
            clone_bare_with_retry(upstream_url, &mirror_root).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to clone mirror for repository `{}` from `{upstream_url}`: {error}",
                        repository.id
                    ),
                }
            })?,
            false,
        )
    };
    let mirror_revision = metadata::resolve_head_revision(&mirror_repository);
    let mirror_state = lifecycle_state_for(mode, mirror_existed, repository.refresh);
    let mirror_synchronized =
        !mirror_existed || mirror_remote_updated || should_fetch(repository.refresh, mode);

    let checkout_existed = checkout_root.exists();
    let mirror_origin = std::fs::canonicalize(&mirror_root)
        .unwrap_or_else(|_| mirror_root.clone())
        .display()
        .to_string();
    let repository_handle = if checkout_existed {
        let repo = open_checkout_with_retry(&checkout_root).map_err(|error| {
            RepoIntelligenceError::InvalidRepositoryPath {
                repo_id: repository.id.clone(),
                path: checkout_root.display().to_string(),
                reason: format!("failed to open managed checkout as git repository: {error}"),
            }
        })?;
        let remote_updated = if matches!(mode, RepositorySyncMode::Status) {
            false
        } else {
            ensure_remote_url(&repo, "origin", mirror_origin.as_str()).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to align managed checkout `{}` remote with mirror `{mirror_origin}`: {error}",
                        repository.id
                    ),
                }
            })?
        };
        if remote_updated || mirror_synchronized || should_fetch(repository.refresh, mode) {
            fetch_origin_with_retry(&repo).map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to refresh managed checkout `{}` from mirror `{mirror_origin}`: {error}",
                    repository.id
                ),
            })?;
        }
        repo
    } else {
        Repository::clone(&mirror_origin, &checkout_root).map_err(|error| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to materialize managed checkout `{}` from mirror `{mirror_origin}`: {error}",
                    repository.id
                ),
            }
        })?
    };

    if !matches!(mode, RepositorySyncMode::Status) {
        refs::sync_checkout_head(&repository_handle, repository.git_ref.as_ref()).map_err(
            |error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to materialize requested git ref for `{}`: {error}",
                    repository.id
                ),
            },
        )?;
    }

    let revision = metadata::resolve_head_revision(&repository_handle);
    let tracking_revision =
        metadata::resolve_tracking_revision(&repository_handle, repository.git_ref.as_ref());
    let checkout_state = lifecycle_state_for(mode, checkout_existed, repository.refresh);
    let fetched_at = metadata::discover_last_fetched_at(&mirror_root)
        .or_else(|| (!matches!(mode, RepositorySyncMode::Status)).then(|| Utc::now().to_rfc3339()));

    Ok(ResolvedRepositorySource {
        checkout_root: checkout_root.clone(),
        mirror_root: Some(mirror_root),
        mirror_revision: mirror_revision.clone(),
        tracking_revision: tracking_revision.clone(),
        last_fetched_at: fetched_at,
        drift_state: metadata::compute_managed_drift_state(
            &repository_handle,
            revision.as_deref(),
            tracking_revision.as_deref(),
            mirror_revision.as_deref(),
        ),
        mirror_state,
        checkout_state,
        source_kind: ResolvedRepositorySourceKind::ManagedRemote,
    })
}

fn lifecycle_state_for(
    mode: RepositorySyncMode,
    existed: bool,
    refresh: RepositoryRefreshPolicy,
) -> RepositoryLifecycleState {
    if !existed {
        return RepositoryLifecycleState::Created;
    }
    if should_fetch(refresh, mode) {
        return RepositoryLifecycleState::Refreshed;
    }
    if matches!(mode, RepositorySyncMode::Status) {
        RepositoryLifecycleState::Observed
    } else {
        RepositoryLifecycleState::Reused
    }
}
