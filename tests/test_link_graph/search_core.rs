use super::*;

#[test]
fn test_link_graph_build_search_and_stats() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchCoreFixture::build("baseline")?;
    let index = fixture.build_index()?;

    let actual = stats_and_hits_snapshot(
        index.stats(),
        index
            .search_planned("beta", 5, LinkGraphSearchOptions::default())
            .1
            .as_slice(),
    );
    assert_search_core_fixture("baseline", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_search_limit_is_enforced() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchCoreFixture::build("limit")?;
    let index = fixture.build_index()?;
    let hits = index
        .search_planned("shared keyword", 2, LinkGraphSearchOptions::default())
        .1;

    let actual = hits_snapshot(hits.as_slice());
    assert_search_core_fixture("limit", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_search_short_circuits_id_directive() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchCoreFixture::build("direct_id")?;
    let index = fixture.build_index()?;
    let (parsed, hits) = index.search_planned(
        "id:docs/b this phrase should not be required",
        5,
        LinkGraphSearchOptions::default(),
    );

    let actual = direct_id_snapshot(&parsed, hits.as_slice());
    assert_search_core_fixture("direct_id", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_search_payload_short_circuits_id_directive()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchCoreFixture::build("direct_id_payload")?;
    let index = fixture.build_index()?;
    let payload =
        index.search_planned_payload("id:notes/release", 5, LinkGraphSearchOptions::default());

    let actual = planned_payload_snapshot(&payload);
    assert_search_core_fixture("direct_id_payload", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_search_fts_boosts_high_reference_notes() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = SearchCoreFixture::build("fts_boost")?;
    let index = fixture.build_index()?;
    let hits = index
        .search_planned("shared phrase", 5, LinkGraphSearchOptions::default())
        .1;

    let actual = hits_snapshot(hits.as_slice());
    assert_search_core_fixture("fts_boost", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_search_fts_prefers_phrase_specific_note_over_generic_index()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchCoreFixture::build("phrase_specific")?;
    let index = fixture.build_index()?;
    let hits = index
        .search_planned("checkpoint schema", 5, LinkGraphSearchOptions::default())
        .1;

    let actual = hits_snapshot(hits.as_slice());
    assert_search_core_fixture("phrase_specific", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_search_sort_by_path() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchCoreFixture::build("sort_path")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        match_strategy: LinkGraphMatchStrategy::Fts,
        case_sensitive: false,
        sort_terms: vec![sort_term(LinkGraphSortField::Path, LinkGraphSortOrder::Asc)],
        ..LinkGraphSearchOptions::default()
    };
    let hits = index.search_planned(".md", 5, options).1;

    let actual = hits_snapshot(hits.as_slice());
    assert_search_core_fixture("sort_path", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_search_planned_payload_has_consistent_counts()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchCoreFixture::build("payload_counts")?;
    let index = fixture.build_index()?;
    let payload =
        index.search_planned_payload("architecture graph", 10, LinkGraphSearchOptions::default());

    let actual = planned_payload_snapshot(&payload);
    assert_search_core_fixture("payload_counts", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_search_planned_payload_escalates_when_graph_hits_are_empty()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchCoreFixture::build("payload_empty_graph")?;
    let index = fixture.build_index()?;
    let payload = index.search_planned_payload(
        "missing-term-never-hit",
        5,
        LinkGraphSearchOptions::default(),
    );

    let actual = planned_payload_snapshot(&payload);
    assert_search_core_fixture("payload_empty_graph", "result.json", &actual);
    Ok(())
}
