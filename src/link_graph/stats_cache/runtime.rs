use crate::link_graph::runtime_config::{
    DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX, resolve_link_graph_cache_runtime,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn now_unix_f64() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

pub(super) fn resolve_stats_cache_runtime() -> Result<(String, String), String> {
    let runtime = resolve_link_graph_cache_runtime()?;
    let key_prefix = if runtime.key_prefix.trim().is_empty() {
        DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX.to_string()
    } else {
        runtime.key_prefix
    };
    Ok((runtime.valkey_url, key_prefix))
}
