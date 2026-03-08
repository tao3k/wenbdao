use serde_json::{Value, json};
use xiuxian_wendao::ParsedLinkGraphQuery;
use xiuxian_wendao::link_graph::{LinkGraphHit, LinkGraphMatchStrategy};

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

pub(super) struct SearchMatchFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl SearchMatchFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = materialize_link_graph_fixture(&format!(
            "link_graph/search_match_strategies/{scenario}/input"
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

pub(super) fn assert_search_match_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/search_match_strategies/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(super) fn parsed_query_snapshot(parsed: &ParsedLinkGraphQuery) -> Value {
    json!({
        "query": parsed.query,
        "match_strategy": match_strategy_label(parsed.options.match_strategy),
    })
}

pub(super) fn hits_outline_snapshot(hits: &[LinkGraphHit]) -> Value {
    json!({
        "hit_count": hits.len(),
        "hits": hits.iter().map(snapshot_hit_outline).collect::<Vec<_>>(),
    })
}

fn snapshot_hit_outline(hit: &LinkGraphHit) -> Value {
    json!({
        "stem": hit.stem,
        "path": hit.path,
        "best_section": hit.best_section,
        "match_reason": hit.match_reason,
    })
}

fn match_strategy_label(strategy: LinkGraphMatchStrategy) -> &'static str {
    match strategy {
        LinkGraphMatchStrategy::Fts => "fts",
        LinkGraphMatchStrategy::PathFuzzy => "path_fuzzy",
        LinkGraphMatchStrategy::Exact => "exact",
        LinkGraphMatchStrategy::Re => "regex",
    }
}
