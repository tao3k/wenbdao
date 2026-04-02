use xiuxian_io::PrjDirs;

use crate::analyzers::config::{RegisteredRepository, RepositoryRefreshPolicy};

#[test]
fn managed_repo_paths_follow_ghq_layout_for_remote_urls() {
    let repository = RegisteredRepository {
        id: "sciml".to_string(),
        path: None,
        url: Some("https://github.com/SciML/BaseModelica.jl.git".to_string()),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };

    assert_eq!(
        crate::git::checkout::namespace::managed_checkout_root_for(&repository),
        PrjDirs::data_home()
            .join("xiuxian-wendao")
            .join("repo-intelligence")
            .join("repos")
            .join("github.com")
            .join("SciML")
            .join("BaseModelica.jl")
    );
    assert_eq!(
        crate::git::checkout::namespace::managed_mirror_root_for(&repository),
        PrjDirs::data_home()
            .join("xiuxian-wendao")
            .join("repo-intelligence")
            .join("mirrors")
            .join("github.com")
            .join("SciML")
            .join("BaseModelica.jl.git")
    );
}

#[test]
fn managed_repo_paths_support_scp_style_remote_urls() {
    let repository = RegisteredRepository {
        id: "sciml".to_string(),
        path: None,
        url: Some("git@github.com:SciML/BaseModelica.jl.git".to_string()),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };

    assert_eq!(
        crate::git::checkout::namespace::managed_checkout_root_for(&repository),
        PrjDirs::data_home()
            .join("xiuxian-wendao")
            .join("repo-intelligence")
            .join("repos")
            .join("github.com")
            .join("SciML")
            .join("BaseModelica.jl")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn resolve_repository_source_overrides_managed_remote_url_from_config() {
    use std::fs;
    use std::path::Path;

    use git2::{Repository, Signature};
    use uuid::Uuid;

    use crate::git::checkout::{RepositorySyncMode, resolve_repository_source};

    use super::helpers::init_test_repository;

    let source_a = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(source_a.path().join("src")).expect("create src dir");
    init_test_repository(source_a.path());
    fs::write(
        source_a.path().join("src").join("source_a.jl"),
        "const SOURCE = :a\n",
    )
    .expect("write source a marker");
    let repository_a = Repository::open(source_a.path()).expect("open source a repository");
    let mut index_a = repository_a.index().expect("open source a index");
    index_a
        .add_path(Path::new("src/source_a.jl"))
        .expect("stage source a marker");
    let tree_id_a = index_a.write_tree().expect("write source a tree");
    let tree_a = repository_a
        .find_tree(tree_id_a)
        .expect("find source a tree");
    let signature =
        Signature::now("checkout-test", "checkout-test@example.com").expect("signature");
    let head_a = repository_a
        .head()
        .expect("source a head")
        .peel_to_commit()
        .expect("source a commit");
    repository_a
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "add source a marker",
            &tree_a,
            &[&head_a],
        )
        .expect("commit source a marker");

    let source_b = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(source_b.path().join("src")).expect("create src dir");
    init_test_repository(source_b.path());
    fs::write(
        source_b.path().join("src").join("source_b.jl"),
        "const SOURCE = :b\n",
    )
    .expect("write source b marker");
    let repository_b = Repository::open(source_b.path()).expect("open source b repository");
    let mut index_b = repository_b.index().expect("open source b index");
    index_b
        .add_path(Path::new("src/source_b.jl"))
        .expect("stage source b marker");
    let tree_id_b = index_b.write_tree().expect("write source b tree");
    let tree_b = repository_b
        .find_tree(tree_id_b)
        .expect("find source b tree");
    let head_b = repository_b
        .head()
        .expect("source b head")
        .peel_to_commit()
        .expect("source b commit");
    repository_b
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "add source b marker",
            &tree_b,
            &[&head_b],
        )
        .expect("commit source b marker");

    let repo_id = format!("managed-url-override-{}", Uuid::new_v4());
    let repository = RegisteredRepository {
        id: repo_id.clone(),
        path: None,
        url: Some(source_a.path().display().to_string()),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let mirror_root = crate::git::checkout::namespace::managed_mirror_root_for(&repository);
    let checkout_root = crate::git::checkout::namespace::managed_checkout_root_for(&repository);
    if mirror_root.exists() {
        fs::remove_dir_all(&mirror_root).expect("cleanup stale mirror");
    }
    if checkout_root.exists() {
        fs::remove_dir_all(&checkout_root).expect("cleanup stale checkout");
    }

    let resolved_a = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve managed checkout from source a");
    let mirror_repository_a =
        Repository::open_bare(resolved_a.mirror_root.as_ref().expect("mirror root"))
            .expect("open mirror repository a");
    assert_eq!(
        crate::git::checkout::managed::current_remote_url(&mirror_repository_a, "origin")
            .as_deref(),
        Some(source_a.path().display().to_string().as_str())
    );
    assert!(resolved_a.checkout_root.join("src/source_a.jl").exists());

    let repository = RegisteredRepository {
        id: repo_id,
        path: None,
        url: Some(source_b.path().display().to_string()),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let resolved_b = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve managed checkout from source b");
    let mirror_repository_b =
        Repository::open_bare(resolved_b.mirror_root.as_ref().expect("mirror root"))
            .expect("open mirror repository b");
    assert_eq!(
        crate::git::checkout::managed::current_remote_url(&mirror_repository_b, "origin")
            .as_deref(),
        Some(source_b.path().display().to_string().as_str())
    );
    assert!(resolved_b.checkout_root.join("src/source_b.jl").exists());

    fs::remove_dir_all(
        resolved_b
            .mirror_root
            .as_ref()
            .expect("mirror root should exist"),
    )
    .expect("cleanup managed mirror");
    fs::remove_dir_all(&resolved_b.checkout_root).expect("cleanup managed checkout");
}
