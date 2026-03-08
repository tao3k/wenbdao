use super::support::{LinkGraphScope, LinkGraphSearchFilters, LinkGraphSearchOptions, json};
use crate::test_link_graph::tree_scope_fixture_support::{
    TreeScopeFixture, assert_tree_scope_fixture, ordered_section_labels,
};

#[test]
fn test_link_graph_search_tree_hops_limit_section_expansion()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = TreeScopeFixture::build("tree_hops_limit_section_expansion")?;
    let index = fixture.build_index()?;

    let base = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            scope: Some(LinkGraphScope::SectionOnly),
            per_doc_section_cap: Some(10),
            min_section_words: Some(0),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let hops_zero = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            max_tree_hops: Some(0),
            ..base.filters.clone()
        },
        ..base.clone()
    };
    let hops_one = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            max_tree_hops: Some(1),
            ..base.filters.clone()
        },
        ..base
    };

    let hits_zero = index.search_planned("needle focus", 20, hops_zero).1;
    let hits_one = index.search_planned("needle focus", 20, hops_one).1;

    let actual = json!({
        "hops_zero": ordered_section_labels(hits_zero.as_slice()),
        "hops_one": ordered_section_labels(hits_one.as_slice()),
    });
    assert_tree_scope_fixture("tree_hops_limit_section_expansion", "result.json", &actual);
    Ok(())
}
