use super::*;

#[test]
fn test_link_graph_search_edge_type_filter_restricts_structural_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = TreeScopeFixture::build("semantic_edges_restrict_section_scope")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            scope: Some(LinkGraphScope::SectionOnly),
            edge_types: vec![LinkGraphEdgeType::Semantic],
            min_section_words: Some(0),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let hits = index.search_planned("alpha", 10, options).1;

    let actual = tree_hit_outline_snapshot(hits.as_slice());
    assert_tree_scope_fixture(
        "semantic_edges_restrict_section_scope",
        "result.json",
        &actual,
    );
    Ok(())
}
