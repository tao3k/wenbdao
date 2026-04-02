use std::cmp::Ordering;
use std::fs;
use std::path::Path;

use chrono::Utc;
use git2::{Oid, Repository};

use crate::analyzers::config::RepositoryRef;
use crate::analyzers::query::RepoSyncDriftState;

use super::LocalCheckoutMetadata;

/// Discovers metadata from a local checkout path.
#[must_use]
pub fn discover_checkout_metadata(path: &Path) -> Option<LocalCheckoutMetadata> {
    if !path.is_dir() {
        return None;
    }

    let repository = Repository::open(path).ok()?;
    Some(LocalCheckoutMetadata {
        revision: resolve_head_revision(&repository),
        remote_url: repository
            .find_remote("origin")
            .ok()
            .and_then(|remote| remote.url().map(str::to_string)),
    })
}

pub(super) fn resolve_head_revision(repository: &Repository) -> Option<String> {
    repository
        .head()
        .ok()
        .and_then(|head| head.target().map(|oid| oid.to_string()))
}

pub(super) fn resolve_tracking_revision(
    repository: &Repository,
    git_ref: Option<&RepositoryRef>,
) -> Option<String> {
    match git_ref {
        Some(RepositoryRef::Commit(sha)) => Some(sha.clone()),
        Some(RepositoryRef::Tag(tag)) => repository
            .find_reference(format!("refs/tags/{tag}").as_str())
            .ok()
            .and_then(|reference| reference.target().map(|oid| oid.to_string())),
        Some(RepositoryRef::Branch(branch)) => repository
            .find_reference(format!("refs/remotes/origin/{branch}").as_str())
            .ok()
            .and_then(|reference| reference.target().map(|oid| oid.to_string())),
        None => repository
            .find_reference("refs/remotes/origin/HEAD")
            .ok()
            .and_then(|reference| reference.symbolic_target().map(str::to_string))
            .and_then(|target| repository.find_reference(target.as_str()).ok())
            .and_then(|reference| reference.target().map(|oid| oid.to_string()))
            .or_else(|| {
                [
                    "refs/remotes/origin/main".to_string(),
                    "refs/remotes/origin/master".to_string(),
                ]
                .into_iter()
                .find_map(|reference| {
                    repository
                        .find_reference(reference.as_str())
                        .ok()
                        .and_then(|reference| reference.target().map(|oid| oid.to_string()))
                })
            }),
    }
}

pub(super) fn compute_managed_drift_state(
    repository: &Repository,
    checkout_revision: Option<&str>,
    tracking_revision: Option<&str>,
    mirror_revision: Option<&str>,
) -> RepoSyncDriftState {
    let Some(checkout_revision) = checkout_revision else {
        return RepoSyncDriftState::Unknown;
    };
    let Some(mirror_revision) = mirror_revision else {
        return RepoSyncDriftState::Unknown;
    };

    if checkout_revision == mirror_revision {
        return RepoSyncDriftState::InSync;
    }

    let Some(tracking_revision) = tracking_revision else {
        return RepoSyncDriftState::Unknown;
    };

    if checkout_revision == tracking_revision {
        return RepoSyncDriftState::Behind;
    }

    if tracking_revision == mirror_revision {
        return match compare_revision_lineage(repository, checkout_revision, tracking_revision) {
            Some(Ordering::Greater) => RepoSyncDriftState::Ahead,
            Some(Ordering::Less) => RepoSyncDriftState::Behind,
            Some(Ordering::Equal) => RepoSyncDriftState::InSync,
            None => RepoSyncDriftState::Diverged,
        };
    }

    RepoSyncDriftState::Diverged
}

fn compare_revision_lineage(repository: &Repository, left: &str, right: &str) -> Option<Ordering> {
    let left = Oid::from_str(left).ok()?;
    let right = Oid::from_str(right).ok()?;

    if left == right {
        return Some(Ordering::Equal);
    }

    let left_descends = repository.graph_descendant_of(left, right).ok()?;
    let right_descends = repository.graph_descendant_of(right, left).ok()?;

    match (left_descends, right_descends) {
        (true, false) => Some(Ordering::Greater),
        (false, true) => Some(Ordering::Less),
        (false, false) => None,
        (true, true) => Some(Ordering::Equal),
    }
}

pub(super) fn discover_last_fetched_at(mirror_root: &Path) -> Option<String> {
    ["FETCH_HEAD", "HEAD"]
        .into_iter()
        .filter_map(|name| fs::metadata(mirror_root.join(name)).ok())
        .filter_map(|metadata| metadata.modified().ok())
        .max()
        .map(|modified| chrono::DateTime::<Utc>::from(modified).to_rfc3339())
}
