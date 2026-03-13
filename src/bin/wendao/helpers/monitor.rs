//! Monitor-related CLI helpers.

use serde_json::json;
use xiuxian_wendao::LinkGraphRelatedPprDiagnostics;

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
