//! Integration tests for deterministic projected page-index tree search.

use crate::support::repo_intelligence::{assert_repo_json_snapshot, sample_projection_analysis};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    ProjectionPageKind, RepoProjectedPageIndexTreeSearchQuery,
    build_repo_projected_page_index_tree_search,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_index_tree_search_matches_section_hits() -> TestResult {
    let analysis = sample_projection_analysis("projection-sample");

    let result = build_repo_projected_page_index_tree_search(
        &RepoProjectedPageIndexTreeSearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "anchors".to_string(),
            kind: Some(ProjectionPageKind::Reference),
            limit: 10,
        },
        &analysis,
    );

    assert_repo_json_snapshot(
        "repo_projected_page_index_tree_search_result",
        json!(result),
    );
    Ok(())
}
