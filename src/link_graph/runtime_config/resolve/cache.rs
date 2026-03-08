use crate::link_graph::runtime_config::constants::{
    DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX, LINK_GRAPH_CACHE_VALKEY_URL_ENV,
    LINK_GRAPH_VALKEY_KEY_PREFIX_ENV, LINK_GRAPH_VALKEY_TTL_SECONDS_ENV,
};
use crate::link_graph::runtime_config::models::LinkGraphCacheRuntimeConfig;
use crate::link_graph::runtime_config::settings::{
    first_non_empty, get_setting_string, merged_wendao_settings, parse_positive_u64,
};

pub(crate) fn resolve_link_graph_cache_runtime() -> Result<LinkGraphCacheRuntimeConfig, String> {
    let settings = merged_wendao_settings();

    let valkey_url = first_non_empty(&[
        get_setting_string(&settings, "link_graph.cache.valkey_url"),
        std::env::var(LINK_GRAPH_CACHE_VALKEY_URL_ENV).ok(),
    ])
    .ok_or_else(|| {
        "link_graph cache valkey url is required (set VALKEY_URL or link_graph.cache.valkey_url)"
            .to_string()
    })?;

    let key_prefix = first_non_empty(&[
        get_setting_string(&settings, "link_graph.cache.key_prefix"),
        std::env::var(LINK_GRAPH_VALKEY_KEY_PREFIX_ENV).ok(),
        Some(DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX.to_string()),
    ])
    .unwrap_or_else(|| DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX.to_string());

    let ttl_raw = first_non_empty(&[
        get_setting_string(&settings, "link_graph.cache.ttl_seconds"),
        std::env::var(LINK_GRAPH_VALKEY_TTL_SECONDS_ENV).ok(),
    ]);
    let ttl_seconds = ttl_raw.as_deref().and_then(parse_positive_u64);

    Ok(LinkGraphCacheRuntimeConfig::from_parts(
        &valkey_url,
        Some(&key_prefix),
        ttl_seconds,
    ))
}
