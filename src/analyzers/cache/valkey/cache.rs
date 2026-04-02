#[cfg(test)]
use std::collections::BTreeMap;

use crate::analyzers::cache::RepositoryAnalysisCacheKey;
use crate::analyzers::plugin::RepositoryAnalysisOutput;

use super::runtime::{ValkeyAnalysisCacheRuntime, resolve_valkey_analysis_cache_runtime};
use super::storage::{decode_analysis_payload, encode_analysis_payload, valkey_analysis_key};

#[derive(Debug, Clone)]
pub struct ValkeyAnalysisCache {
    runtime: ValkeyAnalysisCacheRuntime,
    #[cfg(test)]
    shadow: std::sync::Arc<std::sync::RwLock<BTreeMap<String, String>>>,
}

impl ValkeyAnalysisCache {
    /// Creates a new Valkey cache client if configured.
    ///
    /// # Errors
    ///
    /// Returns an error when Valkey runtime configuration is invalid.
    pub fn new() -> Result<Option<Self>, crate::analyzers::errors::RepoIntelligenceError> {
        Ok(resolve_valkey_analysis_cache_runtime()?.map(Self::from_runtime))
    }

    #[cfg(test)]
    pub(crate) fn for_tests(key_prefix: &str, ttl_seconds: Option<u64>) -> Self {
        Self::from_runtime(ValkeyAnalysisCacheRuntime::for_tests(
            key_prefix,
            ttl_seconds,
        ))
    }

    /// Retrieves a cached analysis result.
    #[must_use]
    pub fn get(&self, cache_key: &RepositoryAnalysisCacheKey) -> Option<RepositoryAnalysisOutput> {
        let storage_key = valkey_analysis_key(cache_key, self.runtime.key_prefix.as_str())?;
        #[cfg(test)]
        if self.runtime.client.is_none() {
            return self
                .shadow
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&storage_key)
                .and_then(|payload| decode_analysis_payload(cache_key, payload));
        }
        let client = self.runtime.client.as_ref()?;
        let mut connection = client.get_connection().ok()?;
        let payload = redis::cmd("GET")
            .arg(&storage_key)
            .query::<Option<String>>(&mut connection)
            .ok()??;
        decode_analysis_payload(cache_key, payload.as_str())
    }

    /// Stores an analysis result in the cache.
    pub fn set(&self, cache_key: &RepositoryAnalysisCacheKey, analysis: &RepositoryAnalysisOutput) {
        let Some(storage_key) = valkey_analysis_key(cache_key, self.runtime.key_prefix.as_str())
        else {
            return;
        };
        let Some(payload) = encode_analysis_payload(cache_key, analysis) else {
            return;
        };
        #[cfg(test)]
        if self.runtime.client.is_none() {
            self.shadow
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(storage_key, payload);
            return;
        }
        let Some(client) = self.runtime.client.as_ref() else {
            return;
        };
        let Ok(mut connection) = client.get_connection() else {
            return;
        };
        if let Some(ttl_seconds) = self.runtime.ttl_seconds {
            let _ = redis::cmd("SETEX")
                .arg(&storage_key)
                .arg(ttl_seconds)
                .arg(&payload)
                .query::<()>(&mut connection);
            return;
        }
        let _ = redis::cmd("SET")
            .arg(&storage_key)
            .arg(&payload)
            .query::<()>(&mut connection);
    }

    fn from_runtime(runtime: ValkeyAnalysisCacheRuntime) -> Self {
        Self {
            runtime,
            #[cfg(test)]
            shadow: std::sync::Arc::new(std::sync::RwLock::new(BTreeMap::new())),
        }
    }
}
