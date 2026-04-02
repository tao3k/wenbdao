use super::*;
use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::test_support::assert_studio_json_snapshot;
use crate::gateway::studio::types::{UiConfig, UiProjectConfig, UiRepoProjectConfig};
use git2::Repository;
use serde_json::json;
use tempfile::tempdir;

struct VfsFixture {
    state: StudioState,
    _temp_dir: tempfile::TempDir,
}

fn make_vfs_fixture() -> VfsFixture {
    let temp_dir =
        tempdir().unwrap_or_else(|err| panic!("failed to create vfs fixture tempdir: {err}"));
    let docs_dir = temp_dir.path().join("docs");
    let skills_dir = temp_dir.path().join("internal_skills").join("writer");

    std::fs::create_dir_all(&docs_dir)
        .unwrap_or_else(|err| panic!("failed to create docs dir: {err}"));
    std::fs::create_dir_all(&skills_dir)
        .unwrap_or_else(|err| panic!("failed to create internal skills dir: {err}"));

    std::fs::write(docs_dir.join("guide.md"), "# Guide\n\nHello.\n")
        .unwrap_or_else(|err| panic!("failed to write docs fixture: {err}"));
    std::fs::write(skills_dir.join("SKILL.md"), "---\nname: Writer\n---\n")
        .unwrap_or_else(|err| panic!("failed to write skill fixture: {err}"));

    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().to_path_buf();
    state.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string(), "internal_skills".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    VfsFixture {
        state,
        _temp_dir: temp_dir,
    }
}

#[test]
fn scan_roots_only_includes_configured_roots() {
    let fixture = make_vfs_fixture();

    let result = scan_roots(&fixture.state);
    let mut entries = result
        .entries
        .iter()
        .map(|entry| {
            json!({
                "path": entry.path,
                "name": entry.name,
                "isDir": entry.is_dir,
                "category": entry.category,
                "size": entry.size,
                "contentType": entry.content_type,
                "hasFrontmatter": entry.has_frontmatter,
                "wendaoId": entry.wendao_id,
                "projectName": entry.project_name,
                "rootLabel": entry.root_label,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));

    assert_studio_json_snapshot(
        "vfs_scan_roots_payload",
        json!({
            "entries": entries,
            "fileCount": result.file_count,
            "dirCount": result.dir_count,
        }),
    );
}

#[test]
fn list_root_entries_reflects_runtime_root_resolution() {
    let fixture = make_vfs_fixture();

    let entries = list_root_entries(&fixture.state);
    let mut roots = entries
        .iter()
        .map(|entry| {
            json!({
                "path": entry.path,
                "name": entry.name,
                "isDir": entry.is_dir,
                "size": entry.size,
                "contentType": entry.content_type,
                "projectName": entry.project_name,
                "rootLabel": entry.root_label,
            })
        })
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));

    assert_studio_json_snapshot("vfs_root_entries_payload", json!({ "entries": roots }));
}

#[test]
fn get_entry_resolves_configured_relative_roots() {
    let fixture = make_vfs_fixture();

    let entry = get_entry(&fixture.state, "docs/guide.md")
        .unwrap_or_else(|err| panic!("expected docs file entry: {err}"));

    assert_studio_json_snapshot(
        "vfs_get_entry_payload",
        json!({
            "path": entry.path,
            "name": entry.name,
            "isDir": entry.is_dir,
            "size": entry.size,
            "contentType": entry.content_type,
            "projectName": entry.project_name,
            "rootLabel": entry.root_label,
            "projectRoot": entry.project_root,
            "projectDirs": entry.project_dirs,
        }),
    );
}

#[tokio::test]
async fn read_content_supports_only_configured_roots() {
    let fixture = make_vfs_fixture();

    let payload = read_content(&fixture.state, "internal_skills/writer/SKILL.md")
        .await
        .unwrap_or_else(|err| panic!("expected skill content to load: {err}"));

    assert_studio_json_snapshot(
        "vfs_read_content_payload",
        json!({
            "path": payload.path,
            "content": payload.content,
            "contentType": payload.content_type,
        }),
    );
}

