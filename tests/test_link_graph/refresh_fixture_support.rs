use serde_json::{Value, json};
use xiuxian_wendao::link_graph::{LinkGraphHit, LinkGraphRefreshMode, LinkGraphStats};

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::fixture_read::read_fixture;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

pub(super) struct RefreshFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl RefreshFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir =
            materialize_link_graph_fixture(&format!("link_graph/refresh/{scenario}/input"))?;
        let root = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_dir: temp_dir,
            root,
        })
    }

    pub(super) fn root(&self) -> &std::path::Path {
        self.root.as_path()
    }

    pub(super) fn path(&self, relative: &str) -> std::path::PathBuf {
        self.root.join(relative)
    }
}

pub(super) fn read_refresh_fixture(scenario: &str, relative: &str) -> String {
    read_fixture(&format!("link_graph/refresh/{scenario}"), relative)
}

pub(super) fn assert_refresh_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/refresh/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(super) fn refresh_sequence_snapshot(
    old_hits: &[LinkGraphHit],
    update_mode: LinkGraphRefreshMode,
    new_hits: &[LinkGraphHit],
    delete_mode: LinkGraphRefreshMode,
    final_stats: LinkGraphStats,
) -> Value {
    json!({
        "old_hits": refresh_hits_snapshot(old_hits),
        "update_mode": refresh_mode_label(update_mode),
        "new_hits": refresh_hits_snapshot(new_hits),
        "delete_mode": refresh_mode_label(delete_mode),
        "final_stats": stats_snapshot(final_stats),
    })
}

pub(super) fn refresh_hits_snapshot(hits: &[LinkGraphHit]) -> Value {
    let mut ordered = hits.iter().map(snapshot_hit).collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });
    json!({
        "hit_count": ordered.len(),
        "hits": ordered,
    })
}

pub(super) fn stats_snapshot(stats: LinkGraphStats) -> Value {
    json!({
        "total_notes": stats.total_notes,
        "orphans": stats.orphans,
        "links_in_graph": stats.links_in_graph,
        "nodes_in_graph": stats.nodes_in_graph,
    })
}

fn snapshot_hit(hit: &LinkGraphHit) -> Value {
    json!({
        "stem": hit.stem,
        "title": hit.title,
        "path": hit.path,
        "best_section": hit.best_section,
        "match_reason": hit.match_reason,
    })
}

pub(super) fn refresh_mode_label(mode: LinkGraphRefreshMode) -> &'static str {
    match mode {
        LinkGraphRefreshMode::Noop => "noop",
        LinkGraphRefreshMode::Delta => "delta",
        LinkGraphRefreshMode::Full => "full",
    }
}
