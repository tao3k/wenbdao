use std::sync::Arc;

use super::*;
use crate::gateway::studio::router::{GatewayState, StudioState};
use crate::gateway::studio::types::UiConfig;
use serde::Deserialize;
use serde_json::json;
use tempfile::tempdir;

#[path = "support.rs"]
mod support;
use support::{assert_studio_json_snapshot, round_f32};

struct GraphFixture {
    state: Arc<GatewayState>,
    _temp_dir: tempfile::TempDir,
}

#[derive(Debug, Deserialize)]
struct TestWendaoConfig {
    link_graph: Option<TestLinkGraphConfig>,
}

#[derive(Debug, Deserialize)]
struct TestLinkGraphConfig {
    projects: Option<std::collections::BTreeMap<String, TestProjectConfig>>,
}

#[derive(Debug, Deserialize)]
struct TestProjectConfig {
    root: String,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    watch_patterns: Vec<String>,
    #[serde(default)]
    include_dirs_auto: bool,
    #[serde(default)]
    include_dirs_auto_candidates: Vec<String>,
}

fn make_graph_fixture(docs: Vec<(&str, &str)>) -> GraphFixture {
    let temp_dir =
        tempdir().unwrap_or_else(|err| panic!("failed to create graph fixture tempdir: {err}"));
    for (name, content) in docs {
        let absolute_path = temp_dir.path().join(name);
        if let Some(parent) = absolute_path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("failed to create fixture doc parent {name}: {err}"));
        }
        std::fs::write(absolute_path, content)
            .unwrap_or_else(|err| panic!("failed to write fixture doc {name}: {err}"));
    }

    let mut studio_state = StudioState::new();
    studio_state.project_root = temp_dir.path().to_path_buf();

    GraphFixture {
        state: Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio_state),
        }),
        _temp_dir: temp_dir,
    }
}

fn push_ui_config_from_toml(fixture: &GraphFixture, toml_content: &str) {
    let parsed: TestWendaoConfig = toml::from_str(toml_content)
        .unwrap_or_else(|err| panic!("failed to parse test wendao.toml: {err}"));
    let projects = parsed
        .link_graph
        .and_then(|link_graph| link_graph.projects)
        .unwrap_or_default()
        .into_iter()
        .map(
            |(name, project)| crate::gateway::studio::types::UiProjectConfig {
                name,
                root: project.root,
                paths: project.paths,
                watch_patterns: project.watch_patterns,
                include_dirs_auto: project.include_dirs_auto,
                include_dirs_auto_candidates: project.include_dirs_auto_candidates,
            },
        )
        .collect::<Vec<_>>();

    fixture.state.studio.set_ui_config(UiConfig { projects });
}

#[tokio::test]
async fn node_neighbors_returns_live_neighbors() {
    let fixture = make_graph_fixture(vec![
        ("alpha.md", "# Alpha\n\nSee [[beta]].\n"),
        ("beta.md", "# Beta\n\nSee [[gamma]].\n"),
        ("gamma.md", "# Gamma\n\nTail node.\n"),
    ]);
    push_ui_config_from_toml(
        &fixture,
        r#"
[link_graph.projects.kernel]
root = "."
paths = ["."]
watch_patterns = ["**/*.md"]
"#,
    );

    let result = node_neighbors(fixture.state.as_ref(), "alpha.md").await;
    let Ok(response) = result else {
        panic!("expected node neighbors request to succeed");
    };

    assert_studio_json_snapshot(
        "graph_node_neighbors",
        json!({
            "nodeId": response.node_id,
            "name": response.name,
            "nodeType": response.node_type,
            "incoming": response.incoming,
            "outgoing": response.outgoing,
            "twoHop": response.two_hop,
        }),
    );
}

