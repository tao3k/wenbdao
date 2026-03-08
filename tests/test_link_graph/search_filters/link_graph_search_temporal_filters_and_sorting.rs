use super::support::{
    LinkGraphSearchOptions, LinkGraphSortField, LinkGraphSortOrder, json, sort_term,
};
use crate::test_link_graph::search_filters_fixture_support::{
    SearchFilterFixture, assert_search_filter_fixture, ordered_hit_paths,
};

#[test]
fn test_link_graph_search_temporal_filters_and_sorting() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchFilterFixture::build("temporal_filters_and_sorting")?;
    let index = fixture.build_index()?;

    let created_window = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(
            LinkGraphSortField::Created,
            LinkGraphSortOrder::Asc,
        )],
        created_after: Some(1_704_153_600),
        created_before: Some(1_704_758_400),
        ..LinkGraphSearchOptions::default()
    };
    let modified_sorted = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(
            LinkGraphSortField::Modified,
            LinkGraphSortOrder::Desc,
        )],
        modified_after: Some(1_704_153_600),
        ..LinkGraphSearchOptions::default()
    };

    let actual = json!({
        "created_window": ordered_hit_paths(index.search_planned("", 10, created_window).1.as_slice()),
        "modified_sorted": index
            .search_planned("", 10, modified_sorted)
            .1
            .iter()
            .map(|hit| hit.path.clone())
            .collect::<Vec<_>>(),
    });
    assert_search_filter_fixture("temporal_filters_and_sorting", "result.json", &actual);
    Ok(())
}
