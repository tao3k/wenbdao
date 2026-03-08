use crate::link_graph::runtime_config::constants::{
    LINK_GRAPH_RELATED_MAX_CANDIDATES_ENV, LINK_GRAPH_RELATED_MAX_PARTITIONS_ENV,
    LINK_GRAPH_RELATED_TIME_BUDGET_MS_ENV,
};
use crate::link_graph::runtime_config::models::LinkGraphRelatedRuntimeConfig;
use crate::link_graph::runtime_config::settings::{
    first_non_empty, get_setting_string, merged_wendao_settings, parse_positive_f64,
    parse_positive_usize,
};

pub(crate) fn resolve_link_graph_related_runtime() -> LinkGraphRelatedRuntimeConfig {
    let settings = merged_wendao_settings();
    let mut resolved = LinkGraphRelatedRuntimeConfig::default();

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.related_ppr.max_candidates"),
        std::env::var(LINK_GRAPH_RELATED_MAX_CANDIDATES_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.max_candidates = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.related_ppr.max_partitions"),
        std::env::var(LINK_GRAPH_RELATED_MAX_PARTITIONS_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.max_partitions = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.related_ppr.time_budget_ms"),
        std::env::var(LINK_GRAPH_RELATED_TIME_BUDGET_MS_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_f64)
    {
        resolved.time_budget_ms = value;
    }

    resolved
}