#[tokio::test]
async fn graph_neighbors_includes_center_node_and_links() {
    let fixture = make_graph_fixture(vec![
        ("alpha.md", "# Alpha\n\nSee [[beta]].\n"),
        ("beta.md", "# Beta\n\nBody.\n"),
    ]);
    push_ui_config_from_toml(
        &fixture,
        r#"
[link_graph.projects.kernel]
root = "."
paths = ["."]
watch_patterns = ["**/*.md"]
"#,
    );

    let result = graph_neighbors(fixture.state.as_ref(), "alpha.md", "both", 2, 10).await;
    let Ok(response) = result else {
        panic!("expected graph neighbors request to succeed");
    };

    let mut nodes = response
        .nodes
        .into_iter()
        .map(|node| {
            json!({
                "id": node.id,
                "label": node.label,
                "path": node.path,
                "nodeType": node.node_type,
                "isCenter": node.is_center,
                "distance": node.distance,
            })
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));

    let mut links = response
        .links
        .into_iter()
        .map(|link| {
            json!({
                "source": link.source,
                "target": link.target,
                "direction": link.direction,
                "distance": link.distance,
            })
        })
        .collect::<Vec<_>>();
    links.sort_by(|left, right| {
        left["source"]
            .as_str()
            .cmp(&right["source"].as_str())
            .then_with(|| left["target"].as_str().cmp(&right["target"].as_str()))
    });

    assert_studio_json_snapshot(
        "graph_neighbors_payload",
        json!({
            "center": {
                "id": response.center.id,
                "label": response.center.label,
                "path": response.center.path,
                "nodeType": response.center.node_type,
                "isCenter": response.center.is_center,
                "distance": response.center.distance,
            },
            "nodes": nodes,
            "links": links,
            "totalNodes": response.total_nodes,
            "totalLinks": response.total_links,
        }),
    );
}

#[tokio::test]
async fn topology_3d_returns_nodes_and_links() {
    let fixture = make_graph_fixture(vec![
        ("alpha.md", "# Alpha\n\nSee [[beta]].\n"),
        ("beta.md", "# Beta\n\nBody.\n"),
    ]);
    push_ui_config_from_toml(
        &fixture,
        r#"
[link_graph.projects.kernel]
root = "."
paths = ["."]
watch_patterns = ["**/*.md"]
"#,
    );

    let result = topology_3d(fixture.state.as_ref()).await;
    let Ok(response) = result else {
        panic!("expected topology request to succeed");
    };

    let mut nodes = response
        .nodes
        .into_iter()
        .map(|node| {
            json!({
                "id": node.id,
                "name": node.name,
                "nodeType": node.node_type,
                "position": node.position.map(round_f32),
                "clusterId": node.cluster_id,
            })
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));

    let mut links = response
        .links
        .into_iter()
        .map(|link| {
            json!({
                "from": link.from,
                "to": link.to,
                "label": link.label,
            })
        })
        .collect::<Vec<_>>();
    links.sort_by(|left, right| {
        left["from"]
            .as_str()
            .cmp(&right["from"].as_str())
            .then_with(|| left["to"].as_str().cmp(&right["to"].as_str()))
    });

    let mut clusters = response
        .clusters
        .into_iter()
        .map(|cluster| {
            json!({
                "id": cluster.id,
                "name": cluster.name,
                "centroid": cluster.centroid.map(round_f32),
                "nodeCount": cluster.node_count,
                "color": cluster.color,
            })
        })
        .collect::<Vec<_>>();
    clusters.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));

    assert_studio_json_snapshot(
        "topology_3d_payload",
        json!({
            "nodes": nodes,
            "links": links,
            "clusters": clusters,
        }),
    );
}

#[tokio::test]
async fn graph_neighbors_returns_not_found_for_unknown_node() {
    let fixture = make_graph_fixture(vec![("alpha.md", "# Alpha\n\nBody.\n")]);
    push_ui_config_from_toml(
        &fixture,
        r#"
[link_graph.projects.kernel]
root = "."
paths = ["."]
watch_patterns = ["**/*.md"]
"#,
    );

    let result = graph_neighbors(fixture.state.as_ref(), "missing.md", "both", 2, 10).await;
    let Err(error) = result else {
        panic!("expected missing node lookup to fail");
    };

    assert_eq!(error.status(), axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error.code(), "NOT_FOUND");
}

