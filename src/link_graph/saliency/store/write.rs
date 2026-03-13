use super::common::{normalize_policy, now_unix_i64, redis_client, resolve_runtime};
use super::read::load_current_state;
use crate::link_graph::runtime_config::{
    DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX, resolve_link_graph_coactivation_runtime,
};
use crate::link_graph::saliency::{
    DEFAULT_DECAY_RATE, DEFAULT_SALIENCY_BASE, LINK_GRAPH_SALIENCY_SCHEMA_VERSION,
    LinkGraphSaliencyPolicy, LinkGraphSaliencyState, LinkGraphSaliencyTouchRequest,
    calc::compute_link_graph_saliency, edge_in_key, edge_out_key, saliency_key,
};
use std::collections::HashSet;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
struct TouchUpdateSpec {
    activation_delta: u64,
    saliency_base: Option<f64>,
    decay_rate_override: Option<f64>,
    policy: LinkGraphSaliencyPolicy,
    now_unix: i64,
}

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

fn apply_touch_with_connection(
    conn: &mut redis::Connection,
    node_id: &str,
    key_prefix: &str,
    spec: TouchUpdateSpec,
) -> Result<LinkGraphSaliencyState, String> {
    let cache_key = saliency_key(node_id, key_prefix);
    let delta_activation = spec.activation_delta.max(1);
    let normalized_policy = spec.policy.normalized();
    let existing = load_current_state(conn, &cache_key, node_id, normalized_policy);

    // Settlement strategy:
    // - Use current score as the next baseline.
    // - Apply decay across elapsed time and only this touch delta for activation boost.
    // - Persist settled score as both `current_saliency` and next `saliency_base`.
    let (baseline, decay_rate, total_activation, delta_days) = if let Some(state) = existing {
        let elapsed_seconds =
            unix_seconds_to_f64((spec.now_unix - state.last_accessed_unix).max(0));
        (
            spec.saliency_base.unwrap_or(state.current_saliency),
            spec.decay_rate_override.unwrap_or(state.decay_rate),
            state.activation_count.saturating_add(delta_activation),
            elapsed_seconds / 86_400.0,
        )
    } else {
        (
            spec.saliency_base.unwrap_or(DEFAULT_SALIENCY_BASE),
            spec.decay_rate_override.unwrap_or(DEFAULT_DECAY_RATE),
            delta_activation,
            0.0,
        )
    };

    let settled_score = compute_link_graph_saliency(
        baseline,
        decay_rate,
        delta_activation,
        delta_days,
        normalized_policy,
    );
    let state = LinkGraphSaliencyState {
        schema: LINK_GRAPH_SALIENCY_SCHEMA_VERSION.to_string(),
        node_id: node_id.to_string(),
        saliency_base: settled_score,
        decay_rate,
        activation_count: total_activation,
        last_accessed_unix: spec.now_unix,
        current_saliency: settled_score,
        updated_at_unix: unix_seconds_to_f64(spec.now_unix),
    };
    let encoded = serde_json::to_string(&state)
        .map_err(|err| format!("failed to serialize link_graph saliency state: {err}"))?;

    redis::cmd("SET")
        .arg(&cache_key)
        .arg(encoded)
        .query::<()>(conn)
        .map_err(|err| format!("failed to SET link_graph saliency entry: {err}"))?;

    update_inbound_edge_scores(conn, node_id, key_prefix, settled_score);
    Ok(state)
}

fn direct_neighbor_node_ids(
    conn: &mut redis::Connection,
    node_id: &str,
    key_prefix: &str,
    max_neighbors_per_direction: usize,
) -> Vec<String> {
    if max_neighbors_per_direction == 0 {
        return Vec::new();
    }

    let outbound_key = edge_out_key(node_id, key_prefix);
    let mut inbound_neighbors = redis::cmd("SMEMBERS")
        .arg(edge_in_key(node_id, key_prefix))
        .query::<Vec<String>>(conn)
        .unwrap_or_default();
    inbound_neighbors.sort_unstable();
    let outbound_limit =
        isize::try_from(max_neighbors_per_direction.saturating_sub(1)).unwrap_or(isize::MAX);
    let outbound_neighbors = redis::cmd("ZREVRANGE")
        .arg(&outbound_key)
        .arg(0)
        .arg(outbound_limit)
        .query::<Vec<String>>(conn)
        .unwrap_or_default();

    let mut seen: HashSet<String> = HashSet::new();
    let mut neighbors = Vec::new();
    for neighbor in outbound_neighbors
        .into_iter()
        .chain(inbound_neighbors.into_iter())
    {
        let trimmed = neighbor.trim();
        if trimmed.is_empty() || trimmed == node_id {
            continue;
        }
        if !seen.insert(trimmed.to_string()) {
            continue;
        }
        neighbors.push(trimmed.to_string());
        if neighbors.len() >= max_neighbors_per_direction.saturating_mul(2) {
            break;
        }
    }
    neighbors
}

fn propagate_coactivation(
    conn: &mut redis::Connection,
    node_id: &str,
    key_prefix: &str,
    now_unix: i64,
    policy: LinkGraphSaliencyPolicy,
) {
    let runtime = resolve_link_graph_coactivation_runtime();
    if !runtime.enabled {
        return;
    }

    let scaled_alpha = policy.alpha * runtime.alpha_scale;
    if scaled_alpha <= f64::EPSILON {
        return;
    }

    let propagated_policy = LinkGraphSaliencyPolicy {
        alpha: scaled_alpha,
        minimum: policy.minimum,
        maximum: policy.maximum,
    }
    .normalized();
    let propagated_spec = TouchUpdateSpec {
        activation_delta: 1,
        saliency_base: None,
        decay_rate_override: None,
        policy: propagated_policy,
        now_unix,
    };

    for neighbor_id in direct_neighbor_node_ids(
        conn,
        node_id,
        key_prefix,
        runtime.max_neighbors_per_direction,
    ) {
        if let Err(error) =
            apply_touch_with_connection(conn, &neighbor_id, key_prefix, propagated_spec)
        {
            log::warn!(
                "Failed to propagate co-activation from '{node_id}' to '{neighbor_id}': {error}"
            );
        }
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
    let now_unix = now_unix.unwrap_or_else(now_unix_i64);

    let policy = normalize_policy(alpha, minimum_saliency, maximum_saliency);
    let client = redis_client(valkey_url)?;
    let mut conn = client
        .get_connection()
        .map_err(|err| format!("failed to connect valkey for link_graph saliency store: {err}"))?;

    let state = apply_touch_with_connection(
        &mut conn,
        node_id,
        prefix,
        TouchUpdateSpec {
            activation_delta,
            saliency_base,
            decay_rate_override,
            policy,
            now_unix,
        },
    )?;
    propagate_coactivation(&mut conn, node_id, prefix, now_unix, policy);
    Ok(state)
}

fn unix_seconds_to_f64(seconds: i64) -> f64 {
    u64::try_from(seconds).map_or(0.0, |value| Duration::from_secs(value).as_secs_f64())
}