#[tokio::test]
async fn glob_dirs_filter_scan_and_read_results() {
    let temp_dir =
        tempdir().unwrap_or_else(|err| panic!("failed to create glob filter tempdir: {err}"));
    let docs_dir = temp_dir.path().join("docs");
    let private_dir = docs_dir.join("private");
    std::fs::create_dir_all(&private_dir)
        .unwrap_or_else(|err| panic!("failed to create glob filter dirs: {err}"));
    std::fs::write(docs_dir.join("guide.md"), "# Public\n")
        .unwrap_or_else(|err| panic!("failed to write public markdown fixture: {err}"));
    std::fs::write(private_dir.join("secret.md"), "# Secret\n")
        .unwrap_or_else(|err| panic!("failed to write private markdown fixture: {err}"));

    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().to_path_buf();
    state.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec![
                "docs".to_string(),
                "**/*.md".to_string(),
                "!docs/private/**".to_string(),
            ],
        }],
        repo_projects: Vec::new(),
    });

    let paths = scan_roots(&state)
        .entries
        .into_iter()
        .map(|entry| entry.path)
        .collect::<Vec<_>>();
    assert!(paths.iter().any(|path| path == "docs/guide.md"));
    assert!(!paths.iter().any(|path| path == "docs/private/secret.md"));

    let blocked = read_content(&state, "docs/private/secret.md").await;
    let Err(VfsError::NotFound(path)) = blocked else {
        panic!("expected glob-filtered file read to fail");
    };
    assert_eq!(path, "docs/private/secret.md");
}

#[test]
fn scan_roots_exposes_project_grouping_metadata_for_monorepo_docs() {
    let temp_dir =
        tempdir().unwrap_or_else(|err| panic!("failed to create monorepo tempdir: {err}"));
    let alpha_project_root = temp_dir.path().join("packages").join("alpha");
    let beta_project_root = temp_dir.path().join("packages").join("beta");
    let alpha_docs = alpha_project_root.join("docs");
    let beta_docs = beta_project_root.join("docs");

    std::fs::create_dir_all(&alpha_docs)
        .unwrap_or_else(|err| panic!("failed to create alpha docs dir: {err}"));
    std::fs::create_dir_all(&beta_docs)
        .unwrap_or_else(|err| panic!("failed to create beta docs dir: {err}"));
    std::fs::write(
        alpha_project_root.join("Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\n",
    )
    .unwrap_or_else(|err| panic!("failed to write alpha Cargo.toml: {err}"));
    std::fs::write(
        beta_project_root.join("Cargo.toml"),
        "[package]\nname = \"beta\"\nversion = \"0.1.0\"\n",
    )
    .unwrap_or_else(|err| panic!("failed to write beta Cargo.toml: {err}"));
    std::fs::write(alpha_docs.join("guide.md"), "# Alpha\n")
        .unwrap_or_else(|err| panic!("failed to write alpha docs fixture: {err}"));
    std::fs::write(beta_docs.join("guide.md"), "# Beta\n")
        .unwrap_or_else(|err| panic!("failed to write beta docs fixture: {err}"));

    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().to_path_buf();
    state.set_ui_config(UiConfig {
        projects: vec![
            UiProjectConfig {
                name: "alpha".to_string(),
                root: "packages/alpha".to_string(),
                dirs: vec!["docs".to_string()],
            },
            UiProjectConfig {
                name: "beta".to_string(),
                root: "packages/beta".to_string(),
                dirs: vec!["docs".to_string()],
            },
        ],
        repo_projects: Vec::new(),
    });

    let mut project_entries = scan_roots(&state)
        .entries
        .into_iter()
        .filter(|entry| entry.project_name.is_some())
        .map(|entry| {
            json!({
                "path": entry.path,
                "name": entry.name,
                "projectName": entry.project_name,
                "rootLabel": entry.root_label,
                "isDir": entry.is_dir,
            })
        })
        .collect::<Vec<_>>();
    project_entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));

    assert_studio_json_snapshot(
        "vfs_monorepo_project_roots_payload",
        json!({ "entries": project_entries }),
    );
}

#[test]
fn resolve_navigation_target_preserves_project_metadata_for_scoped_display_paths() {
    let temp_dir =
        tempdir().unwrap_or_else(|err| panic!("failed to create scoped navigation tempdir: {err}"));
    let alpha_project_root = temp_dir.path().join("packages").join("alpha");
    let beta_project_root = temp_dir.path().join("packages").join("beta");
    let alpha_docs = alpha_project_root.join("docs");
    let beta_docs = beta_project_root.join("docs");

    std::fs::create_dir_all(&alpha_docs)
        .unwrap_or_else(|err| panic!("failed to create alpha docs dir: {err}"));
    std::fs::create_dir_all(&beta_docs)
        .unwrap_or_else(|err| panic!("failed to create beta docs dir: {err}"));
    std::fs::write(alpha_docs.join("guide.md"), "# Alpha\n")
        .unwrap_or_else(|err| panic!("failed to write alpha docs fixture: {err}"));
    std::fs::write(beta_docs.join("guide.md"), "# Beta\n")
        .unwrap_or_else(|err| panic!("failed to write beta docs fixture: {err}"));

    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().to_path_buf();
    state.set_ui_config(UiConfig {
        projects: vec![
            UiProjectConfig {
                name: "alpha".to_string(),
                root: "packages/alpha".to_string(),
                dirs: vec!["docs".to_string()],
            },
            UiProjectConfig {
                name: "beta".to_string(),
                root: "packages/beta".to_string(),
                dirs: vec!["docs".to_string()],
            },
        ],
        repo_projects: Vec::new(),
    });

    let target = resolve_navigation_target(&state, "alpha/docs/guide.md");

    assert_eq!(target.path, "alpha/docs/guide.md");
    assert_eq!(target.project_name.as_deref(), Some("alpha"));
    assert_eq!(target.root_label.as_deref(), Some("docs"));
    assert_eq!(target.category, "doc");
}

