use serde_json::{Value, json};
use xiuxian_wendao::ParsedLinkGraphQuery;
use xiuxian_wendao::link_graph::{
    LinkGraphDisplayHit, LinkGraphHit, LinkGraphPlannedSearchPayload, LinkGraphRetrievalPlanRecord,
    LinkGraphStats,
};

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

const SCORE_PRECISION: f64 = 1_000_000_000_000.0;

pub(super) struct SearchCoreFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl SearchCoreFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir =
            materialize_link_graph_fixture(&format!("link_graph/search_core/{scenario}/input"))?;
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

pub(super) fn assert_search_core_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/search_core/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(super) fn stats_and_hits_snapshot(stats: LinkGraphStats, hits: &[LinkGraphHit]) -> Value {
    json!({
        "stats": stats_snapshot(stats),
        "hits": hits.iter().map(snapshot_hit).collect::<Vec<_>>(),
    })
}

pub(super) fn hits_snapshot(hits: &[LinkGraphHit]) -> Value {
    json!({
        "hits": hits.iter().map(snapshot_hit).collect::<Vec<_>>(),
    })
}

pub(super) fn direct_id_snapshot(parsed: &ParsedLinkGraphQuery, hits: &[LinkGraphHit]) -> Value {
    json!({
        "direct_id": parsed.direct_id,
        "query": parsed.query,
        "hits": hits.iter().map(snapshot_hit).collect::<Vec<_>>(),
    })
}

pub(super) fn planned_payload_snapshot(payload: &LinkGraphPlannedSearchPayload) -> Value {
    json!({
        "query": payload.query,
        "hit_count": payload.hit_count,
        "section_hit_count": payload.section_hit_count,
        "requested_mode": payload.requested_mode,
        "selected_mode": payload.selected_mode,
        "reason": payload.reason,
        "graph_hit_count": payload.graph_hit_count,
        "source_hint_count": payload.source_hint_count,
        "graph_confidence_score": round_score(payload.graph_confidence_score),
        "graph_confidence_level": payload.graph_confidence_level,
        "hits": payload.hits.iter().map(snapshot_display_hit).collect::<Vec<_>>(),
        "results": payload.results.iter().map(snapshot_hit).collect::<Vec<_>>(),
        "retrieval_plan": payload.retrieval_plan.as_ref().map(snapshot_retrieval_plan),
    })
}

fn stats_snapshot(stats: LinkGraphStats) -> Value {
    json!({
        "total_notes": stats.total_notes,
        "orphans": stats.orphans,
        "links_in_graph": stats.links_in_graph,
        "nodes_in_graph": stats.nodes_in_graph,
    })
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

fn snapshot_display_hit(hit: &LinkGraphDisplayHit) -> Value {
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

fn snapshot_retrieval_plan(plan: &LinkGraphRetrievalPlanRecord) -> Value {
    json!({
        "schema": plan.schema,
        "requested_mode": plan.requested_mode,
        "selected_mode": plan.selected_mode,
        "reason": plan.reason,
        "backend_name": plan.backend_name,
        "graph_hit_count": plan.graph_hit_count,
        "source_hint_count": plan.source_hint_count,
        "graph_confidence_score": round_score(plan.graph_confidence_score),
        "graph_confidence_level": plan.graph_confidence_level,
        "semantic_policy": {
            "document_scope": plan.semantic_policy.document_scope,
            "min_vector_score": plan.semantic_policy.min_vector_score.map(round_score),
        },
        "budget": {
            "candidate_limit": plan.budget.candidate_limit,
            "max_sources": plan.budget.max_sources,
            "rows_per_source": plan.budget.rows_per_source,
        },
    })
}

fn round_score(value: f64) -> f64 {
    (value * SCORE_PRECISION).round() / SCORE_PRECISION
}
