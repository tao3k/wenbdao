use crate::link_graph::runtime_config::constants::{
    LINK_GRAPH_COACTIVATION_ALPHA_SCALE_ENV, LINK_GRAPH_COACTIVATION_ENABLED_ENV,
    LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION_ENV,
    LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH_ENV,
};
use crate::link_graph::runtime_config::models::LinkGraphCoactivationRuntimeConfig;
use crate::link_graph::runtime_config::settings::{
    first_non_empty, get_setting_string, merged_wendao_settings, parse_bool, parse_positive_f64,
    parse_positive_usize,
};

/// Resolve coactivation runtime config with blueprint-aligned keys.
///
/// Config keys follow the living_brain_v2 blueprint: `link_graph.saliency.coactivation.*`
pub fn resolve_link_graph_coactivation_runtime() -> LinkGraphCoactivationRuntimeConfig {
    let settings = merged_wendao_settings();
    let mut resolved = LinkGraphCoactivationRuntimeConfig::default();

    // Blueprint-aligned keys: link_graph.saliency.coactivation.*
    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.saliency.coactivation.enabled"),
        std::env::var(LINK_GRAPH_COACTIVATION_ENABLED_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_bool)
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
        resolved.alpha_scale = value;
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
