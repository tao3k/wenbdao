use super::support::{
    LinkGraphPprSubgraphMode, LinkGraphRelatedFilter, LinkGraphRelatedPprOptions,
    LinkGraphSearchFilters, LinkGraphSearchOptions, LinkGraphSortField, LinkGraphSortOrder, json,
    sort_term,
};
use crate::test_link_graph::search_filters_fixture_support::{
    SearchFilterFixture, assert_search_filter_fixture, ordered_hit_paths,
};

#[test]
fn test_link_graph_search_filters_related_accepts_ppr_options()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchFilterFixture::build("related_accepts_ppr_options")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        filters: LinkGraphSearchFilters {
            related: Some(LinkGraphRelatedFilter {
                seeds: vec!["b".to_string()],
                max_distance: Some(2),
                ppr: Some(LinkGraphRelatedPprOptions {
                    alpha: Some(0.9),
                    max_iter: Some(64),
                    tol: Some(1e-6),
                    subgraph_mode: Some(LinkGraphPprSubgraphMode::Force),
                }),
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };

    let actual = json!({
        "paths": ordered_hit_paths(index.search_planned("", 10, options).1.as_slice()),
    });
    assert_search_filter_fixture("related_accepts_ppr_options", "result.json", &actual);
    Ok(())
}
