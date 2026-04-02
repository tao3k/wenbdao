use crate::analyzers::errors::RepoIntelligenceError;
use crate::valkey_common::{normalize_key_prefix, open_client};

const ANALYZER_VALKEY_URL_ENV: &str = "XIUXIAN_WENDAO_ANALYZER_VALKEY_URL";
const VALKEY_URL_ENV: &str = "VALKEY_URL";
const REDIS_URL_ENV: &str = "REDIS_URL";
const ANALYZER_VALKEY_KEY_PREFIX_ENV: &str = "XIUXIAN_WENDAO_ANALYZER_VALKEY_KEY_PREFIX";
const ANALYZER_VALKEY_TTL_ENV: &str = "XIUXIAN_WENDAO_ANALYZER_VALKEY_TTL_SECS";
const DEFAULT_ANALYZER_VALKEY_KEY_PREFIX: &str = "xiuxian_wendao:repo_analysis";

#[derive(Debug, Clone)]
pub(super) struct ValkeyAnalysisCacheRuntime {
    pub(super) client: Option<redis::Client>,
    pub(super) key_prefix: String,
    pub(super) ttl_seconds: Option<u64>,
}

impl ValkeyAnalysisCacheRuntime {
    #[cfg(test)]
    pub(super) fn for_tests(key_prefix: &str, ttl_seconds: Option<u64>) -> Self {
        Self {
            client: None,
            key_prefix: normalize_key_prefix(key_prefix, DEFAULT_ANALYZER_VALKEY_KEY_PREFIX),
            ttl_seconds,
        }
    }
}

pub(super) fn resolve_valkey_analysis_cache_runtime()
-> Result<Option<ValkeyAnalysisCacheRuntime>, RepoIntelligenceError> {
    resolve_valkey_analysis_cache_runtime_with_lookup(&|name| std::env::var(name).ok())
}

pub(super) fn resolve_valkey_analysis_cache_runtime_with_lookup(
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<ValkeyAnalysisCacheRuntime>, RepoIntelligenceError> {
    let Some((env_name, url)) = first_non_empty_named_lookup(
        &[ANALYZER_VALKEY_URL_ENV, VALKEY_URL_ENV, REDIS_URL_ENV],
        lookup,
    ) else {
        return Ok(None);
    };
    let client =
        open_client(url.as_str()).map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!("invalid analyzer valkey url from {env_name}: {error}"),
        })?;
    let key_prefix = normalize_key_prefix(
        lookup(ANALYZER_VALKEY_KEY_PREFIX_ENV)
            .unwrap_or_default()
            .as_str(),
        DEFAULT_ANALYZER_VALKEY_KEY_PREFIX,
    );
    let ttl_seconds = resolve_optional_ttl_seconds_with_lookup(lookup)?;
    Ok(Some(ValkeyAnalysisCacheRuntime {
        client: Some(client),
        key_prefix,
        ttl_seconds,
    }))
}

fn first_non_empty_named_lookup(
    names: &[&str],
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<(String, String)> {
    names.iter().find_map(|name| {
        lookup(name)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(|value| ((*name).to_string(), value))
    })
}

fn resolve_optional_ttl_seconds_with_lookup(
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<u64>, RepoIntelligenceError> {
    let Some(raw_ttl) = lookup(ANALYZER_VALKEY_TTL_ENV) else {
        return Ok(None);
    };
    let trimmed = raw_ttl.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let ttl_seconds = trimmed.parse::<u64>().map_err(|error| {
        RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "{ANALYZER_VALKEY_TTL_ENV} must be a non-negative integer, got `{trimmed}`: {error}"
            ),
        }
    })?;
    Ok((ttl_seconds > 0).then_some(ttl_seconds))
}
