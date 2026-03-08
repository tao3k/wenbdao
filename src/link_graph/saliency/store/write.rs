use super::common::{normalize_policy, now_unix_i64, redis_client, resolve_runtime};
use super::read::load_current_state;
use crate::link_graph::runtime_config::DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX;
use crate::link_graph::saliency::{
    DEFAULT_DECAY_RATE, DEFAULT_SALIENCY_BASE, LINK_GRAPH_SALIENCY_SCHEMA_VERSION,
    LinkGraphSaliencyState, LinkGraphSaliencyTouchRequest, calc::compute_link_graph_saliency,
    edge_in_key, edge_out_key, saliency_key,
};
use std::time::Duration;

fn update_inbound_edge_scores(
    conn: &mut redis::Connection,
    node_id: &str,
    key_prefix: &str,
    saliency_score: f64,
) {
    let inbound_key = edge_in_key(node_id, key_prefix);
    let inbound_sources = redis::cmd("SMEMBERS")
        .arg(&inbound_key)
        .query::<Vec<String>>(conn)
        .unwrap_or_default();
    for source in inbound_sources {
        let out_key = edge_out_key(source.trim(), key_prefix);
        let _ = redis::cmd("ZADD")
            .arg(&out_key)
            .arg(saliency_score)
            .arg(node_id)
            .query::<i64>(conn);
    }
}

/// Delete one saliency state entry by node id.
///
/// # Errors
///
/// Returns an error when runtime configuration is invalid or Valkey operations fail.
pub fn valkey_saliency_del(node_id: &str) -> Result<(), String> {
    let (valkey_url, key_prefix) = resolve_runtime()?;
    let trimmed = node_id.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let cache_key = saliency_key(trimmed, &key_prefix);
    let client = redis_client(&valkey_url)?;
    let mut conn = client
        .get_connection()
        .map_err(|err| format!("failed to connect valkey for link_graph saliency store: {err}"))?;
    redis::cmd("DEL")
        .arg(&cache_key)
        .query::<i64>(&mut conn)
        .map_err(|err| format!("failed to DEL link_graph saliency entry: {err}"))?;
    Ok(())
}

/// Update saliency using runtime-configured Valkey settings.
///
/// # Errors
///
/// Returns an error when runtime configuration is invalid or Valkey operations fail.
pub fn valkey_saliency_touch(
    request: LinkGraphSaliencyTouchRequest,
) -> Result<LinkGraphSaliencyState, String> {
    let (valkey_url, key_prefix) = resolve_runtime()?;
    valkey_saliency_touch_with_valkey(request, &valkey_url, Some(&key_prefix))
}

/// Update saliency using an explicit Valkey endpoint.
///
/// This applies decay + activation settlement and persists the resulting state.
///
/// # Errors
///
/// Returns an error when inputs are invalid, serialization fails, or Valkey operations fail.
pub fn valkey_saliency_touch_with_valkey(
    request: LinkGraphSaliencyTouchRequest,
    valkey_url: &str,
    key_prefix: Option<&str>,
) -> Result<LinkGraphSaliencyState, String> {
    let LinkGraphSaliencyTouchRequest {
        node_id,
        activation_delta,
        saliency_base,
        decay_rate: decay_rate_override,
        alpha,
        minimum_saliency,
        maximum_saliency,
        now_unix,
    } = request;

    let node_id = node_id.trim();
    if node_id.is_empty() {
        return Err("node_id must be non-empty".to_string());
    }
    if valkey_url.trim().is_empty() {
        return Err("link_graph saliency valkey_url must be non-empty".to_string());
    }
    let prefix = key_prefix
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX);
    let cache_key = saliency_key(node_id, prefix);
    let now_unix = now_unix.unwrap_or_else(now_unix_i64);
    let delta_activation = activation_delta.max(1);

    let policy = normalize_policy(alpha, minimum_saliency, maximum_saliency);
    let client = redis_client(valkey_url)?;
    let mut conn = client
        .get_connection()
        .map_err(|err| format!("failed to connect valkey for link_graph saliency store: {err}"))?;

    let existing = load_current_state(&mut conn, &cache_key, node_id, policy);
    // Settlement strategy:
    // - Use current score as the next baseline.
    // - Apply decay across elapsed time and only this touch delta for activation boost.
    // - Persist settled score as both `current_saliency` and next `saliency_base`.
    let (baseline, decay_rate, total_activation, delta_days) = if let Some(state) = existing {
        let elapsed_seconds = unix_seconds_to_f64((now_unix - state.last_accessed_unix).max(0));
        (
            saliency_base.unwrap_or(state.current_saliency),
            decay_rate_override.unwrap_or(state.decay_rate),
            state.activation_count.saturating_add(delta_activation),
            elapsed_seconds / 86_400.0,
        )
    } else {
        (
            saliency_base.unwrap_or(DEFAULT_SALIENCY_BASE),
            decay_rate_override.unwrap_or(DEFAULT_DECAY_RATE),
            delta_activation,
            0.0,
        )
    };

    let settled_score =
        compute_link_graph_saliency(baseline, decay_rate, delta_activation, delta_days, policy);
    let state = LinkGraphSaliencyState {
        schema: LINK_GRAPH_SALIENCY_SCHEMA_VERSION.to_string(),
        node_id: node_id.to_string(),
        saliency_base: settled_score,
        decay_rate,
        activation_count: total_activation,
        last_accessed_unix: now_unix,
        current_saliency: settled_score,
        updated_at_unix: unix_seconds_to_f64(now_unix),
    };
    let encoded = serde_json::to_string(&state)
        .map_err(|err| format!("failed to serialize link_graph saliency state: {err}"))?;

    redis::cmd("SET")
        .arg(&cache_key)
        .arg(encoded)
        .query::<()>(&mut conn)
        .map_err(|err| format!("failed to SET link_graph saliency entry: {err}"))?;

    update_inbound_edge_scores(&mut conn, node_id, prefix, settled_score);
    Ok(state)
}

fn unix_seconds_to_f64(seconds: i64) -> f64 {
    u64::try_from(seconds).map_or(0.0, |value| Duration::from_secs(value).as_secs_f64())
}
