use crate::link_graph::models::{QuantumAnchorHit, QuantumContext, QuantumFusionOptions};
use crate::link_graph::runtime_config::{
    DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX, resolve_link_graph_cache_runtime,
};
use crate::schemas::LINK_GRAPH_QUANTUM_CONTEXT_SNAPSHOT_V1;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Schema version identifier for persisted quantum-context snapshots.
pub const LINK_GRAPH_QUANTUM_CONTEXT_SNAPSHOT_SCHEMA_VERSION: &str =
    "xiuxian_wendao.link_graph.quantum_context_snapshot.v1";

static LINK_GRAPH_QUANTUM_CONTEXT_SNAPSHOT_SCHEMA_FINGERPRINT: OnceLock<String> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
/// Persisted snapshot of quantum-fusion contexts for rollback.
pub struct QuantumContextSnapshot {
    /// Schema version tag.
    pub schema_version: String,
    /// Schema fingerprint derived from canonical JSON schema.
    pub schema_fingerprint: String,
    /// Deterministic snapshot identifier.
    pub snapshot_id: String,
    /// Optional user query text associated with the snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_text: Option<String>,
    /// Snapshot creation time (unix seconds).
    pub saved_at_unix: f64,
    /// Anchor hits used to build the snapshot.
    pub anchors: Vec<QuantumAnchorHit>,
    /// Fusion options used to score contexts.
    pub options: QuantumFusionOptions,
    /// Fused contexts captured in this snapshot.
    pub contexts: Vec<QuantumContext>,
}

#[derive(Serialize)]
struct QuantumContextSnapshotKey<'a> {
    query_text: &'a str,
    anchors: &'a [QuantumAnchorHit],
    options: &'a QuantumFusionOptions,
}

impl QuantumContextSnapshot {
    /// Build a snapshot payload from the provided inputs.
    #[must_use]
    pub fn new(
        snapshot_id: impl Into<String>,
        query_text: Option<&str>,
        anchors: Vec<QuantumAnchorHit>,
        options: QuantumFusionOptions,
        contexts: Vec<QuantumContext>,
    ) -> Self {
        let saved_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |delta| delta.as_secs_f64());
        Self {
            schema_version: LINK_GRAPH_QUANTUM_CONTEXT_SNAPSHOT_SCHEMA_VERSION.to_string(),
            schema_fingerprint: quantum_context_snapshot_schema_fingerprint().to_string(),
            snapshot_id: snapshot_id.into(),
            query_text: query_text
                .and_then(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                })
                .map(str::to_string),
            saved_at_unix,
            anchors,
            options,
            contexts,
        }
    }
}

/// Compute a deterministic snapshot id for a query + anchors payload.
///
/// # Errors
///
/// Returns an error when the query text is empty or JSON serialization fails.
pub fn quantum_context_snapshot_id(
    query_text: &str,
    anchors: &[QuantumAnchorHit],
    options: &QuantumFusionOptions,
) -> Result<String, String> {
    let trimmed = query_text.trim();
    if trimmed.is_empty() {
        return Err("query_text must be non-empty".to_string());
    }
    let payload = QuantumContextSnapshotKey {
        query_text: trimmed,
        anchors,
        options,
    };
    let encoded = serde_json::to_string(&payload)
        .map_err(|err| format!("failed to encode quantum-context snapshot key: {err}"))?;
    let mut hasher = DefaultHasher::new();
    encoded.hash(&mut hasher);
    LINK_GRAPH_QUANTUM_CONTEXT_SNAPSHOT_SCHEMA_VERSION.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

fn quantum_context_snapshot_schema_fingerprint() -> &'static str {
    LINK_GRAPH_QUANTUM_CONTEXT_SNAPSHOT_SCHEMA_FINGERPRINT.get_or_init(|| {
        let mut hasher = DefaultHasher::new();
        LINK_GRAPH_QUANTUM_CONTEXT_SNAPSHOT_V1.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    })
}

fn snapshot_cache_key(snapshot_id: &str, key_prefix: &str) -> String {
    format!("{key_prefix}:quantum_context:snapshot:{snapshot_id}")
}

fn resolve_snapshot_runtime() -> Result<(String, String), String> {
    let runtime = resolve_link_graph_cache_runtime()?;
    let key_prefix = if runtime.key_prefix.trim().is_empty() {
        DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX.to_string()
    } else {
        runtime.key_prefix
    };
    Ok((runtime.valkey_url, key_prefix))
}

fn decode_snapshot_payload(raw: &str, snapshot_id: &str) -> Option<QuantumContextSnapshot> {
    let parsed = serde_json::from_str::<QuantumContextSnapshot>(raw).ok()?;
    if parsed.schema_version != LINK_GRAPH_QUANTUM_CONTEXT_SNAPSHOT_SCHEMA_VERSION {
        return None;
    }
    if parsed.schema_fingerprint != quantum_context_snapshot_schema_fingerprint() {
        return None;
    }
    if parsed.snapshot_id != snapshot_id {
        return None;
    }
    Some(parsed)
}

/// Read a quantum-context snapshot using runtime-configured Valkey settings.
///
/// # Errors
///
/// Returns an error when runtime configuration is invalid or Valkey operations fail.
pub fn valkey_quantum_context_snapshot_get(
    snapshot_id: &str,
) -> Result<Option<QuantumContextSnapshot>, String> {
    let (valkey_url, key_prefix) = resolve_snapshot_runtime()?;
    valkey_quantum_context_snapshot_get_with_valkey(snapshot_id, &valkey_url, Some(&key_prefix))
}

