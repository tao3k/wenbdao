use crate::link_graph::runtime_config::constants::{
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_AGENT_ID,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_EVIDENCE_PREFIX,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_IDEMPOTENCY_SCAN_LIMIT,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_PERSIST_RETRY_ATTEMPTS,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_RELATION,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_WORKER_TIME_BUDGET_MS,
};
use crate::link_graph::runtime_config::models::LinkGraphAgenticRuntimeConfig;

pub(super) fn finalize_execution_defaults(resolved: &mut LinkGraphAgenticRuntimeConfig) {
    if resolved.execution_relation.trim().is_empty() {
        resolved.execution_relation = DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_RELATION.to_string();
    }
    if resolved.execution_agent_id.trim().is_empty() {
        resolved.execution_agent_id = DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_AGENT_ID.to_string();
    }
    if resolved.execution_evidence_prefix.trim().is_empty() {
        resolved.execution_evidence_prefix =
            DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_EVIDENCE_PREFIX.to_string();
    }
    if resolved.execution_persist_retry_attempts == 0 {
        resolved.execution_persist_retry_attempts =
            DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_PERSIST_RETRY_ATTEMPTS;
    }
    if resolved.execution_idempotency_scan_limit == 0 {
        resolved.execution_idempotency_scan_limit =
            DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_IDEMPOTENCY_SCAN_LIMIT;
    }
    if !(resolved.execution_worker_time_budget_ms.is_finite()
        && resolved.execution_worker_time_budget_ms > 0.0)
    {
        resolved.execution_worker_time_budget_ms =
            DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_WORKER_TIME_BUDGET_MS;
    }
}
