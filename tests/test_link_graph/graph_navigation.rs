use super::*;

#[test]
fn test_link_graph_neighbors_related_metadata_and_toc() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = NavigationFixture::build("neighbors_related_metadata_and_toc")?;
    let index = LinkGraphIndex::build(&fixture.path("root")).map_err(|e| e.clone())?;

    let neighbors = index.neighbors("a", LinkGraphDirection::Both, 1, 10);
    let related = index.related("a", 2, 10);
    let metadata = index.metadata("b").ok_or("missing metadata")?;
    let toc = index.toc(10);

    let actual = navigation_surface_snapshot(&neighbors, &related, &metadata, &toc);
    assert_graph_navigation_fixture("neighbors_related_metadata_and_toc", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_related_with_diagnostics_returns_metrics()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = NavigationFixture::build("related_with_diagnostics")?;
    let index = LinkGraphIndex::build(&fixture.path("root")).map_err(|e| e.clone())?;

    let ppr = LinkGraphRelatedPprOptions {
        alpha: Some(0.9),
        max_iter: Some(64),
        tol: Some(1e-6),
        subgraph_mode: Some(LinkGraphPprSubgraphMode::Force),
    };
    let (rows, diagnostics) = index.related_with_diagnostics("b", 2, 10, Some(&ppr));

    let actual = related_diagnostics_snapshot(&rows, diagnostics);
    assert_graph_navigation_fixture("related_with_diagnostics", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_related_from_seeds_with_diagnostics_partitions_when_forced()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = NavigationFixture::build("related_from_seeds_with_diagnostics")?;
    let index = LinkGraphIndex::build(&fixture.path("root")).map_err(|e| e.clone())?;

    let seeds = vec!["b".to_string(), "e".to_string()];
    let ppr = LinkGraphRelatedPprOptions {
        alpha: Some(0.85),
        max_iter: Some(48),
        tol: Some(1e-6),
        subgraph_mode: Some(LinkGraphPprSubgraphMode::Force),
    };
    let (rows, diagnostics) = index.related_from_seeds_with_diagnostics(&seeds, 2, 20, Some(&ppr));

    let actual = related_diagnostics_snapshot(&rows, diagnostics);
    assert_graph_navigation_fixture(
        "related_from_seeds_with_diagnostics",
        "result.json",
        &actual,
    );
    Ok(())
}
