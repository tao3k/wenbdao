use serde_json::json;
use std::collections::BTreeMap;
use xiuxian_wendao::link_graph::LinkGraphPromotedOverlayTelemetry;
use xiuxian_wendao::{LinkGraphAgenticExecutionResult, LinkGraphRelatedPprDiagnostics};

pub(crate) fn build_related_monitor_phases(
    diagnostics: Option<LinkGraphRelatedPprDiagnostics>,
) -> Vec<serde_json::Value> {
    let Some(row) = diagnostics else {
        return Vec::new();
    };
    let mut phases: Vec<serde_json::Value> = Vec::new();
    phases.push(json!({
        "phase": "link_graph.related.ppr",
        "duration_ms": row.total_duration_ms,
        "extra": {
            "candidate_count": row.candidate_count,
            "candidate_cap": row.candidate_cap,
            "candidate_capped": row.candidate_capped,
            "graph_node_count": row.graph_node_count,
            "subgraph_count": row.subgraph_count,
            "subgraph_mode": row.subgraph_mode,
            "horizon_restricted": row.horizon_restricted,
            "time_budget_ms": row.time_budget_ms,
            "timed_out": row.timed_out,
            "iteration_count": row.iteration_count,
            "final_residual": row.final_residual,
        }
    }));
    phases.push(json!({
        "phase": "link_graph.related.subgraph.partition",
        "duration_ms": row.partition_duration_ms,
        "extra": {
            "subgraph_count": row.subgraph_count,
            "partition_max_node_count": row.partition_max_node_count,
            "partition_min_node_count": row.partition_min_node_count,
            "partition_avg_node_count": row.partition_avg_node_count,
            "timed_out": row.timed_out,
        }
    }));
    phases.push(json!({
        "phase": "link_graph.related.subgraph.fusion",
        "duration_ms": row.fusion_duration_ms,
        "extra": {
            "subgraph_count": row.subgraph_count,
            "timed_out": row.timed_out,
        }
    }));
    phases
}

pub(crate) fn build_promoted_overlay_monitor_phase(
    overlay: &LinkGraphPromotedOverlayTelemetry,
) -> serde_json::Value {
    json!({
        "phase": "link_graph.overlay.promoted",
        "duration_ms": 0.0,
        "extra": {
            "applied": overlay.applied,
            "source": overlay.source,
            "scanned_rows": overlay.scanned_rows,
            "promoted_rows": overlay.promoted_rows,
            "added_edges": overlay.added_edges,
        }
    })
}

#[derive(Default)]
struct AgenticPhaseAggregate {
    count: usize,
    total_duration_ms: f64,
    max_duration_ms: f64,
    item_count_sum: usize,
}

pub(crate) fn build_agentic_monitor_phases(
    result: &LinkGraphAgenticExecutionResult,
) -> Vec<serde_json::Value> {
    let mut phases: Vec<serde_json::Value> = Vec::new();
    phases.push(json!({
        "phase": "agentic.plan",
        "duration_ms": result.plan.elapsed_ms,
        "extra": {
            "workers": result.plan.workers.len(),
            "selected_pairs": result.plan.selected_pairs,
            "candidate_notes": result.plan.candidate_notes,
            "timed_out": result.plan.timed_out,
        }
    }));
    phases.push(json!({
        "phase": "agentic.execute.total",
        "duration_ms": result.elapsed_ms,
        "extra": {
            "worker_runs": result.worker_runs.len(),
            "prepared_proposals": result.prepared_proposals,
            "persisted_proposals": result.persisted_proposals,
            "skipped_duplicates": result.skipped_duplicates,
            "failed_proposals": result.failed_proposals,
            "persist_attempts": result.persist_attempts,
            "timed_out": result.timed_out,
        }
    }));

    let mut grouped: BTreeMap<String, AgenticPhaseAggregate> = BTreeMap::new();
    for worker in &result.worker_runs {
        for phase in &worker.phases {
            let key = format!("agentic.{}", phase.phase);
            let aggregate = grouped.entry(key).or_default();
            aggregate.count = aggregate.count.saturating_add(1);
            aggregate.total_duration_ms += phase.duration_ms;
            aggregate.max_duration_ms = aggregate.max_duration_ms.max(phase.duration_ms);
            aggregate.item_count_sum = aggregate.item_count_sum.saturating_add(phase.item_count);
        }
    }
    for (phase, aggregate) in grouped {
        phases.push(json!({
            "phase": phase,
            "count": aggregate.count,
            "duration_ms": aggregate.total_duration_ms,
            "max_duration_ms": aggregate.max_duration_ms,
            "extra": {
                "item_count_sum": aggregate.item_count_sum,
            }
        }));
    }
    phases
}

pub(crate) fn build_phase_monitor_summary(phases: &[serde_json::Value]) -> serde_json::Value {
    let mut slowest_phase: Option<(String, f64)> = None;
    let mut largest_item_phase: Option<(String, u64)> = None;

    for row in phases {
        let Some(phase) = row
            .get("phase")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
        else {
            continue;
        };
        if let Some(duration_ms) = row.get("duration_ms").and_then(serde_json::Value::as_f64) {
            match &slowest_phase {
                Some((_, current)) if duration_ms <= *current => {}
                _ => slowest_phase = Some((phase.clone(), duration_ms)),
            }
        }
        let item_count = row
            .get("extra")
            .and_then(|value| value.get("item_count_sum"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        match &largest_item_phase {
            Some((_, current)) if item_count <= *current => {}
            _ => largest_item_phase = Some((phase, item_count)),
        }
    }

    json!({
        "slowest_phase": slowest_phase.map(|(phase, duration_ms)| json!({
            "phase": phase,
            "duration_ms": duration_ms,
        })),
        "largest_item_phase": largest_item_phase.map(|(phase, item_count)| json!({
            "phase": phase,
            "item_count": item_count,
        })),
    })
}

pub(crate) fn build_agentic_monitor_summary(phases: &[serde_json::Value]) -> serde_json::Value {
    build_phase_monitor_summary(phases)
}