#[test]
fn scan_roots_snapshots_config_root_relative_resolution() {
    let temp_dir = tempdir()
        .unwrap_or_else(|err| panic!("failed to create config-root fixture tempdir: {err}"));
    let repo_docs = temp_dir.path().join("docs");
    let studio_docs = temp_dir
        .path()
        .join(".data")
        .join("wendao-frontend")
        .join("docs");

    std::fs::create_dir_all(&repo_docs)
        .unwrap_or_else(|err| panic!("failed to create repo docs dir: {err}"));
    std::fs::create_dir_all(&studio_docs)
        .unwrap_or_else(|err| panic!("failed to create studio docs dir: {err}"));
    std::fs::write(repo_docs.join("main.md"), "# Main\n")
        .unwrap_or_else(|err| panic!("failed to write repo docs fixture: {err}"));
    std::fs::write(studio_docs.join("kernel.md"), "# Kernel\n")
        .unwrap_or_else(|err| panic!("failed to write studio docs fixture: {err}"));

    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().join(".data").join("wendao-frontend");
    state.set_ui_config(UiConfig {
        projects: vec![
            UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            },
            UiProjectConfig {
                name: "main".to_string(),
                root: temp_dir.path().to_string_lossy().to_string(),
                dirs: vec!["docs".to_string()],
            },
        ],
        repo_projects: Vec::new(),
    });

    let mut entries = scan_roots(&state)
        .entries
        .into_iter()
        .map(|entry| {
            json!({
                "path": entry.path,
                "name": entry.name,
                "projectName": entry.project_name,
                "rootLabel": entry.root_label,
                "isDir": entry.is_dir,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));

    assert_studio_json_snapshot(
        "vfs_config_root_relative_resolution_payload",
        json!({ "entries": entries }),
    );
}

#[test]
fn get_entry_supports_tilde_project_root() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let temp_dir = tempfile::Builder::new()
        .prefix("wendao-studio-tilde-")
        .tempdir_in(home.as_path())
        .unwrap_or_else(|err| panic!("failed to create tilde fixture tempdir in HOME: {err}"));

    let docs_dir = temp_dir.path().join("docs");
    std::fs::create_dir_all(&docs_dir)
        .unwrap_or_else(|err| panic!("failed to create tilde docs dir: {err}"));
    std::fs::write(docs_dir.join("guide.md"), "# Tilde Guide\n")
        .unwrap_or_else(|err| panic!("failed to write tilde docs fixture: {err}"));

    let relative_to_home = temp_dir
        .path()
        .strip_prefix(home.as_path())
        .unwrap_or_else(|err| panic!("failed to derive tempdir path under HOME: {err}"))
        .to_string_lossy()
        .replace('\\', "/");
    let configured_root = format!("~/{relative_to_home}");

    let state = StudioState::new();
    state.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "main".to_string(),
            root: configured_root,
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    let entry = get_entry(&state, "docs/guide.md")
        .unwrap_or_else(|err| panic!("expected docs file entry under tilde root: {err}"));
    assert_eq!(entry.path, "docs/guide.md");
    assert_eq!(entry.name, "guide.md");
    assert!(!entry.is_dir);
}

#[test]
fn graph_lookup_candidates_normalize_wendao_uri_targets() {
    let fixture = make_vfs_fixture();

    let candidates =
        graph_lookup_candidates(&fixture.state, "$wendao://internal_skills/writer/SKILL.md");

    assert!(candidates.contains(&"$wendao://internal_skills/writer/SKILL.md".to_string()));
    assert!(candidates.contains(&"wendao://internal_skills/writer/SKILL.md".to_string()));
    assert!(candidates.contains(&"internal_skills/writer/SKILL.md".to_string()));
}

#[test]
fn graph_lookup_candidates_normalize_id_targets() {
    let fixture = make_vfs_fixture();

    let candidates = graph_lookup_candidates(&fixture.state, "id:docs/guide.md");

    assert!(candidates.contains(&"id:docs/guide.md".to_string()));
    assert!(candidates.contains(&"docs/guide.md".to_string()));
}

