use super::support::{
    LinkGraphEdgeType, LinkGraphLinkFilter, LinkGraphSearchFilters, LinkGraphSearchOptions, json,
};
use crate::test_link_graph::tree_scope_fixture_support::{
    TreeScopeFixture, assert_tree_scope_fixture,
};

#[test]
fn test_link_graph_search_edge_type_filter_allows_verified_for_graph_filters()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = TreeScopeFixture::build("verified_edges_keep_graph_filters")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            link_to: Some(LinkGraphLinkFilter {
                seeds: vec!["b".to_string()],
                ..LinkGraphLinkFilter::default()
            }),
            edge_types: vec![LinkGraphEdgeType::Verified],
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let hits = index.search_planned("", 10, options).1;

    let actual = json!({
        "paths": hits.iter().map(|hit| hit.path.clone()).collect::<Vec<_>>(),
    });
    assert_tree_scope_fixture("verified_edges_keep_graph_filters", "result.json", &actual);
    Ok(())
}
