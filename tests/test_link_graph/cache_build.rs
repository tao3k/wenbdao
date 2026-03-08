use super::cache_build_fixture_support::{
    CacheBuildFixture, assert_cache_build_fixture, cache_hits_snapshot, cache_stats_snapshot,
    read_cache_build_fixture, saliency_state_snapshot,
};
use super::support::{clear_cache_keys, count_cache_keys, unique_cache_prefix, write_file};
use serde_json::json;
use xiuxian_wendao::link_graph::{LinkGraphIndex, LinkGraphSearchOptions};
use xiuxian_wendao::{
    LinkGraphSaliencyPolicy, compute_link_graph_saliency, valkey_saliency_get_with_valkey,
};

#[test]
fn test_link_graph_build_with_cache_reuses_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let prefix = unique_cache_prefix();
    clear_cache_keys(&prefix)?;

    let fixture = CacheBuildFixture::build("reuses_snapshot")?;
    let index1 = LinkGraphIndex::build_with_cache_with_valkey(
        fixture.root(),
        &[],
        &[],
        "redis://127.0.0.1:6379/0",
        Some(&prefix),
        Some(300),
    )
    .map_err(|e| e.clone())?;
    let index2 = LinkGraphIndex::build_with_cache_with_valkey(
        fixture.root(),
        &[],
        &[],
        "redis://127.0.0.1:6379/0",
        Some(&prefix),
        Some(300),
    )
    .map_err(|e| e.clone())?;

    let actual = json!({
        "index1_stats": cache_stats_snapshot(&index1),
        "index2_stats": cache_stats_snapshot(&index2),
        "same_total_notes": index1.stats().total_notes == index2.stats().total_notes,
        "same_links_in_graph": index1.stats().links_in_graph == index2.stats().links_in_graph,
        "has_cache_keys": count_cache_keys(&prefix)? >= 1,
    });
    assert_cache_build_fixture("reuses_snapshot", "result.json", &actual);
    clear_cache_keys(&prefix)?;
    Ok(())
}

#[test]
fn test_link_graph_build_with_cache_detects_file_change() -> Result<(), Box<dyn std::error::Error>>
{
    let prefix = unique_cache_prefix();
    clear_cache_keys(&prefix)?;

    let fixture = CacheBuildFixture::build("detects_file_change")?;
    let path = fixture.path("docs/a.md");

    let _ = LinkGraphIndex::build_with_cache_with_valkey(
        fixture.root(),
        &[],
        &[],
        "redis://127.0.0.1:6379/0",
        Some(&prefix),
        Some(300),
    )
    .map_err(|e| e.clone())?;

    write_file(
        path.as_path(),
        read_cache_build_fixture("detects_file_change", "update/docs/a.md").as_str(),
    )?;

    let refreshed = LinkGraphIndex::build_with_cache_with_valkey(
        fixture.root(),
        &[],
        &[],
        "redis://127.0.0.1:6379/0",
        Some(&prefix),
        Some(300),
    )
    .map_err(|e| e.clone())?;
    let hits = refreshed
        .search_planned(
            "updated phrase for cache invalidation",
            5,
            LinkGraphSearchOptions::default(),
        )
        .1;

    let actual = json!({
        "hits": cache_hits_snapshot(hits.as_slice()),
        "stats": cache_stats_snapshot(&refreshed),
    });
    assert_cache_build_fixture("detects_file_change", "result.json", &actual);
    clear_cache_keys(&prefix)?;
    Ok(())
}

#[test]
fn test_link_graph_build_with_cache_seeds_saliency_from_frontmatter()
-> Result<(), Box<dyn std::error::Error>> {
    let prefix = unique_cache_prefix();
    clear_cache_keys(&prefix)?;

    let fixture = CacheBuildFixture::build("seeds_saliency")?;
    let _index = LinkGraphIndex::build_with_cache_with_valkey(
        fixture.root(),
        &[],
        &[],
        "redis://127.0.0.1:6379/0",
        Some(&prefix),
        Some(300),
    )
    .map_err(|e| e.clone())?;

    let state =
        valkey_saliency_get_with_valkey("docs/a", "redis://127.0.0.1:6379/0", Some(&prefix))
            .map_err(|e| e.clone())?;
    let seeded = state.ok_or("missing seeded saliency state for docs/a")?;
    let expected =
        compute_link_graph_saliency(9.0, 0.2, 0, 0.0, LinkGraphSaliencyPolicy::default());

    let actual = saliency_state_snapshot(&seeded, expected);
    assert_cache_build_fixture("seeds_saliency", "result.json", &actual);

    clear_cache_keys(&prefix)?;
    Ok(())
}
