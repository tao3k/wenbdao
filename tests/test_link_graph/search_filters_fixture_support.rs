use serde_json::Value;
use xiuxian_wendao::link_graph::LinkGraphHit;

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

pub(super) struct SearchFilterFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl SearchFilterFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir =
            materialize_link_graph_fixture(&format!("link_graph/search_filters/{scenario}/input"))?;
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

pub(super) fn assert_search_filter_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/search_filters/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(super) fn ordered_hit_paths(hits: &[LinkGraphHit]) -> Vec<String> {
    let mut paths = hits.iter().map(|hit| hit.path.clone()).collect::<Vec<_>>();
    paths.sort();
    paths
}