/// Read a quantum-context snapshot from a specific Valkey endpoint.
///
/// # Errors
///
/// Returns an error when inputs are invalid or Valkey operations fail.
pub fn valkey_quantum_context_snapshot_get_with_valkey(
    snapshot_id: &str,
    valkey_url: &str,
    key_prefix: Option<&str>,
) -> Result<Option<QuantumContextSnapshot>, String> {
    let trimmed = snapshot_id.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if valkey_url.trim().is_empty() {
        return Err("link_graph snapshot valkey_url must be non-empty".to_string());
    }
    let prefix = key_prefix
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX);
    let cache_key = snapshot_cache_key(trimmed, prefix);

    let client = redis::Client::open(valkey_url)
        .map_err(|err| format!("invalid valkey url for link_graph snapshot store: {err}"))?;
    let mut conn = client
        .get_connection()
        .map_err(|err| format!("failed to connect valkey for link_graph snapshot store: {err}"))?;
    let raw = redis::cmd("GET")
        .arg(&cache_key)
        .query::<Option<String>>(&mut conn)
        .map_err(|err| format!("failed to GET link_graph snapshot: {err}"))?;
    let Some(payload_raw) = raw else {
        return Ok(None);
    };

    if let Some(snapshot) = decode_snapshot_payload(&payload_raw, trimmed) {
        return Ok(Some(snapshot));
    }

    let _ = redis::cmd("DEL").arg(&cache_key).query::<i64>(&mut conn);
    Ok(None)
}

/// Write a quantum-context snapshot using runtime-configured Valkey settings.
///
/// # Errors
///
/// Returns an error when runtime configuration is invalid or Valkey operations fail.
pub fn valkey_quantum_context_snapshot_save(
    snapshot: &QuantumContextSnapshot,
) -> Result<(), String> {
    let (valkey_url, key_prefix) = resolve_snapshot_runtime()?;
    valkey_quantum_context_snapshot_save_with_valkey(snapshot, &valkey_url, Some(&key_prefix))
}

/// Write a quantum-context snapshot to a specific Valkey endpoint.
///
/// # Errors
///
/// Returns an error when inputs are invalid, serialization fails, or Valkey operations fail.
pub fn valkey_quantum_context_snapshot_save_with_valkey(
    snapshot: &QuantumContextSnapshot,
    valkey_url: &str,
    key_prefix: Option<&str>,
) -> Result<(), String> {
    let snapshot_id = snapshot.snapshot_id.trim();
    if snapshot_id.is_empty() {
        return Err("snapshot_id must be non-empty".to_string());
    }
    if valkey_url.trim().is_empty() {
        return Err("link_graph snapshot valkey_url must be non-empty".to_string());
    }
    let prefix = key_prefix
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX);
    let cache_key = snapshot_cache_key(snapshot_id, prefix);
    let encoded = serde_json::to_string(snapshot)
        .map_err(|err| format!("failed to serialize link_graph snapshot: {err}"))?;

    let client = redis::Client::open(valkey_url)
        .map_err(|err| format!("invalid valkey url for link_graph snapshot store: {err}"))?;
    let mut conn = client
        .get_connection()
        .map_err(|err| format!("failed to connect valkey for link_graph snapshot store: {err}"))?;
    redis::cmd("SET")
        .arg(&cache_key)
        .arg(&encoded)
        .query::<()>(&mut conn)
        .map_err(|err| format!("failed to SET link_graph snapshot: {err}"))?;
    Ok(())
}

/// Delete a quantum-context snapshot using runtime-configured Valkey settings.
///
/// # Errors
///
/// Returns an error when runtime configuration is invalid or Valkey operations fail.
pub fn valkey_quantum_context_snapshot_drop(snapshot_id: &str) -> Result<(), String> {
    let (valkey_url, key_prefix) = resolve_snapshot_runtime()?;
    let trimmed = snapshot_id.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let cache_key = snapshot_cache_key(trimmed, &key_prefix);

    let client = redis::Client::open(valkey_url.as_str())
        .map_err(|err| format!("invalid valkey url for link_graph snapshot store: {err}"))?;
    let mut conn = client
        .get_connection()
        .map_err(|err| format!("failed to connect valkey for link_graph snapshot store: {err}"))?;
    redis::cmd("DEL")
        .arg(&cache_key)
        .query::<i64>(&mut conn)
        .map_err(|err| format!("failed to DEL link_graph snapshot: {err}"))?;
    Ok(())
}

/// Roll back to a stored quantum-context snapshot by id.
///
/// # Errors
///
/// Returns an error when runtime configuration is invalid or Valkey operations fail.
pub fn valkey_quantum_context_snapshot_rollback(
    snapshot_id: &str,
) -> Result<Option<Vec<QuantumContext>>, String> {
    let (valkey_url, key_prefix) = resolve_snapshot_runtime()?;
    valkey_quantum_context_snapshot_rollback_with_valkey(
        snapshot_id,
        &valkey_url,
        Some(&key_prefix),
    )
}

/// Roll back to a stored quantum-context snapshot using an explicit Valkey endpoint.
///
/// # Errors
///
/// Returns an error when inputs are invalid or Valkey operations fail.
pub fn valkey_quantum_context_snapshot_rollback_with_valkey(
    snapshot_id: &str,
    valkey_url: &str,
    key_prefix: Option<&str>,
) -> Result<Option<Vec<QuantumContext>>, String> {
    Ok(
        valkey_quantum_context_snapshot_get_with_valkey(snapshot_id, valkey_url, key_prefix)?
            .map(|snapshot| snapshot.contexts),
    )
}
