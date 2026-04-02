//! Integration tests for deterministic projected-page lookup.

use std::fs;

use crate::support::repo_intelligence::{
    assert_repo_json_snapshot, create_sample_modelica_repo, sample_projection_analysis,
};
use serde_json::json;
use xiuxian_wendao::analyzers::{
    RepoProjectedPageQuery, RepoProjectedPagesQuery, build_repo_projected_page,
    build_repo_projected_pages, repo_projected_page_from_config, repo_projected_pages_from_config,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn projected_page_lookup_resolves_one_stable_page() -> TestResult {
    let analysis = sample_projection_analysis("projection-sample");

    let pages = build_repo_projected_pages(
        &RepoProjectedPagesQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    );

    let page_id = pages
        .pages
        .iter()
        .find(|page| page.title == "solve")
        .map(|page| page.page_id.clone())
        .expect("expected a projected page titled `solve`");

    let result = build_repo_projected_page(
        &RepoProjectedPageQuery {
            repo_id: "projection-sample".to_string(),
            page_id,
        },
        &analysis,
    )?;

    assert_repo_json_snapshot("repo_projected_page_result", json!(result));
    Ok(())
}

#[cfg(feature = "modelica")]
#[test]
fn modelica_plugin_projected_page_lookup_resolves_one_stable_page() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_sample_modelica_repo(temp.path(), "Projectionica")?;
    let config_path = temp.path().join("modelica-projected-page.wendao.toml");
    fs::write(
        &config_path,
        format!(
            r#"[link_graph.projects.modelica-projected-page]
root = "{}"
plugins = ["modelica"]
"#,
            repo_dir.display()
        ),
    )?;

    let pages = repo_projected_pages_from_config(
        &RepoProjectedPagesQuery {
            repo_id: "modelica-projected-page".to_string(),
        },
        Some(&config_path),
        temp.path(),
    )?;
    let page_id = pages
        .pages
        .iter()
        .find(|page| {
            page.page_id.contains(":symbol:") && page.title == "Projectionica.Controllers.PI"
        })
        .map(|page| page.page_id.clone())
        .expect("expected a projected page for Projectionica.Controllers.PI");

    let result = repo_projected_page_from_config(
        &RepoProjectedPageQuery {
            repo_id: "modelica-projected-page".to_string(),
            page_id,
        },
        Some(&config_path),
        temp.path(),
    )?;

    assert_repo_json_snapshot("repo_projected_page_modelica_result", json!(result));
    Ok(())
}
