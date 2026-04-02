//! Integration tests for deterministic mixed projected retrieval hit lookup.

use crate::support::repo_intelligence::{assert_repo_json_snapshot, sample_projection_analysis};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    ProjectedPageIndexNode, RepoProjectedPageIndexTreesQuery, RepoProjectedRetrievalHitQuery,
    build_repo_projected_page_index_trees, build_repo_projected_retrieval_hit,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_retrieval_hit_lookup_resolves_page_hit_without_node() -> TestResult {
    let analysis = sample_projection_analysis("projection-sample");

    let result = build_repo_projected_retrieval_hit(
        &RepoProjectedRetrievalHitQuery {
            repo_id: "projection-sample".to_string(),
            page_id: "repo:projection-sample:projection:reference:symbol:repo:projection-sample:symbol:ProjectionPkg.solve"
                .to_string(),
            node_id: None,
        },
        &analysis,
    )?;

    assert_repo_json_snapshot("repo_projected_retrieval_hit_page_result", json!(result));
    Ok(())
}

#[test]
fn projected_retrieval_hit_lookup_resolves_node_hit_when_node_id_is_present() -> TestResult {
    let analysis = sample_projection_analysis("projection-sample");

    let trees = build_repo_projected_page_index_trees(
        &RepoProjectedPageIndexTreesQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    )?;

    let tree = trees
        .trees
        .iter()
        .find(|tree| tree.title == "solve")
        .expect("expected a projected page-index tree titled `solve`");
    let node_id = find_node_id(tree.roots.as_slice(), "Anchors")
        .expect("expected a projected page-index node titled `Anchors`");

    let result = build_repo_projected_retrieval_hit(
        &RepoProjectedRetrievalHitQuery {
            repo_id: "projection-sample".to_string(),
            page_id: tree.page_id.clone(),
            node_id: Some(node_id),
        },
        &analysis,
    )?;

    assert_repo_json_snapshot("repo_projected_retrieval_hit_node_result", json!(result));
    Ok(())
}

fn find_node_id(nodes: &[ProjectedPageIndexNode], title: &str) -> Option<String> {
    for node in nodes {
        if node.title == title {
            return Some(node.node_id.clone());
        }
        if let Some(node_id) = find_node_id(node.children.as_slice(), title) {
            return Some(node_id);
        }
    }
    None
}