#[test]
fn scan_roots_carries_project_metadata_on_descendants() {
    let temp_dir = tempdir()
        .unwrap_or_else(|err| panic!("failed to create descendant metadata tempdir: {err}"));
    let alpha_docs = temp_dir.path().join("packages").join("alpha").join("docs");
    let beta_docs = temp_dir.path().join("packages").join("beta").join("docs");

    std::fs::create_dir_all(&alpha_docs)
        .unwrap_or_else(|err| panic!("failed to create alpha docs dir: {err}"));
    std::fs::create_dir_all(&beta_docs)
        .unwrap_or_else(|err| panic!("failed to create beta docs dir: {err}"));
    std::fs::write(alpha_docs.join("alpha-guide.md"), "# Alpha\n")
        .unwrap_or_else(|err| panic!("failed to write alpha guide fixture: {err}"));
    std::fs::write(beta_docs.join("beta-guide.md"), "# Beta\n")
        .unwrap_or_else(|err| panic!("failed to write beta guide fixture: {err}"));

    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().to_path_buf();
    state.set_ui_config(UiConfig {
        projects: vec![
            UiProjectConfig {
                name: "alpha".to_string(),
                root: "packages/alpha".to_string(),
                dirs: vec!["docs".to_string()],
            },
            UiProjectConfig {
                name: "beta".to_string(),
                root: "packages/beta".to_string(),
                dirs: vec!["docs".to_string()],
            },
        ],
        repo_projects: Vec::new(),
    });

    let entries = scan_roots(&state).entries;
    let alpha_entry = entries
        .iter()
        .find(|entry| entry.name == "alpha-guide.md")
        .unwrap_or_else(|| panic!("expected alpha descendant entry in scan payload"));
    let beta_entry = entries
        .iter()
        .find(|entry| entry.name == "beta-guide.md")
        .unwrap_or_else(|| panic!("expected beta descendant entry in scan payload"));

    assert_eq!(alpha_entry.project_name.as_deref(), Some("alpha"));
    assert_eq!(alpha_entry.root_label.as_deref(), Some("docs"));
    assert_eq!(alpha_entry.project_root.as_deref(), Some("packages/alpha"));
    assert_eq!(
        alpha_entry.project_dirs.clone().unwrap_or_default(),
        vec!["docs".to_string()]
    );
    assert_eq!(beta_entry.project_name.as_deref(), Some("beta"));
    assert_eq!(beta_entry.root_label.as_deref(), Some("docs"));
    assert_eq!(beta_entry.project_root.as_deref(), Some("packages/beta"));
    assert_eq!(
        beta_entry.project_dirs.clone().unwrap_or_default(),
        vec!["docs".to_string()]
    );
}

#[tokio::test]
async fn scan_roots_and_read_content_support_repo_checkout_roots() {
    let temp_dir =
        tempdir().unwrap_or_else(|err| panic!("failed to create repo root tempdir: {err}"));
    let repo_root = temp_dir.path().join("repos").join("mcl");
    let modelica_dir = repo_root.join("Modelica");
    std::fs::create_dir_all(&modelica_dir)
        .unwrap_or_else(|err| panic!("failed to create repo fixture dirs: {err}"));
    std::fs::write(
        modelica_dir.join("package.mo"),
        "within Modelica; end Modelica;\n",
    )
    .unwrap_or_else(|err| panic!("failed to write repo fixture file: {err}"));
    Repository::init(&repo_root)
        .unwrap_or_else(|err| panic!("failed to initialize git repo fixture: {err}"));

    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.config_root = temp_dir.path().to_path_buf();
    state.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: vec![UiRepoProjectConfig {
            id: "mcl".to_string(),
            root: Some("repos/mcl".to_string()),
            url: None,
            git_ref: None,
            refresh: Some("manual".to_string()),
            plugins: vec!["modelica".to_string()],
        }],
    });

    let scanned_paths = scan_roots(&state)
        .entries
        .into_iter()
        .map(|entry| entry.path)
        .collect::<Vec<_>>();
    assert!(
        scanned_paths.iter().any(|path| path == "mcl"),
        "expected repo root in VFS scan payload, got: {scanned_paths:?}"
    );
    assert!(
        scanned_paths
            .iter()
            .any(|path| path == "mcl/Modelica/package.mo"),
        "expected repo file path in VFS scan payload, got: {scanned_paths:?}"
    );

    let payload = read_content(&state, "mcl/Modelica/package.mo")
        .await
        .unwrap_or_else(|err| panic!("expected repo checkout content to load: {err}"));
    assert_eq!(payload.path, "mcl/Modelica/package.mo");
    assert!(payload.content.contains("within Modelica;"));
}
