//! Fixture-backed projections for bounded agentic expansion tests.

use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::{Value, json};
use tempfile::TempDir;
use xiuxian_wendao::{
    LinkGraphAgenticCandidatePair, LinkGraphAgenticExecutionResult, LinkGraphAgenticExpansionPlan,
    LinkGraphAgenticWorkerExecution, LinkGraphAgenticWorkerPhase, LinkGraphAgenticWorkerPlan,
};

use super::fixture_json_assertions::assert_json_fixture_eq;
use super::link_graph_fixture_tree::materialize_link_graph_fixture;

const FLOAT_PRECISION: f64 = 1_000_000_000_000.0;

pub(crate) struct AgenticExpansionFixture {
    _temp_dir: TempDir,
    root: PathBuf,
}

impl AgenticExpansionFixture {
    pub(crate) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = materialize_link_graph_fixture(&format!(
            "link_graph/agentic_expansion/{scenario}/input"
        ))?;
        let root = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_dir: temp_dir,
            root,
        })
    }

    pub(crate) fn root(&self) -> &std::path::Path {
        self.root.as_path()
    }
}

pub(crate) fn assert_agentic_expansion_fixture(scenario: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/agentic_expansion/{scenario}/expected"),
        "result.json",
        actual,
    );
}

pub(crate) fn plan_snapshot(plan: &LinkGraphAgenticExpansionPlan) -> Value {
    let worker_pair_sum = plan
        .workers
        .iter()
        .map(|worker| worker.pair_count)
        .sum::<usize>();
    let unique_pair_count = unique_pair_count(&plan.workers);

    json!({
        "query": plan.query,
        "total_notes": plan.total_notes,
        "candidate_notes": plan.candidate_notes,
        "total_possible_pairs": plan.total_possible_pairs,
        "evaluated_pairs": plan.evaluated_pairs,
        "selected_pairs": plan.selected_pairs,
        "timed_out": plan.timed_out,
        "capped_by_pair_limit": plan.capped_by_pair_limit,
        "config": {
            "max_workers": plan.config.max_workers,
            "max_candidates": plan.config.max_candidates,
            "max_pairs_per_worker": plan.config.max_pairs_per_worker,
            "time_budget_ms": round_float(plan.config.time_budget_ms),
        },
        "elapsed_ms_non_negative": plan.elapsed_ms >= 0.0,
        "worker_count": plan.workers.len(),
        "worker_pair_sum": worker_pair_sum,
        "selected_pairs_matches_worker_sum": plan.selected_pairs == worker_pair_sum,
        "unique_pair_count": unique_pair_count,
        "workers": plan
            .workers
            .iter()
            .map(worker_plan_snapshot)
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn execution_snapshot(result: &LinkGraphAgenticExecutionResult) -> Value {
    json!({
        "query": result.query,
        "config": {
            "expansion": {
                "max_workers": result.config.expansion.max_workers,
                "max_candidates": result.config.expansion.max_candidates,
                "max_pairs_per_worker": result.config.expansion.max_pairs_per_worker,
                "time_budget_ms": round_float(result.config.expansion.time_budget_ms),
            },
            "worker_time_budget_ms": round_float(result.config.worker_time_budget_ms),
            "persist_suggestions": result.config.persist_suggestions,
            "persist_retry_attempts": result.config.persist_retry_attempts,
            "idempotency_scan_limit": result.config.idempotency_scan_limit,
            "relation": result.config.relation,
            "agent_id": result.config.agent_id,
            "evidence_prefix": result.config.evidence_prefix,
            "created_at_unix": result.config.created_at_unix.map(round_float),
        },
        "plan": plan_snapshot(&result.plan),
        "worker_runs": result
            .worker_runs
            .iter()
            .map(worker_execution_snapshot)
            .collect::<Vec<_>>(),
        "prepared_proposals": result.prepared_proposals,
        "persisted_proposals": result.persisted_proposals,
        "skipped_duplicates": result.skipped_duplicates,
        "failed_proposals": result.failed_proposals,
        "persist_attempts": result.persist_attempts,
        "timed_out": result.timed_out,
        "elapsed_ms_non_negative": result.elapsed_ms >= 0.0,
        "errors": result.errors,
    })
}

fn worker_plan_snapshot(worker: &LinkGraphAgenticWorkerPlan) -> Value {
    json!({
        "worker_id": worker.worker_id,
        "seed_ids": worker.seed_ids,
        "pair_count": worker.pair_count,
        "pair_count_matches_pairs_len": worker.pair_count == worker.pairs.len(),
        "pairs": worker.pairs.iter().map(pair_snapshot).collect::<Vec<_>>(),
    })
}

fn pair_snapshot(pair: &LinkGraphAgenticCandidatePair) -> Value {
    json!({
        "left_id": pair.left_id,
        "right_id": pair.right_id,
        "priority": round_float(pair.priority),
    })
}

fn worker_execution_snapshot(worker: &LinkGraphAgenticWorkerExecution) -> Value {
    json!({
        "worker_id": worker.worker_id,
        "pair_budget": worker.pair_budget,
        "processed_pairs": worker.processed_pairs,
        "prepared_proposals": worker.prepared_proposals,
        "persisted_proposals": worker.persisted_proposals,
        "skipped_duplicates": worker.skipped_duplicates,
        "failed_proposals": worker.failed_proposals,
        "persist_attempts": worker.persist_attempts,
        "timed_out": worker.timed_out,
        "elapsed_ms_non_negative": worker.elapsed_ms >= 0.0,
        "estimated_prompt_tokens_positive": worker.estimated_prompt_tokens > 0,
        "estimated_completion_tokens": worker.estimated_completion_tokens,
        "phases_len": worker.phases.len(),
        "phases": worker.phases.iter().map(worker_phase_snapshot).collect::<Vec<_>>(),
    })
}

fn worker_phase_snapshot(phase: &LinkGraphAgenticWorkerPhase) -> Value {
    json!({
        "phase": phase.phase,
        "item_count": phase.item_count,
        "duration_ms_non_negative": phase.duration_ms >= 0.0,
    })
}

fn unique_pair_count(workers: &[LinkGraphAgenticWorkerPlan]) -> usize {
    let mut unique_pairs = HashSet::<(String, String)>::new();
    for worker in workers {
        for pair in &worker.pairs {
            let key = if pair.left_id <= pair.right_id {
                (pair.left_id.clone(), pair.right_id.clone())
            } else {
                (pair.right_id.clone(), pair.left_id.clone())
            };
            unique_pairs.insert(key);
        }
    }
    unique_pairs.len()
}

fn round_float(value: f64) -> f64 {
    (value * FLOAT_PRECISION).round() / FLOAT_PRECISION
}
