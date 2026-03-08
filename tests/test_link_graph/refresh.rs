use super::refresh_fixture_support::{
    RefreshFixture, assert_refresh_fixture, read_refresh_fixture, refresh_hits_snapshot,
    refresh_mode_label, refresh_sequence_snapshot, stats_snapshot,
};
use super::support::write_file;
use serde_json::json;
use std::fs;
use xiuxian_wendao::link_graph::{LinkGraphIndex, LinkGraphSearchOptions};

#[test]
fn test_link_graph_refresh_incremental_updates_and_deletes_notes()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = RefreshFixture::build("incremental_update_and_delete")?;
    let b_path = fixture.path("docs/b.md");

    let mut index = LinkGraphIndex::build(fixture.root()).map_err(|e| e.clone())?;
    let old_hits = index
        .search_planned("old keyword", 5, LinkGraphSearchOptions::default())
        .1;

    write_file(
        &b_path,
        read_refresh_fixture("incremental_update_and_delete", "update/docs/b.md").as_str(),
    )?;
    let update_mode = index
        .refresh_incremental_with_threshold(std::slice::from_ref(&b_path), 256)
        .map_err(|e| e.clone())?;
    let new_hits = index
        .search_planned("new keyword", 5, LinkGraphSearchOptions::default())
        .1;

    fs::remove_file(&b_path)?;
    let delete_mode = index
        .refresh_incremental_with_threshold(std::slice::from_ref(&b_path), 256)
        .map_err(|e| e.clone())?;

    let actual = refresh_sequence_snapshot(
        old_hits.as_slice(),
        update_mode,
        new_hits.as_slice(),
        delete_mode,
        index.stats(),
    );
    assert_refresh_fixture("incremental_update_and_delete", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_refresh_incremental_with_threshold_modes()
-> Result<(), Box<dyn std::error::Error>> {
    let full_fixture = RefreshFixture::build("threshold_modes")?;
    let full_a_path = full_fixture.path("docs/a.md");

    let mut full_index = LinkGraphIndex::build(full_fixture.root()).map_err(|e| e.clone())?;
    let noop = full_index
        .refresh_incremental_with_threshold(&[], 1)
        .map_err(|e| e.clone())?;

    write_file(
        &full_a_path,
        read_refresh_fixture("threshold_modes", "update/docs/a.md").as_str(),
    )?;
    let full = full_index
        .refresh_incremental_with_threshold(std::slice::from_ref(&full_a_path), 1)
        .map_err(|e| e.clone())?;
    let full_hits = full_index
        .search_planned("new token", 5, LinkGraphSearchOptions::default())
        .1;

    let delta_fixture = RefreshFixture::build("threshold_modes")?;
    let delta_a_path = delta_fixture.path("docs/a.md");

    let mut delta_index = LinkGraphIndex::build(delta_fixture.root()).map_err(|e| e.clone())?;
    write_file(
        &delta_a_path,
        read_refresh_fixture("threshold_modes", "update/docs/a.md").as_str(),
    )?;
    let delta = delta_index
        .refresh_incremental_with_threshold(std::slice::from_ref(&delta_a_path), 256)
        .map_err(|e| e.clone())?;
    let delta_hits = delta_index
        .search_planned("new token", 5, LinkGraphSearchOptions::default())
        .1;

    let actual = json!({
        "noop_mode": refresh_mode_label(noop),
        "full_mode": refresh_mode_label(full),
        "delta_mode": refresh_mode_label(delta),
        "full_refresh": {
            "hits": refresh_hits_snapshot(full_hits.as_slice()),
            "stats": stats_snapshot(full_index.stats()),
        },
        "delta_refresh": {
            "hits": refresh_hits_snapshot(delta_hits.as_slice()),
            "stats": stats_snapshot(delta_index.stats()),
        },
    });
    assert_refresh_fixture("threshold_modes", "result.json", &actual);
    Ok(())
}
