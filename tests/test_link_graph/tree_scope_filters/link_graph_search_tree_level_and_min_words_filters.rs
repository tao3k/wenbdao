use super::support::{LinkGraphScope, LinkGraphSearchFilters, LinkGraphSearchOptions};
use crate::test_link_graph::tree_scope_fixture_support::{
    TreeScopeFixture, assert_tree_scope_fixture, tree_hit_outline_snapshot,
};

#[test]
fn test_link_graph_search_tree_level_and_min_words_filters()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = TreeScopeFixture::build("tree_level_and_min_words")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            scope: Some(LinkGraphScope::SectionOnly),
            max_heading_level: Some(2),
            min_section_words: Some(4),
            per_doc_section_cap: Some(10),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let hits = index.search_planned("needle", 20, options).1;

    let actual = tree_hit_outline_snapshot(hits.as_slice());
    assert_tree_scope_fixture("tree_level_and_min_words", "result.json", &actual);
    Ok(())
}
