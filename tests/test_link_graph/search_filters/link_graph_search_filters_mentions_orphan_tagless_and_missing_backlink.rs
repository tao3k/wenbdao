use super::*;

#[test]
fn test_link_graph_search_filters_mentions_orphan_tagless_and_missing_backlink()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchFilterFixture::build("mentions_orphan_tagless_missing_backlink")?;
    let index = fixture.build_index()?;

    let mentions_options = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        filters: LinkGraphSearchFilters {
            mentions_of: vec!["alpha signal".to_string()],
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let mentioned_by_options = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        filters: LinkGraphSearchFilters {
            mentioned_by_notes: vec!["a".to_string()],
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let orphan_options = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        filters: LinkGraphSearchFilters {
            orphan: true,
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let tagless_options = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        filters: LinkGraphSearchFilters {
            tagless: true,
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let missing_backlink_options = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        filters: LinkGraphSearchFilters {
            missing_backlink: true,
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };

    let actual = json!({
        "mentions": ordered_hit_paths(index.search_planned("", 10, mentions_options).1.as_slice()),
        "mentioned_by": ordered_hit_paths(index.search_planned("", 10, mentioned_by_options).1.as_slice()),
        "orphan": ordered_hit_paths(index.search_planned("", 10, orphan_options).1.as_slice()),
        "tagless": ordered_hit_paths(index.search_planned("", 10, tagless_options).1.as_slice()),
        "missing_backlink": ordered_hit_paths(index.search_planned("", 10, missing_backlink_options).1.as_slice()),
    });
    assert_search_filter_fixture(
        "mentions_orphan_tagless_missing_backlink",
        "result.json",
        &actual,
    );
    Ok(())
}
