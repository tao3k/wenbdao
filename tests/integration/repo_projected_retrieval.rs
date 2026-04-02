//! Integration tests for deterministic mixed projected retrieval.

use crate::support::repo_intelligence::{assert_repo_json_snapshot, sample_projection_analysis};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    ProjectionPageKind, RepoProjectedRetrievalQuery, build_repo_projected_retrieval,
};

#[test]
fn projected_retrieval_merges_page_and_node_hits() {
    let analysis = sample_projection_analysis("projection-sample");
    let result = build_repo_projected_retrieval(
        &RepoProjectedRetrievalQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: Some(ProjectionPageKind::Reference),
            limit: 10,
        },
        &analysis,
    );

    assert_repo_json_snapshot("repo_projected_retrieval_result", json!(result));
}
