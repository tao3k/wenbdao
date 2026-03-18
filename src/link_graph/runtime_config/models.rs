use super::constants::{
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_AGENT_ID,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_EVIDENCE_PREFIX,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_IDEMPOTENCY_SCAN_LIMIT,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_PERSIST_RETRY_ATTEMPTS,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_PERSIST_SUGGESTIONS_DEFAULT,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_RELATION,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_WORKER_TIME_BUDGET_MS,
    DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_CANDIDATES,
    DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_PAIRS_PER_WORKER,
    DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_WORKERS,
    DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_TIME_BUDGET_MS,
    DEFAULT_LINK_GRAPH_AGENTIC_SEARCH_PROVISIONAL_LIMIT,
    DEFAULT_LINK_GRAPH_AGENTIC_SUGGESTED_LINK_MAX_ENTRIES, DEFAULT_LINK_GRAPH_CANDIDATE_MULTIPLIER,
    DEFAULT_LINK_GRAPH_COACTIVATION_ALPHA_SCALE, DEFAULT_LINK_GRAPH_COACTIVATION_ENABLED,
    DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE, DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS,
    DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION,
    DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH, DEFAULT_LINK_GRAPH_HYBRID_MIN_HITS,
    DEFAULT_LINK_GRAPH_HYBRID_MIN_TOP_SCORE, DEFAULT_LINK_GRAPH_MAX_SOURCES,
    DEFAULT_LINK_GRAPH_RELATED_MAX_CANDIDATES, DEFAULT_LINK_GRAPH_RELATED_MAX_PARTITIONS,
    DEFAULT_LINK_GRAPH_RELATED_TIME_BUDGET_MS, DEFAULT_LINK_GRAPH_RETRIEVAL_MODE,
    DEFAULT_LINK_GRAPH_ROWS_PER_SOURCE, DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX,
};
use crate::link_graph::models::LinkGraphRetrievalMode;

#[derive(Debug, Clone)]
pub(crate) struct LinkGraphCacheRuntimeConfig {
    pub valkey_url: String,
    pub key_prefix: String,
    pub ttl_seconds: Option<u64>,
}

impl LinkGraphCacheRuntimeConfig {
    pub(crate) fn from_parts(
        valkey_url: &str,
        key_prefix: Option<&str>,
        ttl_seconds: Option<u64>,
    ) -> Self {
        let resolved_url = valkey_url.trim().to_string();
        let resolved_prefix = key_prefix
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX)
            .to_string();
        Self {
            valkey_url: resolved_url,
            key_prefix: resolved_prefix,
            ttl_seconds: ttl_seconds.filter(|value| *value > 0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LinkGraphRelatedRuntimeConfig {
    pub max_candidates: usize,
    pub max_partitions: usize,
    pub time_budget_ms: f64,
}

impl Default for LinkGraphRelatedRuntimeConfig {
    fn default() -> Self {
        Self {
            max_candidates: DEFAULT_LINK_GRAPH_RELATED_MAX_CANDIDATES,
            max_partitions: DEFAULT_LINK_GRAPH_RELATED_MAX_PARTITIONS,
            time_budget_ms: DEFAULT_LINK_GRAPH_RELATED_TIME_BUDGET_MS,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LinkGraphCoactivationRuntimeConfig {
    pub enabled: bool,
    pub alpha_scale: f64,
    pub max_neighbors_per_direction: usize,
    pub max_hops: usize,
    pub max_total_propagations: usize,
    pub hop_decay_scale: f64,
    pub touch_queue_depth: usize,
}

impl Default for LinkGraphCoactivationRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_LINK_GRAPH_COACTIVATION_ENABLED,
            alpha_scale: DEFAULT_LINK_GRAPH_COACTIVATION_ALPHA_SCALE,
            max_neighbors_per_direction:
                DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION,
            max_hops: DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS,
            max_total_propagations: DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION
                .saturating_mul(2),
            hop_decay_scale: DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE,
            touch_queue_depth: DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH,
        }
    }
}

pub struct LinkGraphRetrievalPolicyRuntimeConfig {
    pub mode: LinkGraphRetrievalMode,
    pub candidate_multiplier: usize,
    pub max_sources: usize,
    pub hybrid_min_hits: usize,
    pub hybrid_min_top_score: f64,
    pub graph_rows_per_source: usize,
}

impl Default for LinkGraphRetrievalPolicyRuntimeConfig {
    fn default() -> Self {
        Self {
            mode: LinkGraphRetrievalMode::from_alias(DEFAULT_LINK_GRAPH_RETRIEVAL_MODE)
                .unwrap_or_default(),
            candidate_multiplier: DEFAULT_LINK_GRAPH_CANDIDATE_MULTIPLIER,
            max_sources: DEFAULT_LINK_GRAPH_MAX_SOURCES,
            hybrid_min_hits: DEFAULT_LINK_GRAPH_HYBRID_MIN_HITS,
            hybrid_min_top_score: DEFAULT_LINK_GRAPH_HYBRID_MIN_TOP_SCORE,
            graph_rows_per_source: DEFAULT_LINK_GRAPH_ROWS_PER_SOURCE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinkGraphAgenticRuntimeConfig {
    pub suggested_link_max_entries: usize,
    pub suggested_link_ttl_seconds: Option<u64>,
    pub search_include_provisional_default: bool,
    pub search_provisional_limit: usize,
    pub expansion_max_workers: usize,
    pub expansion_max_candidates: usize,
    pub expansion_max_pairs_per_worker: usize,
    pub expansion_time_budget_ms: f64,
    pub execution_worker_time_budget_ms: f64,
    pub execution_persist_suggestions_default: bool,
    pub execution_persist_retry_attempts: usize,
    pub execution_idempotency_scan_limit: usize,
    pub execution_relation: String,
    pub execution_agent_id: String,
    pub execution_evidence_prefix: String,
}

impl Default for LinkGraphAgenticRuntimeConfig {
    fn default() -> Self {
        Self {
            suggested_link_max_entries: DEFAULT_LINK_GRAPH_AGENTIC_SUGGESTED_LINK_MAX_ENTRIES,
            suggested_link_ttl_seconds: None,
            search_include_provisional_default: false,
            search_provisional_limit: DEFAULT_LINK_GRAPH_AGENTIC_SEARCH_PROVISIONAL_LIMIT,
            expansion_max_workers: DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_WORKERS,
            expansion_max_candidates: DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_CANDIDATES,
            expansion_max_pairs_per_worker:
                DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_PAIRS_PER_WORKER,
            expansion_time_budget_ms: DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_TIME_BUDGET_MS,
            execution_worker_time_budget_ms:
                DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_WORKER_TIME_BUDGET_MS,
            execution_persist_suggestions_default:
                DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_PERSIST_SUGGESTIONS_DEFAULT,
            execution_persist_retry_attempts:
                DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_PERSIST_RETRY_ATTEMPTS,
            execution_idempotency_scan_limit:
                DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_IDEMPOTENCY_SCAN_LIMIT,
            execution_relation: DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_RELATION.to_string(),
            execution_agent_id: DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_AGENT_ID.to_string(),
            execution_evidence_prefix: DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_EVIDENCE_PREFIX
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Resolved `LinkGraph` index scope derived from runtime configuration.
pub struct LinkGraphIndexRuntimeConfig {
    /// Relative include directories used for index scope.
    pub include_dirs: Vec<String>,
    /// Relative directory names excluded from indexing.
    pub exclude_dirs: Vec<String>,
}
