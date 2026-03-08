use super::*;

#[test]
fn test_link_graph_search_edge_type_filter_restricts_semantic_graph_filters()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = TreeScopeFixture::build("structural_edges_restrict_graph_filters")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            link_to: Some(LinkGraphLinkFilter {
                seeds: vec!["b".to_string()],
                ..LinkGraphLinkFilter::default()
            }),
            edge_types: vec![LinkGraphEdgeType::Structural],
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let hits = index.search_planned("", 10, options).1;

    let actual = tree_hit_outline_snapshot(hits.as_slice());
    assert_tree_scope_fixture(
        "structural_edges_restrict_graph_filters",
        "result.json",
        &actual,
    );
    Ok(())
}
