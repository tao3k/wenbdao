//! Integration tests for LinkGraph page-index build and lineage retrieval.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;
#[path = "support/link_graph_fixture_tree.rs"]
mod link_graph_fixture_tree;
#[path = "support/page_index_fixture_support.rs"]
mod page_index_fixture_support;

use page_index_fixture_support::{
    PageIndexFixture, assert_page_index_fixture, page_index_tree_snapshot, read_page_index_fixture,
    semantic_documents_snapshot,
};
use serde_json::json;

#[test]
fn test_link_graph_page_index_builds_hierarchy_and_line_ranges()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = PageIndexFixture::build("hierarchy")?;
    let index = fixture.build_index()?;
    let roots = index.page_index("alpha").ok_or("missing page index")?;

    let actual = page_index_tree_snapshot(roots);
    assert_page_index_fixture("hierarchy", "tree.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_page_index_generates_single_root_for_headingless_docs()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = PageIndexFixture::build("headingless")?;
    let index = fixture.build_index()?;
    let roots = index.page_index("plain").ok_or("missing page index")?;

    let actual = page_index_tree_snapshot(roots);
    assert_page_index_fixture("headingless", "tree.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_page_index_thins_small_parent_sections() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = PageIndexFixture::build("thinning")?;
    let index = fixture.build_index()?;
    let roots = index.page_index("thin").ok_or("missing page index")?;

    let actual = page_index_tree_snapshot(roots);
    assert_page_index_fixture("thinning", "tree.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_page_index_refresh_updates_incremental_tree()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = PageIndexFixture::build("refresh")?;
    let path = fixture.path("docs/refresh.md");
    let mut index = fixture.build_index()?;
    let initial = index
        .page_index("refresh")
        .ok_or("missing initial page index")?;
    let initial_snapshot = page_index_tree_snapshot(initial);

    std::fs::write(
        path.as_path(),
        read_page_index_fixture("refresh", "update/docs/refresh.md"),
    )?;
    index.refresh_incremental(std::slice::from_ref(&path))?;

    let refreshed = index
        .page_index("refresh")
        .ok_or("missing refreshed page index")?;
    let actual = serde_json::json!({
        "initial": initial_snapshot,
        "refreshed": page_index_tree_snapshot(refreshed),
    });
    assert_page_index_fixture("refresh", "incremental_refresh.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_page_index_exports_semantic_documents() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = PageIndexFixture::build("semantic_documents")?;
    let index = fixture.build_index()?;
    let documents = index
        .semantic_documents_for("alpha")
        .ok_or("missing semantic documents")?;

    let actual = semantic_documents_snapshot(documents.as_slice());
    assert_page_index_fixture("semantic_documents", "documents.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_page_index_retrieves_parent_chain_for_nested_anchor()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = PageIndexFixture::build("hierarchy")?;
    let index = fixture.build_index()?;
    let roots = index.page_index("alpha").ok_or("missing page index")?;
    let beta = roots
        .first()
        .and_then(|root| root.children.first())
        .ok_or("missing beta node")?;
    let gamma = beta.children.first().ok_or("missing gamma node")?;

    assert_eq!(
        index.page_index_semantic_path(gamma.node_id.as_str()),
        Some(vec![
            "Alpha".to_string(),
            "Beta".to_string(),
            "Gamma".to_string(),
        ])
    );
    assert_eq!(
        index.page_index_trace_label(gamma.node_id.as_str()),
        Some("[Path: Alpha > Beta > Gamma]".to_string())
    );
    Ok(())
}

#[test]
fn test_link_graph_page_index_parent_chain_exposes_parent_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = PageIndexFixture::build("hierarchy")?;
    let index = fixture.build_index()?;
    let roots = index.page_index("alpha").ok_or("missing page index")?;
    let root = roots.first().ok_or("missing root node")?;
    let beta = root.children.first().ok_or("missing beta node")?;
    let gamma = beta.children.first().ok_or("missing gamma node")?;

    let actual = json!({
        "root": parent_record(&index, root.node_id.as_str()),
        "beta": parent_record(&index, beta.node_id.as_str()),
        "gamma": parent_record(&index, gamma.node_id.as_str()),
    });
    assert_page_index_fixture("hierarchy", "parent_chain.json", &actual);
    Ok(())
}

fn parent_record(index: &xiuxian_wendao::LinkGraphIndex, node_id: &str) -> serde_json::Value {
    match index.page_index_parent_id(node_id) {
        Some(parent_id) => json!({
            "node_id": node_id,
            "parent_id": parent_id,
            "known": true,
        }),
        None => json!({
            "node_id": node_id,
            "parent_id": null,
            "known": false,
        }),
    }
}
