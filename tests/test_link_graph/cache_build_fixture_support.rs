use serde_json::{Value, json};
use xiuxian_wendao::LinkGraphSaliencyState;
use xiuxian_wendao::link_graph::LinkGraphHit;

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::fixture_read::read_fixture;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

const FLOAT_PRECISION: f64 = 1_000_000_000_000.0;

pub(super) struct CacheBuildFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl CacheBuildFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir =
            materialize_link_graph_fixture(&format!("link_graph/cache_build/{scenario}/input"))?;
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

pub(super) fn read_cache_build_fixture(scenario: &str, relative: &str) -> String {
    read_fixture(&format!("link_graph/cache_build/{scenario}"), relative)
}

pub(super) fn assert_cache_build_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/cache_build/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(super) fn cache_stats_snapshot(index: &xiuxian_wendao::LinkGraphIndex) -> Value {
    let stats = index.stats();
    json!({
        "total_notes": stats.total_notes,
        "orphans": stats.orphans,
        "links_in_graph": stats.links_in_graph,
        "nodes_in_graph": stats.nodes_in_graph,
    })
}

pub(super) fn cache_hits_snapshot(hits: &[LinkGraphHit]) -> Value {
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

pub(super) fn saliency_state_snapshot(
    state: &LinkGraphSaliencyState,
    expected_current_saliency: f64,
) -> Value {
    json!({
        "node_id": state.node_id,
        "saliency_base": round_float(state.saliency_base),
        "decay_rate": round_float(state.decay_rate),
        "activation_count": state.activation_count,
        "current_saliency": round_float(state.current_saliency),
        "expected_current_saliency": round_float(expected_current_saliency),
        "current_matches_expected": (state.current_saliency - expected_current_saliency).abs() < 1e-9,
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

fn round_float(value: f64) -> f64 {
    (value * FLOAT_PRECISION).round() / FLOAT_PRECISION
}
