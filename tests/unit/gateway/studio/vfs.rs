use super::*;
use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::types::{UiConfig, UiProjectConfig};
use serde_json::json;
use tempfile::tempdir;

#[path = "support.rs"]
mod support;
use support::assert_studio_json_snapshot;

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
    state.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            paths: vec!["docs".to_string(), "internal_skills".to_string()],
            watch_patterns: vec!["**/*.md".to_string(), "**/SKILL.md".to_string()],
            include_dirs_auto: true,
            include_dirs_auto_candidates: vec!["docs".to_string(), "internal_skills".to_string()],
        }],
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
    state.set_ui_config(UiConfig {
        projects: vec![
            UiProjectConfig {
                name: "alpha".to_string(),
                root: "packages/alpha".to_string(),
                paths: vec!["docs".to_string()],
                watch_patterns: vec!["**/*.md".to_string()],
                include_dirs_auto: false,
                include_dirs_auto_candidates: Vec::new(),
            },
            UiProjectConfig {
                name: "beta".to_string(),
                root: "packages/beta".to_string(),
                paths: vec!["docs".to_string()],
                watch_patterns: vec!["**/*.md".to_string()],
                include_dirs_auto: false,
                include_dirs_auto_candidates: Vec::new(),
            },
        ],
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
