use super::*;

#[test]
fn test_link_graph_parse_search_query_supports_path_fuzzy_strategy() {
    let parsed = parse_search_query(
        "match:path_fuzzy architecture graph",
        LinkGraphSearchOptions::default(),
    );

    let actual = parsed_query_snapshot(&parsed);
    assert_search_match_fixture("parse_path_fuzzy", "result.json", &actual);
}

#[test]
fn test_link_graph_search_path_fuzzy_prefers_path_and_section()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchMatchFixture::build("path_fuzzy_prefers_path_and_section")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        match_strategy: LinkGraphMatchStrategy::PathFuzzy,
        case_sensitive: false,
        ..LinkGraphSearchOptions::default()
    };
    let hits = index
        .search_planned("architecture graph engine", 5, options)
        .1;

    let actual = hits_outline_snapshot(hits.as_slice());
    assert_search_match_fixture(
        "path_fuzzy_prefers_path_and_section",
        "result.json",
        &actual,
    );
    Ok(())
}

#[test]
fn test_link_graph_search_path_fuzzy_ignores_fenced_headings()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchMatchFixture::build("path_fuzzy_ignores_fenced_headings")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        match_strategy: LinkGraphMatchStrategy::PathFuzzy,
        case_sensitive: false,
        ..LinkGraphSearchOptions::default()
    };
    let hits = index
        .search_planned("architecture real heading graph", 5, options)
        .1;

    let actual = hits_outline_snapshot(hits.as_slice());
    assert_search_match_fixture("path_fuzzy_ignores_fenced_headings", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_search_path_fuzzy_handles_duplicate_headings()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchMatchFixture::build("path_fuzzy_handles_duplicate_headings")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        match_strategy: LinkGraphMatchStrategy::PathFuzzy,
        case_sensitive: false,
        ..LinkGraphSearchOptions::default()
    };
    let hits = index
        .search_planned("architecture api router", 5, options)
        .1;

    let actual = hits_outline_snapshot(hits.as_slice());
    assert_search_match_fixture(
        "path_fuzzy_handles_duplicate_headings",
        "result.json",
        &actual,
    );
    Ok(())
}

#[test]
fn test_link_graph_search_with_exact_strategy() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchMatchFixture::build("exact_strategy")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        match_strategy: LinkGraphMatchStrategy::Exact,
        case_sensitive: false,
        ..LinkGraphSearchOptions::default()
    };
    let hits = index.search_planned("rust tokenizer", 5, options).1;

    let actual = hits_outline_snapshot(hits.as_slice());
    assert_search_match_fixture("exact_strategy", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_search_with_regex_strategy() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchMatchFixture::build("regex_strategy")?;
    let index = fixture.build_index()?;

    let options = LinkGraphSearchOptions {
        match_strategy: LinkGraphMatchStrategy::Re,
        case_sensitive: false,
        ..LinkGraphSearchOptions::default()
    };
    let hits = index.search_planned("^beta", 5, options).1;

    let actual = hits_outline_snapshot(hits.as_slice());
    assert_search_match_fixture("regex_strategy", "result.json", &actual);
    Ok(())
}
