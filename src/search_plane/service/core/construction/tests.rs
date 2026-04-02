use std::path::PathBuf;

use crate::search_plane::service::core::construction::concurrency::repo_search_read_concurrency_limit_with_lookup;
use crate::search_plane::service::helpers::{default_storage_root, manifest_keyspace_for_project};
use crate::search_plane::{SearchMaintenancePolicy, SearchPlaneService};

#[test]
fn repo_search_read_concurrency_limit_defaults_from_parallelism() {
    let limit = repo_search_read_concurrency_limit_with_lookup(&|_| None, Some(12));
    assert_eq!(limit, 3);
}

#[test]
fn repo_search_read_concurrency_limit_accepts_positive_override() {
    let limit = repo_search_read_concurrency_limit_with_lookup(
        &|key| (key == "XIUXIAN_WENDAO_REPO_SEARCH_READ_CONCURRENCY").then(|| "9".to_string()),
        Some(12),
    );
    assert_eq!(limit, 9);
}

#[test]
fn repo_search_read_concurrency_limit_ignores_invalid_override() {
    let limit = repo_search_read_concurrency_limit_with_lookup(
        &|key| {
            (key == "XIUXIAN_WENDAO_REPO_SEARCH_READ_CONCURRENCY").then(|| "invalid".to_string())
        },
        Some(6),
    );
    assert_eq!(limit, 2);
}

#[test]
fn repo_search_parallelism_reuses_repo_read_budget() {
    let project_root = PathBuf::from("/tmp/search-plane-service");
    let storage_root = default_storage_root(project_root.as_path());
    let manifest_keyspace = manifest_keyspace_for_project(project_root.as_path());
    let service = SearchPlaneService::with_paths(
        project_root,
        storage_root,
        manifest_keyspace,
        SearchMaintenancePolicy::default(),
    );

    assert_eq!(
        service.repo_search_parallelism(usize::MAX),
        service.repo_search_read_concurrency_limit
    );
    assert_eq!(service.repo_search_parallelism(2), 2);
    assert_eq!(service.repo_search_parallelism(0), 1);
}
