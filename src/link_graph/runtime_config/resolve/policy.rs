use crate::link_graph::runtime_config::constants::{
    LINK_GRAPH_COACTIVATION_ALPHA_SCALE_ENV, LINK_GRAPH_COACTIVATION_ENABLED_ENV,
    LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE_ENV, LINK_GRAPH_COACTIVATION_MAX_HOPS_ENV,
    LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION_ENV,
    LINK_GRAPH_COACTIVATION_MAX_TOTAL_PROPAGATIONS_ENV,
    LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH_ENV, LINK_GRAPH_SEMANTIC_IGNITION_BACKEND_ENV,
    LINK_GRAPH_SEMANTIC_IGNITION_EMBEDDING_BASE_URL_ENV,
    LINK_GRAPH_SEMANTIC_IGNITION_EMBEDDING_MODEL_ENV, LINK_GRAPH_SEMANTIC_IGNITION_TABLE_NAME_ENV,
    LINK_GRAPH_SEMANTIC_IGNITION_VECTOR_STORE_PATH_ENV,
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
    let mut max_total_propagations_override: Option<usize> = None;

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
        get_setting_string(&settings, "link_graph.saliency.coactivation.max_hops"),
        std::env::var(LINK_GRAPH_COACTIVATION_MAX_HOPS_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.max_hops = value.clamp(1, 2);
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(
            &settings,
            "link_graph.saliency.coactivation.max_total_propagations",
        ),
        std::env::var(LINK_GRAPH_COACTIVATION_MAX_TOTAL_PROPAGATIONS_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        max_total_propagations_override = Some(value);
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(
            &settings,
            "link_graph.saliency.coactivation.hop_decay_scale",
        ),
        std::env::var(LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_f64)
    {
        resolved.hop_decay_scale = value.clamp(0.0, 1.0);
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

    resolved.max_total_propagations = max_total_propagations_override.unwrap_or_else(|| {
        resolved
            .max_neighbors_per_direction
            .saturating_mul(2)
            .saturating_mul(resolved.max_hops)
    });

    resolved
}

use crate::link_graph::models::LinkGraphRetrievalMode;
use crate::link_graph::runtime_config::models::{
    LinkGraphRetrievalPolicyRuntimeConfig, LinkGraphSemanticIgnitionBackend,
};

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

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.retrieval.semantic_ignition.backend"),
        std::env::var(LINK_GRAPH_SEMANTIC_IGNITION_BACKEND_ENV).ok(),
    ])
    .as_deref()
    .and_then(LinkGraphSemanticIgnitionBackend::from_alias)
    {
        resolved.semantic_ignition.backend = value;
    }

    resolved.semantic_ignition.vector_store_path =
        normalize_optional_runtime_string(first_non_empty(&[
            get_setting_string(
                &settings,
                "link_graph.retrieval.semantic_ignition.vector_store_path",
            ),
            std::env::var(LINK_GRAPH_SEMANTIC_IGNITION_VECTOR_STORE_PATH_ENV).ok(),
        ]));
    resolved.semantic_ignition.table_name = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(
            &settings,
            "link_graph.retrieval.semantic_ignition.table_name",
        ),
        std::env::var(LINK_GRAPH_SEMANTIC_IGNITION_TABLE_NAME_ENV).ok(),
    ]));
    resolved.semantic_ignition.embedding_base_url =
        normalize_optional_runtime_string(first_non_empty(&[
            get_setting_string(
                &settings,
                "link_graph.retrieval.semantic_ignition.embedding_base_url",
            ),
            std::env::var(LINK_GRAPH_SEMANTIC_IGNITION_EMBEDDING_BASE_URL_ENV).ok(),
        ]));
    resolved.semantic_ignition.embedding_model =
        normalize_optional_runtime_string(first_non_empty(&[
            get_setting_string(
                &settings,
                "link_graph.retrieval.semantic_ignition.embedding_model",
            ),
            std::env::var(LINK_GRAPH_SEMANTIC_IGNITION_EMBEDDING_MODEL_ENV).ok(),
        ]));

    resolved
}

fn normalize_optional_runtime_string(value: Option<String>) -> Option<String> {
    value
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
}
