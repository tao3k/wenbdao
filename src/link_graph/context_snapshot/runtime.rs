use crate::link_graph::runtime_config::{
    DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX, resolve_link_graph_cache_runtime,
};
use crate::valkey_common::{normalize_key_prefix, open_client};

pub(crate) fn resolve_snapshot_runtime() -> Result<(String, String), String> {
    let runtime = resolve_link_graph_cache_runtime()?;
    let key_prefix = normalize_snapshot_key_prefix(runtime.key_prefix.as_str());
    Ok((runtime.valkey_url, key_prefix))
}

pub(crate) fn normalize_snapshot_key_prefix(candidate: &str) -> String {
    normalize_key_prefix(candidate, DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX)
}

pub(crate) fn snapshot_cache_key(snapshot_id: &str, key_prefix: &str) -> String {
    format!("{key_prefix}:quantum_context:snapshot:{snapshot_id}")
}

pub(crate) fn snapshot_redis_client(valkey_url: &str) -> Result<redis::Client, String> {
    open_client(valkey_url)
        .map_err(|err| format!("invalid valkey url for link_graph snapshot store: {err}"))
}
