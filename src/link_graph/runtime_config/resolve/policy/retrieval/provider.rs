use serde_yaml::Value;

use crate::link_graph::runtime_config::constants::{
    LINK_GRAPH_JULIA_RERANK_ANALYZER_CONFIG_PATH_ENV,
    LINK_GRAPH_JULIA_RERANK_ANALYZER_STRATEGY_ENV, LINK_GRAPH_JULIA_RERANK_BASE_URL_ENV,
    LINK_GRAPH_JULIA_RERANK_HEALTH_ROUTE_ENV, LINK_GRAPH_JULIA_RERANK_ROUTE_ENV,
    LINK_GRAPH_JULIA_RERANK_SCHEMA_VERSION_ENV, LINK_GRAPH_JULIA_RERANK_SERVICE_MODE_ENV,
    LINK_GRAPH_JULIA_RERANK_SIMILARITY_WEIGHT_ENV, LINK_GRAPH_JULIA_RERANK_TIMEOUT_SECS_ENV,
    LINK_GRAPH_JULIA_RERANK_VECTOR_WEIGHT_ENV,
};
use crate::link_graph::runtime_config::models::retrieval::LinkGraphJuliaRerankRuntimeConfig;
use crate::link_graph::runtime_config::settings::{
    first_non_empty, get_setting_string, parse_positive_f64, parse_positive_usize,
};

pub(super) fn apply_plugin_rerank_runtime_config_to_julia_runtime(
    settings: &Value,
    resolved: &mut LinkGraphJuliaRerankRuntimeConfig,
) {
    resolved.base_url = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.julia_rerank.base_url"),
        std::env::var(LINK_GRAPH_JULIA_RERANK_BASE_URL_ENV).ok(),
    ]));
    resolved.route = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.julia_rerank.route"),
        std::env::var(LINK_GRAPH_JULIA_RERANK_ROUTE_ENV).ok(),
    ]));
    resolved.health_route = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.julia_rerank.health_route"),
        std::env::var(LINK_GRAPH_JULIA_RERANK_HEALTH_ROUTE_ENV).ok(),
    ]));
    resolved.schema_version = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.julia_rerank.schema_version"),
        std::env::var(LINK_GRAPH_JULIA_RERANK_SCHEMA_VERSION_ENV).ok(),
    ]));
    resolved.timeout_secs = first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.julia_rerank.timeout_secs"),
        std::env::var(LINK_GRAPH_JULIA_RERANK_TIMEOUT_SECS_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    .map(|value| value as u64);
    resolved.service_mode = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.julia_rerank.service_mode"),
        std::env::var(LINK_GRAPH_JULIA_RERANK_SERVICE_MODE_ENV).ok(),
    ]));
    resolved.analyzer_config_path = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(
            settings,
            "link_graph.retrieval.julia_rerank.analyzer_config_path",
        ),
        std::env::var(LINK_GRAPH_JULIA_RERANK_ANALYZER_CONFIG_PATH_ENV).ok(),
    ]));
    resolved.analyzer_strategy = normalize_optional_runtime_string(first_non_empty(&[
        get_setting_string(
            settings,
            "link_graph.retrieval.julia_rerank.analyzer_strategy",
        ),
        std::env::var(LINK_GRAPH_JULIA_RERANK_ANALYZER_STRATEGY_ENV).ok(),
    ]));
    resolved.vector_weight = first_non_empty(&[
        get_setting_string(settings, "link_graph.retrieval.julia_rerank.vector_weight"),
        std::env::var(LINK_GRAPH_JULIA_RERANK_VECTOR_WEIGHT_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_f64);
    resolved.similarity_weight = first_non_empty(&[
        get_setting_string(
            settings,
            "link_graph.retrieval.julia_rerank.similarity_weight",
        ),
        std::env::var(LINK_GRAPH_JULIA_RERANK_SIMILARITY_WEIGHT_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_f64);
}

fn normalize_optional_runtime_string(value: Option<String>) -> Option<String> {
    value
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
}
