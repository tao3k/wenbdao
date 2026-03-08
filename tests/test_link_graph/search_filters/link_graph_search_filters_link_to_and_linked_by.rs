use super::support::{
    LinkGraphLinkFilter, LinkGraphSearchFilters, LinkGraphSearchOptions, LinkGraphSortField,
    LinkGraphSortOrder, json, sort_term,
};
use crate::test_link_graph::search_filters_fixture_support::{
    SearchFilterFixture, assert_search_filter_fixture, ordered_hit_paths,
};

#[test]
fn test_link_graph_search_filters_link_to_and_linked_by() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = SearchFilterFixture::build("link_to_and_linked_by")?;
    let index = fixture.build_index()?;

    let link_to_options = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        filters: LinkGraphSearchFilters {
            link_to: Some(LinkGraphLinkFilter {
                seeds: vec!["b".to_string()],
                ..LinkGraphLinkFilter::default()
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let linked_by_options = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        filters: LinkGraphSearchFilters {
            linked_by: Some(LinkGraphLinkFilter {
                seeds: vec!["b".to_string()],
                ..LinkGraphLinkFilter::default()
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };

    let actual = json!({
        "link_to": ordered_hit_paths(index.search_planned("", 10, link_to_options).1.as_slice()),
        "linked_by": ordered_hit_paths(index.search_planned("", 10, linked_by_options).1.as_slice()),
    });
    assert_search_filter_fixture("link_to_and_linked_by", "result.json", &actual);
    Ok(())
}
