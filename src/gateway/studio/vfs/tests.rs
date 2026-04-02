use std::fs;
use std::path::Path;

use git2::{Repository, Signature};
use uuid::Uuid;

use super::content::resolve_vfs_path;
use super::roots::resolve_all_vfs_roots;
use super::scan::scan_all_roots;
use super::scan_roots;
use crate::gateway::studio::router::{StudioState, configured_repositories};
use crate::gateway::studio::types::{UiConfig, UiRepoProjectConfig};
use crate::git::checkout::{RepositorySyncMode, resolve_repository_source};

fn init_git_repository(root: &Path) {
    let repository =
        Repository::init(root).unwrap_or_else(|error| panic!("init repository: {error}"));
    fs::write(
        root.join("Project.toml"),
        "name = \"BaseModelica\"\nversion = \"0.1.0\"\n",
    )
    .unwrap_or_else(|error| panic!("write project file: {error}"));
    fs::create_dir_all(root.join("src")).unwrap_or_else(|error| panic!("create src dir: {error}"));
    fs::write(
        root.join("src").join("BaseModelica.jl"),
        "module BaseModelica\nend\n",
    )
    .unwrap_or_else(|error| panic!("write julia source: {error}"));

    let mut index = repository
        .index()
        .unwrap_or_else(|error| panic!("open index: {error}"));
    index
        .add_path(Path::new("Project.toml"))
        .unwrap_or_else(|error| panic!("stage project file: {error}"));
    index
        .add_path(Path::new("src/BaseModelica.jl"))
        .unwrap_or_else(|error| panic!("stage source file: {error}"));
    let tree_id = index
        .write_tree()
        .unwrap_or_else(|error| panic!("write tree: {error}"));
    let tree = repository
        .find_tree(tree_id)
        .unwrap_or_else(|error| panic!("find tree: {error}"));
    let signature = Signature::now("vfs-test", "vfs-test@example.com")
        .unwrap_or_else(|error| panic!("signature: {error}"));
    repository
        .commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
        .unwrap_or_else(|error| panic!("commit: {error}"));
}

#[test]
fn scan_all_roots_includes_repo_project_checkout_entries() {
    let source = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(source.path());
    let repo_id = format!("repo-vfs-{}", Uuid::new_v4());
    let state = StudioState::new();
    state.set_ui_config(UiConfig {
        projects: Vec::new(),
        repo_projects: vec![UiRepoProjectConfig {
            id: repo_id.clone(),
            root: None,
            url: Some(source.path().display().to_string()),
            git_ref: None,
            refresh: Some("manual".to_string()),
            plugins: vec!["julia".to_string()],
        }],
    });
    let repositories = configured_repositories(&state);
    let repository = repositories
        .first()
        .unwrap_or_else(|| panic!("configured repository"));
    resolve_repository_source(
        repository,
        state.config_root.as_path(),
        RepositorySyncMode::Ensure,
    )
    .unwrap_or_else(|error| panic!("materialize checkout before scan: {error}"));

    let result = scan_all_roots(&state);

    assert!(
        result
            .entries
            .iter()
            .any(|entry| entry.path == format!("{repo_id}/src/BaseModelica.jl"))
    );

    for root in resolve_all_vfs_roots(&state) {
        if root.request_root == repo_id && root.full_path.exists() {
            fs::remove_dir_all(root.full_path)
                .unwrap_or_else(|error| panic!("cleanup managed checkout: {error}"));
        }
    }
}

#[test]
fn resolve_vfs_path_supports_repo_project_checkout_files() {
    let source = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(source.path());
    let repo_id = format!("repo-vfs-resolve-{}", Uuid::new_v4());
    let state = StudioState::new();
    state.set_ui_config(UiConfig {
        projects: Vec::new(),
        repo_projects: vec![UiRepoProjectConfig {
            id: repo_id.clone(),
            root: None,
            url: Some(source.path().display().to_string()),
            git_ref: None,
            refresh: Some("manual".to_string()),
            plugins: vec!["julia".to_string()],
        }],
    });
    let repositories = configured_repositories(&state);
    let repository = repositories
        .first()
        .unwrap_or_else(|| panic!("configured repository"));
    resolve_repository_source(
        repository,
        state.config_root.as_path(),
        RepositorySyncMode::Ensure,
    )
    .unwrap_or_else(|error| panic!("materialize checkout before resolving: {error}"));

    let resolved = resolve_vfs_path(&state, format!("{repo_id}/src/BaseModelica.jl").as_str())
        .unwrap_or_else(|error| panic!("resolve repo vfs path: {error:?}"));

    assert!(resolved.full_path.is_file());

    for root in resolve_all_vfs_roots(&state) {
        if root.request_root == repo_id && root.full_path.exists() {
            fs::remove_dir_all(root.full_path)
                .unwrap_or_else(|error| panic!("cleanup managed checkout: {error}"));
        }
    }
}

#[test]
fn scan_roots_reuses_cached_entries_until_ui_config_changes() {
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let project_root = temp.path().join("workspace");
    let docs_dir = project_root.join("docs");
    fs::create_dir_all(&docs_dir).unwrap_or_else(|error| panic!("create docs dir: {error}"));
    fs::write(docs_dir.join("guide.md"), "# guide\n")
        .unwrap_or_else(|error| panic!("write guide: {error}"));

    let state = StudioState::new();
    state.set_ui_config(UiConfig {
        projects: vec![crate::gateway::studio::types::UiProjectConfig {
            name: "kernel".to_string(),
            root: project_root.display().to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    let first = scan_roots(&state);
    assert!(
        first
            .entries
            .iter()
            .any(|entry| entry.path == "kernel/docs/guide.md")
    );
    fs::remove_file(docs_dir.join("guide.md"))
        .unwrap_or_else(|error| panic!("remove guide: {error}"));
    let cached = scan_roots(&state);
    assert_eq!(cached.entries, first.entries);

    let notes_dir = project_root.join("notes");
    fs::create_dir_all(&notes_dir).unwrap_or_else(|error| panic!("create notes dir: {error}"));
    fs::write(notes_dir.join("todo.md"), "# todo\n")
        .unwrap_or_else(|error| panic!("write note: {error}"));

    state.set_ui_config(UiConfig {
        projects: vec![crate::gateway::studio::types::UiProjectConfig {
            name: "kernel".to_string(),
            root: project_root.display().to_string(),
            dirs: vec!["docs".to_string(), "notes".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    let refreshed = scan_roots(&state);
    assert!(
        refreshed
            .entries
            .iter()
            .any(|entry| entry.path == "kernel/notes/todo.md")
    );
    assert!(
        refreshed
            .entries
            .iter()
            .all(|entry| entry.path != "kernel/docs/guide.md")
    );
}
