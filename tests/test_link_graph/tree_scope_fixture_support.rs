use serde_json::{Value, json};
use xiuxian_wendao::link_graph::LinkGraphHit;

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

pub(super) struct TreeScopeFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl TreeScopeFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = materialize_link_graph_fixture(&format!(
            "link_graph/tree_scope_filters/{scenario}/input"
        ))?;
        let root = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_dir: temp_dir,
            root,
        })
    }

    pub(super) fn build_index(
        &self,
    ) -> Result<xiuxian_wendao::LinkGraphIndex, Box<dyn std::error::Error>> {
        xiuxian_wendao::LinkGraphIndex::build(self.root.as_path())
            .map_err(|error| error.clone().into())
    }
}

pub(super) fn assert_tree_scope_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/tree_scope_filters/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(super) fn tree_hit_outline_snapshot(hits: &[LinkGraphHit]) -> Value {
    let mut ordered = hits.iter().map(snapshot_hit).collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        let left_key = (
            left["path"].as_str().unwrap_or_default(),
            left["best_section"].as_str().unwrap_or_default(),
        );
        let right_key = (
            right["path"].as_str().unwrap_or_default(),
            right["best_section"].as_str().unwrap_or_default(),
        );
        left_key.cmp(&right_key)
    });
    json!({
        "hit_count": ordered.len(),
        "hits": ordered,
    })
}

pub(super) fn per_path_counts_snapshot(hits: &[LinkGraphHit]) -> Value {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for hit in hits {
        *counts.entry(hit.path.clone()).or_default() += 1;
    }

    json!({
        "counts": counts
            .into_iter()
            .map(|(path, count)| json!({ "path": path, "count": count }))
            .collect::<Vec<_>>(),
    })
}

pub(super) fn ordered_section_labels(hits: &[LinkGraphHit]) -> Vec<String> {
    let mut sections = hits
        .iter()
        .filter_map(|hit| hit.best_section.clone())
        .collect::<Vec<_>>();
    sections.sort();
    sections
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