#[tokio::test]
async fn graph_neighbors_resolves_vfs_alias_paths() {
    let fixture = make_graph_fixture(vec![
        ("packages/alpha/docs/index.md", "# Alpha\n\nBody.\n"),
        ("packages/beta/docs/index.md", "# Beta\n\nBody.\n"),
    ]);
    push_ui_config_from_toml(
        &fixture,
        r#"
[link_graph.projects.alpha]
root = "packages/alpha"
paths = ["docs"]
watch_patterns = ["**/*.md"]

[link_graph.projects.beta]
root = "packages/beta"
paths = ["docs"]
watch_patterns = ["**/*.md"]
"#,
    );

    let result = graph_neighbors(fixture.state.as_ref(), "docs-2/index.md", "both", 1, 10).await;
    let Ok(response) = result else {
        panic!("expected aliased graph neighbors request to succeed");
    };

    assert_studio_json_snapshot(
        "graph_neighbors_vfs_alias_payload",
        json!({
            "center": {
                "id": response.center.id,
                "label": response.center.label,
                "path": response.center.path,
                "nodeType": response.center.node_type,
                "isCenter": response.center.is_center,
                "distance": response.center.distance,
            },
            "nodes": response.nodes.into_iter().map(|node| {
                json!({
                    "id": node.id,
                    "label": node.label,
                    "path": node.path,
                    "nodeType": node.node_type,
                    "isCenter": node.is_center,
                    "distance": node.distance,
                })
            }).collect::<Vec<_>>(),
            "links": response.links.into_iter().map(|link| {
                json!({
                    "source": link.source,
                    "target": link.target,
                    "direction": link.direction,
                    "distance": link.distance,
                })
            }).collect::<Vec<_>>(),
            "totalNodes": response.total_nodes,
            "totalLinks": response.total_links,
        }),
    );
}

#[tokio::test]
async fn graph_neighbors_indexes_configured_projects_outside_knowledge_root() {
    let fixture = make_graph_fixture(vec![
        ("docs/overview.md", "# Overview\n\nKernel docs.\n"),
        (
            ".data/qianji-studio/docs/03_features/202_topology_and_graph_navigation.md",
            "# Topology\n\nSee [[overview]].\n",
        ),
    ]);
    push_ui_config_from_toml(
        &fixture,
        r#"
[link_graph.projects.kernel]
root = "."
paths = ["docs"]
watch_patterns = ["**/*.md"]

[link_graph.projects.qianji_studio]
root = ".data/qianji-studio"
paths = ["docs"]
watch_patterns = ["**/*.md"]
"#,
    );

    let result = graph_neighbors(
        fixture.state.as_ref(),
        "docs-2/03_features/202_topology_and_graph_navigation.md",
        "both",
        1,
        10,
    )
    .await;
    let Ok(response) = result else {
        panic!("expected configured project graph neighbors request to succeed");
    };

    assert_studio_json_snapshot(
        "graph_configured_project_alias_payload",
        json!({
            "center": {
                "id": response.center.id,
                "label": response.center.label,
                "path": response.center.path,
                "nodeType": response.center.node_type,
                "isCenter": response.center.is_center,
                "distance": response.center.distance,
            },
            "nodes": response.nodes.into_iter().map(|node| {
                json!({
                    "id": node.id,
                    "label": node.label,
                    "path": node.path,
                    "nodeType": node.node_type,
                    "isCenter": node.is_center,
                    "distance": node.distance,
                })
            }).collect::<Vec<_>>(),
            "links": response.links.into_iter().map(|link| {
                json!({
                    "source": link.source,
                    "target": link.target,
                    "direction": link.direction,
                    "distance": link.distance,
                })
            }).collect::<Vec<_>>(),
            "totalNodes": response.total_nodes,
            "totalLinks": response.total_links,
        }),
    );
}

#[tokio::test]
async fn graph_neighbors_rebuilds_after_ui_config_update() {
    let fixture = make_graph_fixture(vec![
        ("docs/overview.md", "# Overview\n\nKernel docs.\n"),
        (
            ".data/qianji-studio/docs/03_features/202_topology_and_graph_navigation.md",
            "# Topology\n\nSee [[overview]].\n",
        ),
    ]);

    let missing = graph_neighbors(
        fixture.state.as_ref(),
        "docs-2/03_features/202_topology_and_graph_navigation.md",
        "both",
        1,
        10,
    )
    .await;
    let Err(error) = missing else {
        panic!("expected graph request to fail before ui config is pushed");
    };

    assert_eq!(error.status(), axum::http::StatusCode::NOT_FOUND);

    push_ui_config_from_toml(
        &fixture,
        r#"
[link_graph.projects.kernel]
root = "."
paths = ["docs"]
watch_patterns = ["**/*.md"]

[link_graph.projects.qianji_studio]
root = ".data/qianji-studio"
paths = ["docs"]
watch_patterns = ["**/*.md"]
"#,
    );

    let rebuilt = graph_neighbors(
        fixture.state.as_ref(),
        "docs-2/03_features/202_topology_and_graph_navigation.md",
        "both",
        1,
        10,
    )
    .await;
    let Ok(response) = rebuilt else {
        panic!("expected graph request to succeed after ui config update");
    };

    assert_eq!(
        response.center.path,
        "docs-2/03_features/202_topology_and_graph_navigation.md"
    );
}
