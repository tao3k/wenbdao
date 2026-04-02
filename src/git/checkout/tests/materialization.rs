use std::fs;
use std::path::Path;

use uuid::Uuid;
use xiuxian_io::PrjDirs;

use crate::analyzers::config::{RegisteredRepository, RepositoryRefreshPolicy};
use crate::git::checkout::{
    RepositorySyncMode, ResolvedRepositorySourceKind, discover_checkout_metadata,
    resolve_repository_source,
};

use super::helpers::init_test_repository;

#[test]
fn resolve_repository_source_materializes_remote_checkout_under_prj_data_home() {
    let source = tempfile::tempdir().expect("tempdir");
    init_test_repository(source.path());
    let repo_id = format!("checkout-test-{}", Uuid::new_v4());

    let repository = RegisteredRepository {
        id: repo_id.clone(),
        path: None,
        url: Some(source.path().display().to_string()),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let target_root = crate::git::checkout::namespace::managed_checkout_root_for(&repository);
    if target_root.exists() {
        fs::remove_dir_all(&target_root).expect("cleanup stale checkout");
    }

    let resolved = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve managed checkout");

    assert!(resolved.checkout_root.starts_with(PrjDirs::data_home()));
    assert!(resolved.checkout_root.is_dir());
    assert_eq!(
        resolved.source_kind,
        ResolvedRepositorySourceKind::ManagedRemote
    );
    assert!(resolved.tracking_revision.is_some());
    let metadata =
        discover_checkout_metadata(&resolved.checkout_root).expect("discover checkout metadata");
    assert_eq!(
        metadata.remote_url.as_deref(),
        Some(
            std::fs::canonicalize(
                resolved
                    .mirror_root
                    .as_ref()
                    .expect("managed checkout should expose mirror root"),
            )
            .unwrap_or_else(|_| {
                resolved
                    .mirror_root
                    .clone()
                    .expect("managed checkout should expose mirror root")
            })
            .display()
            .to_string()
            .as_str(),
        )
    );
    let mirror_metadata = discover_checkout_metadata(
        resolved
            .mirror_root
            .as_deref()
            .expect("managed checkout should expose mirror root"),
    )
    .expect("discover managed mirror metadata");
    assert_eq!(
        mirror_metadata.remote_url.as_deref(),
        Some(source.path().display().to_string().as_str())
    );

    fs::remove_dir_all(&resolved.checkout_root).expect("cleanup managed checkout");
}
