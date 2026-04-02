//! Integration tests for deterministic projected-page search.

use crate::support::repo_intelligence::{assert_repo_json_snapshot, sample_projection_analysis};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    ProjectionPageKind, RepoProjectedPageSearchQuery, build_repo_projected_page_search,
};

#[test]
fn projected_page_search_matches_reference_pages() {
    let analysis = sample_projection_analysis("projection-sample");
    let result = build_repo_projected_page_search(
        &RepoProjectedPageSearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: Some(ProjectionPageKind::Reference),
            limit: 10,
        },
        &analysis,
    );

    assert_repo_json_snapshot("repo_projected_page_search_result", json!(result));
}
