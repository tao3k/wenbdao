use crate::analyzers::cache::RepositoryAnalysisCacheKey;
use crate::analyzers::query::{
    DocsProjectedGapReportQuery, DocsSearchQuery, RepoProjectedGapReportQuery,
    RepoProjectedPageIndexTreesQuery, RepoProjectedPageSearchQuery, RepoProjectedPagesQuery,
    RepoProjectedRetrievalQuery,
};
use crate::analyzers::service::repository_search_artifacts;

use super::{
    build_docs_projected_gap_report, build_docs_search, build_repo_projected_gap_report,
    build_repo_projected_page_index_trees, build_repo_projected_page_search,
    build_repo_projected_page_search_with_artifacts, build_repo_projected_pages,
    build_repo_projected_retrieval,
};

#[allow(dead_code)]
#[path = "../../../../tests/support/repo_fixture.rs"]
mod repo_fixture;

#[allow(dead_code)]
#[path = "../../../../tests/support/repo_intelligence.rs"]
mod repo_test_support;

fn ok_or_panic<T, E>(result: Result<T, E>, context: &str) -> T
where
    E: std::fmt::Display,
{
    result.unwrap_or_else(|error| panic!("{context}: {error}"))
}

#[test]
fn repo_projected_pages_wraps_projection_fixture() {
    let analysis = repo_test_support::sample_projection_analysis("projection-sample");
    let result = build_repo_projected_pages(
        &RepoProjectedPagesQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    );

    assert_eq!(result.repo_id, "projection-sample");
    assert!(!result.pages.is_empty());
}

#[test]
fn repo_and_docs_gap_reports_share_the_same_surface() {
    let analysis = repo_test_support::sample_projection_analysis("projection-sample");
    let repo_result = build_repo_projected_gap_report(
        &RepoProjectedGapReportQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    );
    let docs_result = build_docs_projected_gap_report(
        &DocsProjectedGapReportQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    );

    assert_eq!(repo_result, docs_result);
}

#[test]
fn docs_and_repo_projected_search_results_match() {
    let analysis = repo_test_support::sample_projection_analysis("projection-sample");
    let repo_result = build_repo_projected_page_search(
        &RepoProjectedPageSearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: None,
            limit: 10,
        },
        &analysis,
    );
    let docs_result = build_docs_search(
        &DocsSearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: None,
            limit: 10,
        },
        &analysis,
    );

    assert_eq!(repo_result, docs_result);
    assert!(!repo_result.pages.is_empty());
}

#[test]
fn projected_page_index_trees_and_retrieval_wrap_the_fixture() {
    let analysis = repo_test_support::sample_projection_analysis("projection-sample");
    let trees = ok_or_panic(
        build_repo_projected_page_index_trees(
            &RepoProjectedPageIndexTreesQuery {
                repo_id: "projection-sample".to_string(),
            },
            &analysis,
        ),
        "fixture should parse into projected page-index trees",
    );
    let retrieval = build_repo_projected_retrieval(
        &RepoProjectedRetrievalQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: None,
            limit: 10,
        },
        &analysis,
    );

    assert_eq!(trees.repo_id, "projection-sample");
    assert!(!trees.trees.is_empty());
    assert_eq!(retrieval.repo_id, "projection-sample");
    assert!(!retrieval.hits.is_empty());
}

#[test]
fn projected_page_search_with_artifacts_matches_direct_search() {
    let analysis = repo_test_support::sample_projection_analysis("projection-artifacts");
    let query = RepoProjectedPageSearchQuery {
        repo_id: "projection-artifacts".to_string(),
        query: "solve".to_string(),
        kind: None,
        limit: 10,
    };
    let artifacts = ok_or_panic(
        repository_search_artifacts(
            &RepositoryAnalysisCacheKey {
                repo_id: "projection-artifacts".to_string(),
                checkout_root: "/virtual/repos/projection-artifacts".to_string(),
                checkout_revision: Some("fixture".to_string()),
                mirror_revision: Some("fixture".to_string()),
                tracking_revision: Some("fixture".to_string()),
                plugin_ids: vec!["fixture-plugin".to_string()],
            },
            &analysis,
        ),
        "projection artifacts should build",
    );

    assert_eq!(
        build_repo_projected_page_search(&query, &analysis),
        build_repo_projected_page_search_with_artifacts(&query, &analysis, artifacts.as_ref())
    );
}
