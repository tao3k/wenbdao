use super::support::{LinkGraphScope, LinkGraphSearchFilters, LinkGraphSearchOptions, json};
use crate::test_link_graph::tree_scope_fixture_support::{
    TreeScopeFixture, assert_tree_scope_fixture, per_path_counts_snapshot,
    tree_hit_outline_snapshot,
};

#[test]
fn test_link_graph_search_mixed_scope_collapse_toggle_changes_output_shape()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = TreeScopeFixture::build("mixed_scope_collapse_toggle")?;
    let index = fixture.build_index()?;

    let collapse_true = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            scope: Some(LinkGraphScope::Mixed),
            collapse_to_doc: Some(true),
            per_doc_section_cap: Some(3),
            min_section_words: Some(0),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let collapse_false = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            scope: Some(LinkGraphScope::Mixed),
            collapse_to_doc: Some(false),
            per_doc_section_cap: Some(3),
            min_section_words: Some(0),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let collapsed_hits = index.search_planned("alpha context", 20, collapse_true).1;
    let expanded_hits = index.search_planned("alpha context", 20, collapse_false).1;

    let actual = json!({
        "collapsed": per_path_counts_snapshot(collapsed_hits.as_slice()),
        "expanded": per_path_counts_snapshot(expanded_hits.as_slice()),
        "expanded_hits": tree_hit_outline_snapshot(expanded_hits.as_slice()),
    });
    assert_tree_scope_fixture("mixed_scope_collapse_toggle", "result.json", &actual);
    Ok(())
}
