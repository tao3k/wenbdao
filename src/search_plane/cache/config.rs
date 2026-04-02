use std::time::Duration;

const QUERY_CACHE_TTL_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_QUERY_CACHE_TTL_SEC";
const AUTOCOMPLETE_CACHE_TTL_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_AUTOCOMPLETE_CACHE_TTL_SEC";
const CACHE_CONNECTION_TIMEOUT_MS_ENV: &str =
    "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_CONNECTION_TIMEOUT_MS";
const CACHE_RESPONSE_TIMEOUT_MS_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_RESPONSE_TIMEOUT_MS";

const DEFAULT_QUERY_CACHE_TTL_SEC: u64 = 90;
const DEFAULT_AUTOCOMPLETE_CACHE_TTL_SEC: u64 = 300;
const DEFAULT_CACHE_CONNECTION_TIMEOUT_MS: u64 = 25;
const DEFAULT_CACHE_RESPONSE_TIMEOUT_MS: u64 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchPlaneCacheTtl {
    HotQuery,
    Autocomplete,
}

impl SearchPlaneCacheTtl {
    pub(crate) fn as_seconds(self, config: &SearchPlaneCacheConfig) -> u64 {
        match self {
            Self::HotQuery => config.query_ttl_seconds,
            Self::Autocomplete => config.autocomplete_ttl_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchPlaneCacheConfig {
    pub(crate) query_ttl_seconds: u64,
    pub(crate) autocomplete_ttl_seconds: u64,
    pub(crate) connection_timeout: Duration,
    pub(crate) response_timeout: Duration,
}

impl Default for SearchPlaneCacheConfig {
    fn default() -> Self {
        Self {
            query_ttl_seconds: DEFAULT_QUERY_CACHE_TTL_SEC,
            autocomplete_ttl_seconds: DEFAULT_AUTOCOMPLETE_CACHE_TTL_SEC,
            connection_timeout: Duration::from_millis(DEFAULT_CACHE_CONNECTION_TIMEOUT_MS),
            response_timeout: Duration::from_millis(DEFAULT_CACHE_RESPONSE_TIMEOUT_MS),
        }
    }
}

impl SearchPlaneCacheConfig {
    pub(crate) fn from_env() -> Self {
        Self {
            query_ttl_seconds: parse_env_u64(QUERY_CACHE_TTL_ENV)
                .unwrap_or(DEFAULT_QUERY_CACHE_TTL_SEC),
            autocomplete_ttl_seconds: parse_env_u64(AUTOCOMPLETE_CACHE_TTL_ENV)
                .unwrap_or(DEFAULT_AUTOCOMPLETE_CACHE_TTL_SEC),
            connection_timeout: Duration::from_millis(
                parse_env_u64(CACHE_CONNECTION_TIMEOUT_MS_ENV)
                    .unwrap_or(DEFAULT_CACHE_CONNECTION_TIMEOUT_MS),
            ),
            response_timeout: Duration::from_millis(
                parse_env_u64(CACHE_RESPONSE_TIMEOUT_MS_ENV)
                    .unwrap_or(DEFAULT_CACHE_RESPONSE_TIMEOUT_MS),
            ),
        }
    }
}

fn parse_env_u64(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
}
