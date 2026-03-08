use crate::link_graph::models::LinkGraphSemanticDocumentScope;
use crate::link_graph::runtime_config::constants::{
    LINK_GRAPH_CANDIDATE_MULTIPLIER_ENV, LINK_GRAPH_HYBRID_MIN_HITS_ENV,
    LINK_GRAPH_HYBRID_MIN_TOP_SCORE_ENV, LINK_GRAPH_MAX_SOURCES_ENV, LINK_GRAPH_RETRIEVAL_MODE_ENV,
    LINK_GRAPH_ROWS_PER_SOURCE_ENV, LINK_GRAPH_SEMANTIC_MIN_VECTOR_SCORE_ENV,
    LINK_GRAPH_SEMANTIC_SUMMARY_ONLY_ENV,
};
use crate::link_graph::runtime_config::models::LinkGraphRetrievalPolicyRuntimeConfig;
use crate::link_graph::runtime_config::settings::{
    first_non_empty, get_setting_bool, get_setting_string, merged_wendao_settings,
    parse_positive_usize,
};

fn parse_mode(raw: &str) -> Option<crate::link_graph::models::LinkGraphRetrievalMode> {
    crate::link_graph::models::LinkGraphRetrievalMode::from_alias(raw)
}

fn parse_score(raw: &str) -> Option<f64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
}

pub(crate) fn resolve_link_graph_retrieval_policy_runtime() -> LinkGraphRetrievalPolicyRuntimeConfig
{
    let settings = merged_wendao_settings();
    let mut resolved = LinkGraphRetrievalPolicyRuntimeConfig::default();

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.retrieval_mode"),
        std::env::var(LINK_GRAPH_RETRIEVAL_MODE_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_mode)
    {
        resolved.mode = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.candidate_multiplier"),
        std::env::var(LINK_GRAPH_CANDIDATE_MULTIPLIER_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.candidate_multiplier = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.max_sources"),
        std::env::var(LINK_GRAPH_MAX_SOURCES_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.max_sources = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.hybrid.min_hits"),
        std::env::var(LINK_GRAPH_HYBRID_MIN_HITS_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.hybrid_min_hits = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.hybrid.min_top_score"),
        std::env::var(LINK_GRAPH_HYBRID_MIN_TOP_SCORE_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_score)
    {
        resolved.hybrid_min_top_score = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.graph_rows_per_source"),
        std::env::var(LINK_GRAPH_ROWS_PER_SOURCE_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.graph_rows_per_source = value;
    }

    if get_setting_bool(&settings, "link_graph.semantic.summary_only")
        .or_else(|| {
            std::env::var(LINK_GRAPH_SEMANTIC_SUMMARY_ONLY_ENV)
                .ok()
                .and_then(|raw| crate::link_graph::runtime_config::settings::parse_bool(&raw))
        })
        .unwrap_or(false)
    {
        resolved.semantic_policy.document_scope = LinkGraphSemanticDocumentScope::SummaryOnly;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(&settings, "link_graph.semantic.min_vector_score"),
        std::env::var(LINK_GRAPH_SEMANTIC_MIN_VECTOR_SCORE_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_score)
    {
        resolved.semantic_policy.min_vector_score = Some(value);
    }

    resolved.semantic_policy = resolved.semantic_policy.normalized();
    resolved
}
