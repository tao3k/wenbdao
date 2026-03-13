use crate::link_graph::runtime_config::constants::{
    LINK_GRAPH_COACTIVATION_ALPHA_SCALE_ENV, LINK_GRAPH_COACTIVATION_ENABLED_ENV,
    LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION_ENV,
    LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH_ENV,
};
use crate::link_graph::runtime_config::models::LinkGraphCoactivationRuntimeConfig;
use crate::link_graph::runtime_config::settings::{
    first_non_empty, get_setting_bool, get_setting_string, merged_wendao_settings, parse_bool,
    parse_positive_f64, parse_positive_usize,
};

#[allow(dead_code)]
pub fn resolve_link_graph_coactivation_runtime() -> LinkGraphCoactivationRuntimeConfig {
    let settings = merged_wendao_settings();
    let mut resolved = LinkGraphCoactivationRuntimeConfig::default();

    let enabled_from_env = std::env::var(LINK_GRAPH_COACTIVATION_ENABLED_ENV)
        .ok()
        .as_deref()
        .and_then(parse_bool);
    if let Some(value) =
        get_setting_bool(&settings, "link_graph.saliency.coactivation.enabled").or(enabled_from_env)
    {
        resolved.enabled = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.saliency.coactivation.alpha_scale"),
        std::env::var(LINK_GRAPH_COACTIVATION_ALPHA_SCALE_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_f64)
    {
        resolved.alpha_scale = value.clamp(0.0, 1.0);
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(
            &settings,
            "link_graph.saliency.coactivation.max_neighbors_per_direction",
        ),
        std::env::var(LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.max_neighbors_per_direction = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(
            &settings,
            "link_graph.saliency.coactivation.touch_queue_depth",
        ),
        std::env::var(LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.touch_queue_depth = value;
    }

    resolved
}

use crate::link_graph::models::LinkGraphRetrievalMode;
use crate::link_graph::runtime_config::models::LinkGraphRetrievalPolicyRuntimeConfig;

/// Resolve retrieval policy runtime configuration from settings.
pub(crate) fn resolve_link_graph_retrieval_policy_runtime() -> LinkGraphRetrievalPolicyRuntimeConfig
{
    let settings = merged_wendao_settings();
    let mut resolved = LinkGraphRetrievalPolicyRuntimeConfig::default();

    if let Some(value) = get_setting_string(&settings, "link_graph.retrieval.mode")
        .as_deref()
        .and_then(LinkGraphRetrievalMode::from_alias)
    {
        resolved.mode = value;
    }

    if let Some(value) = get_setting_string(&settings, "link_graph.retrieval.candidate_multiplier")
        .as_deref()
        .and_then(parse_positive_usize)
    {
        resolved.candidate_multiplier = value;
    }

    if let Some(value) = get_setting_string(&settings, "link_graph.retrieval.max_sources")
        .as_deref()
        .and_then(parse_positive_usize)
    {
        resolved.max_sources = value;
    }

    if let Some(value) = get_setting_string(&settings, "link_graph.retrieval.hybrid_min_hits")
        .as_deref()
        .and_then(parse_positive_usize)
    {
        resolved.hybrid_min_hits = value;
    }

    if let Some(value) = get_setting_string(&settings, "link_graph.retrieval.hybrid_min_top_score")
        .as_deref()
        .and_then(parse_positive_f64)
    {
        resolved.hybrid_min_top_score = value;
    }

    if let Some(value) = get_setting_string(&settings, "link_graph.retrieval.graph_rows_per_source")
        .as_deref()
        .and_then(parse_positive_usize)
    {
        resolved.graph_rows_per_source = value;
    }

    resolved
}
