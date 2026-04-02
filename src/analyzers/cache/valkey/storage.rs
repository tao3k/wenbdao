use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::analyzers::cache::RepositoryAnalysisCacheKey;
use crate::analyzers::plugin::RepositoryAnalysisOutput;

const ANALYZER_CACHE_SCHEMA_VERSION: &str = "xiuxian_wendao.repo_analysis_cache.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ValkeyAnalysisCachePayload {
    schema: String,
    repo_id: String,
    revision: String,
    cached_at_rfc3339: String,
    analysis: RepositoryAnalysisOutput,
}

pub(super) fn valkey_analysis_key(
    cache_key: &RepositoryAnalysisCacheKey,
    key_prefix: &str,
) -> Option<String> {
    let revision = stable_revision(cache_key)?;
    let payload = format!(
        "repo:{}|root:{}|revision:{}|mirror:{}|tracking:{}|plugins:{}",
        cache_key.repo_id.trim(),
        cache_key.checkout_root.trim(),
        revision,
        cache_key
            .mirror_revision
            .as_deref()
            .unwrap_or_default()
            .trim(),
        cache_key
            .tracking_revision
            .as_deref()
            .unwrap_or_default()
            .trim(),
        cache_key.plugin_ids.join(","),
    );
    let token = blake3::hash(payload.as_bytes()).to_hex().to_string();
    Some(format!("{key_prefix}:analysis:{token}"))
}

pub(super) fn encode_analysis_payload(
    cache_key: &RepositoryAnalysisCacheKey,
    analysis: &RepositoryAnalysisOutput,
) -> Option<String> {
    let revision = stable_revision(cache_key)?;
    serde_json::to_string(&ValkeyAnalysisCachePayload {
        schema: ANALYZER_CACHE_SCHEMA_VERSION.to_string(),
        repo_id: cache_key.repo_id.clone(),
        revision: revision.to_string(),
        cached_at_rfc3339: Utc::now().to_rfc3339(),
        analysis: analysis.clone(),
    })
    .ok()
}

pub(super) fn decode_analysis_payload(
    cache_key: &RepositoryAnalysisCacheKey,
    payload: &str,
) -> Option<RepositoryAnalysisOutput> {
    let revision = stable_revision(cache_key)?;
    let decoded = serde_json::from_str::<ValkeyAnalysisCachePayload>(payload).ok()?;
    if decoded.schema != ANALYZER_CACHE_SCHEMA_VERSION {
        return None;
    }
    if decoded.repo_id != cache_key.repo_id || decoded.revision != revision {
        return None;
    }
    Some(decoded.analysis)
}

pub(super) fn stable_revision(cache_key: &RepositoryAnalysisCacheKey) -> Option<&str> {
    cache_key
        .checkout_revision
        .as_deref()
        .or(cache_key.mirror_revision.as_deref())
        .or(cache_key.tracking_revision.as_deref())
}
