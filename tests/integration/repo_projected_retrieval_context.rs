//! Integration tests for deterministic mixed projected retrieval context.

use crate::support::repo_intelligence::{assert_repo_json_snapshot, sample_projection_analysis};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    ProjectedPageIndexNode, RepoProjectedPageIndexTreesQuery, RepoProjectedPagesQuery,
    RepoProjectedRetrievalContextQuery, build_repo_projected_page_index_trees,
    build_repo_projected_pages, build_repo_projected_retrieval_context,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_retrieval_context_lookup_resolves_page_context() -> TestResult {
    let analysis = sample_projection_analysis("projection-sample");

    let pages = build_repo_projected_pages(
        &RepoProjectedPagesQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    );
    let page = pages
        .pages
        .iter()
        .find(|page| page.title == "solve")
        .expect("expected a projected page titled `solve`");

    let result = build_repo_projected_retrieval_context(
        &RepoProjectedRetrievalContextQuery {
            repo_id: "projection-sample".to_string(),
            page_id: page.page_id.clone(),
            node_id: None,
            related_limit: 3,
        },
        &analysis,
    )?;

    assert_repo_json_snapshot(
        "repo_projected_retrieval_context_page_result",
        json!(result),
    );
    Ok(())
}

#[test]
fn projected_retrieval_context_lookup_resolves_node_context() -> TestResult {
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

    let result = build_repo_projected_retrieval_context(
        &RepoProjectedRetrievalContextQuery {
            repo_id: "projection-sample".to_string(),
            page_id: tree.page_id.clone(),
            node_id: Some(node_id),
            related_limit: 3,
        },
        &analysis,
    )?;

    assert_repo_json_snapshot(
        "repo_projected_retrieval_context_node_result",
        json!(result),
    );
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
