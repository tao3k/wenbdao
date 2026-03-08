use serde_json::{Value, json};
use xiuxian_wendao::link_graph::LinkGraphHit;

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

const SCORE_PRECISION: f64 = 1_000_000_000_000.0;

pub(super) struct SeedAndPriorsFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl SeedAndPriorsFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = materialize_link_graph_fixture(&format!(
            "link_graph/seed_and_priors/{scenario}/input"
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

pub(super) fn assert_seed_and_priors_fixture(scenario: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/seed_and_priors/{scenario}/expected"),
        "result.json",
        actual,
    );
}

pub(super) fn hits_snapshot(hits: &[LinkGraphHit]) -> Value {
    json!({
        "hit_count": hits.len(),
        "hits": hits.iter().map(snapshot_hit).collect::<Vec<_>>(),
        "stems": hits.iter().map(|hit| hit.stem.clone()).collect::<Vec<_>>(),
    })
}

pub(super) fn structural_prior_snapshot(
    boosted_hits: &[LinkGraphHit],
    baseline_hits: &[LinkGraphHit],
) -> Result<Value, Box<dyn std::error::Error>> {
    let hub_rank = boosted_hits
        .iter()
        .position(|row| row.stem == "hub")
        .ok_or("missing hub hit with structural priors enabled")?;
    let hub_score_with_priors = boosted_hits
        .iter()
        .find(|row| row.stem == "hub")
        .map(|row| row.score)
        .ok_or("missing hub score with structural priors")?;
    let hub_score_without_semantic_boost = baseline_hits
        .iter()
        .find(|row| row.stem == "hub")
        .map(|row| row.score)
        .ok_or("missing hub score in structural-only baseline")?;
    let hub_match_reason_includes_graph_rank = boosted_hits.iter().any(|row| {
        row.stem == "hub"
            && row
                .match_reason
                .as_deref()
                .unwrap_or_default()
                .contains("graph_rank")
    });

    Ok(json!({
        "boosted": hits_snapshot(boosted_hits),
        "baseline": hits_snapshot(baseline_hits),
        "hub_rank": hub_rank,
        "hub_in_top3": hub_rank < 3,
        "hub_score_with_priors": round_score(hub_score_with_priors),
        "hub_score_without_semantic_boost": round_score(hub_score_without_semantic_boost),
        "hub_score_improved": hub_score_with_priors > hub_score_without_semantic_boost,
        "hub_match_reason_includes_graph_rank": hub_match_reason_includes_graph_rank,
    }))
}

fn snapshot_hit(hit: &LinkGraphHit) -> Value {
    let mut tags = hit.tags.clone();
    tags.sort();

    json!({
        "stem": hit.stem,
        "title": hit.title,
        "path": hit.path,
        "doc_type": hit.doc_type,
        "tags": tags,
        "score": round_score(hit.score),
        "best_section": hit.best_section,
        "match_reason": hit.match_reason,
    })
}

fn round_score(value: f64) -> f64 {
    (value * SCORE_PRECISION).round() / SCORE_PRECISION
}
