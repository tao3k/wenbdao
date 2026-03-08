use super::*;

#[test]
fn test_link_graph_search_filters_related_with_distance() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = SearchFilterFixture::build("related_with_distance")?;
    let index = fixture.build_index()?;

    let options_distance_1 = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        filters: LinkGraphSearchFilters {
            related: Some(LinkGraphRelatedFilter {
                seeds: vec!["b".to_string()],
                max_distance: Some(1),
                ppr: None,
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let options_distance_2 = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        filters: LinkGraphSearchFilters {
            related: Some(LinkGraphRelatedFilter {
                seeds: vec!["b".to_string()],
                max_distance: Some(2),
                ppr: None,
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };

    let actual = json!({
        "distance_1": ordered_hit_paths(index.search_planned("", 10, options_distance_1).1.as_slice()),
        "distance_2": ordered_hit_paths(index.search_planned("", 10, options_distance_2).1.as_slice()),
    });
    assert_search_filter_fixture("related_with_distance", "result.json", &actual);
    Ok(())
}
