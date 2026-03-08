use super::*;

#[test]
fn test_link_graph_search_section_scope_respects_per_doc_cap()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = TreeScopeFixture::build("section_scope_per_doc_cap")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            scope: Some(LinkGraphScope::SectionOnly),
            per_doc_section_cap: Some(1),
            min_section_words: Some(0),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let hits = index.search_planned("alpha marker", 20, options).1;

    let actual = json!({
        "hits": tree_hit_outline_snapshot(hits.as_slice()),
        "per_path": per_path_counts_snapshot(hits.as_slice()),
    });
    assert_tree_scope_fixture("section_scope_per_doc_cap", "result.json", &actual);
    Ok(())
}
