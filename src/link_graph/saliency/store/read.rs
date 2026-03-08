use super::common::{parse_saliency_payload, redis_client, resolve_runtime};
use crate::link_graph::runtime_config::DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX;
use crate::link_graph::saliency::{LinkGraphSaliencyPolicy, LinkGraphSaliencyState, saliency_key};

pub(in crate::link_graph::saliency::store) fn load_current_state(
    conn: &mut redis::Connection,
    cache_key: &str,
    node_id: &str,
    policy: LinkGraphSaliencyPolicy,
) -> Option<LinkGraphSaliencyState> {
    let raw = redis::cmd("GET")
        .arg(cache_key)
        .query::<Option<String>>(conn)
        .ok()?;
    let payload = raw?;
    let parsed = parse_saliency_payload(&payload, node_id, policy);
    if parsed.is_none() {
        let _ = redis::cmd("DEL").arg(cache_key).query::<i64>(conn);
    }
    parsed
}

/// Read one saliency state using runtime-configured Valkey settings.
///
/// # Errors
///
/// Returns an error when runtime configuration is invalid or Valkey operations fail.
pub fn valkey_saliency_get(node_id: &str) -> Result<Option<LinkGraphSaliencyState>, String> {
    let (valkey_url, key_prefix) = resolve_runtime()?;
    valkey_saliency_get_with_valkey(node_id, &valkey_url, Some(&key_prefix))
}

/// Read one saliency state from a specific Valkey endpoint.
///
/// # Errors
///
/// Returns an error when inputs are invalid or Valkey operations fail.
pub fn valkey_saliency_get_with_valkey(
    node_id: &str,
    valkey_url: &str,
    key_prefix: Option<&str>,
) -> Result<Option<LinkGraphSaliencyState>, String> {
    let trimmed = node_id.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if valkey_url.trim().is_empty() {
        return Err("link_graph saliency valkey_url must be non-empty".to_string());
    }
    let prefix = key_prefix
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX);
    let cache_key = saliency_key(trimmed, prefix);

    let policy = LinkGraphSaliencyPolicy::default();
    let client = redis_client(valkey_url)?;
    let mut conn = client
        .get_connection()
        .map_err(|err| format!("failed to connect valkey for link_graph saliency store: {err}"))?;

    let raw = redis::cmd("GET")
        .arg(&cache_key)
        .query::<Option<String>>(&mut conn)
        .map_err(|err| format!("failed to GET link_graph saliency entry: {err}"))?;
    let Some(payload_raw) = raw else {
        return Ok(None);
    };

    if let Some(state) = parse_saliency_payload(&payload_raw, trimmed, policy) {
        return Ok(Some(state));
    }

    let _ = redis::cmd("DEL").arg(&cache_key).query::<i64>(&mut conn);
    Ok(None)
}
