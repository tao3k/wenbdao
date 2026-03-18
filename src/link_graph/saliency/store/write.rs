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
use std::collections::{HashMap, HashSet};
use std::time::Duration;

const OUTBOUND_COACTIVATION_DIRECTION_SCALE: f64 = 1.0;
const INBOUND_COACTIVATION_DIRECTION_SCALE: f64 = 0.25;

#[derive(Debug, Clone, Copy)]
struct TouchUpdateSpec {
    activation_delta: u64,
    saliency_base: Option<f64>,
    decay_rate_override: Option<f64>,
    policy: LinkGraphSaliencyPolicy,
    now_unix: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoactivationNeighborDirection {
    Outbound,
    Inbound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CoactivationNeighbor {
    node_id: String,
    direction: CoactivationNeighborDirection,
    rank: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct CoactivationPropagationTarget {
    node_id: String,
    hop: usize,
    weight: f64,
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

fn direct_coactivation_neighbors(
    conn: &mut redis::Connection,
    node_id: &str,
    key_prefix: &str,
    max_neighbors_per_direction: usize,
) -> Vec<CoactivationNeighbor> {
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
    let mut neighbors: Vec<CoactivationNeighbor> = Vec::new();
    for (rank, neighbor) in outbound_neighbors.into_iter().enumerate() {
        let trimmed = neighbor.trim();
        if trimmed.is_empty() || trimmed == node_id {
            continue;
        }
        if !seen.insert(trimmed.to_string()) {
            continue;
        }
        neighbors.push(CoactivationNeighbor {
            node_id: trimmed.to_string(),
            direction: CoactivationNeighborDirection::Outbound,
            rank,
        });
    }

    for (rank, neighbor) in inbound_neighbors.into_iter().enumerate() {
        let trimmed = neighbor.trim();
        if trimmed.is_empty() || trimmed == node_id {
            continue;
        }
        if !seen.insert(trimmed.to_string()) {
            continue;
        }
        neighbors.push(CoactivationNeighbor {
            node_id: trimmed.to_string(),
            direction: CoactivationNeighborDirection::Inbound,
            rank,
        });
        if neighbors.len() >= max_neighbors_per_direction.saturating_mul(2) {
            break;
        }
    }
    neighbors
}

fn coactivation_weight_for_neighbor(neighbor: &CoactivationNeighbor) -> f64 {
    let rank_f64 = f64::from(u32::try_from(neighbor.rank).unwrap_or(u32::MAX));
    let rank_scale = 1.0 / (rank_f64 + 1.0);
    let direction_scale = match neighbor.direction {
        CoactivationNeighborDirection::Outbound => OUTBOUND_COACTIVATION_DIRECTION_SCALE,
        CoactivationNeighborDirection::Inbound => INBOUND_COACTIVATION_DIRECTION_SCALE,
    };
    rank_scale * direction_scale
}

fn bounded_coactivation_targets(
    conn: &mut redis::Connection,
    node_id: &str,
    key_prefix: &str,
    runtime: &crate::link_graph::runtime_config::models::LinkGraphCoactivationRuntimeConfig,
) -> Vec<CoactivationPropagationTarget> {
    let max_hops = runtime.max_hops.clamp(1, 2);
    let total_budget = runtime.max_total_propagations;
    if total_budget == 0 {
        return Vec::new();
    }

    let mut seen: HashSet<String> = HashSet::from([node_id.to_string()]);
    let mut targets: Vec<CoactivationPropagationTarget> = Vec::new();
    let mut frontier: Vec<(String, f64)> = vec![(node_id.to_string(), 1.0)];

    for hop in 1..=max_hops {
        if frontier.is_empty() || targets.len() >= total_budget {
            break;
        }

        let mut next_frontier_weights: HashMap<String, f64> = HashMap::new();
        for (source_node_id, source_weight) in frontier {
            for neighbor in direct_coactivation_neighbors(
                conn,
                &source_node_id,
                key_prefix,
                runtime.max_neighbors_per_direction,
            ) {
                if seen.contains(&neighbor.node_id) {
                    continue;
                }

                let edge_weight = coactivation_weight_for_neighbor(&neighbor);
                if edge_weight <= f64::EPSILON {
                    continue;
                }

                let hop_weight = if hop <= 1 {
                    source_weight * edge_weight
                } else {
                    source_weight * runtime.hop_decay_scale * edge_weight
                };
                if hop_weight <= f64::EPSILON {
                    continue;
                }

                next_frontier_weights
                    .entry(neighbor.node_id)
                    .and_modify(|weight| *weight = weight.max(hop_weight))
                    .or_insert(hop_weight);
            }
        }

        let mut hop_targets: Vec<CoactivationPropagationTarget> = next_frontier_weights
            .into_iter()
            .map(|(target_node_id, weight)| CoactivationPropagationTarget {
                node_id: target_node_id,
                hop,
                weight,
            })
            .collect();
        hop_targets.sort_by(|left, right| {
            right
                .weight
                .partial_cmp(&left.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.node_id.cmp(&right.node_id))
        });

        let remaining_budget = total_budget.saturating_sub(targets.len());
        if remaining_budget == 0 {
            break;
        }
        hop_targets.truncate(remaining_budget);

        for target in &hop_targets {
            seen.insert(target.node_id.clone());
        }
        frontier = hop_targets
            .iter()
            .map(|target| (target.node_id.clone(), target.weight))
            .collect();
        targets.extend(hop_targets);
    }

    targets
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

    for target in bounded_coactivation_targets(conn, node_id, key_prefix, &runtime) {
        let alpha_scale = target.weight;
        if alpha_scale <= f64::EPSILON {
            continue;
        }
        let weighted_policy = LinkGraphSaliencyPolicy {
            alpha: propagated_spec.policy.alpha * alpha_scale,
            ..propagated_spec.policy
        }
        .normalized();
        let weighted_spec = TouchUpdateSpec {
            policy: weighted_policy,
            ..propagated_spec
        };
        if let Err(error) =
            apply_touch_with_connection(conn, &target.node_id, key_prefix, weighted_spec)
        {
            log::warn!(
                "Failed to propagate co-activation from '{node_id}' to '{neighbor_id}' at hop {hop}: {error}",
                neighbor_id = target.node_id,
                hop = target.hop
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
