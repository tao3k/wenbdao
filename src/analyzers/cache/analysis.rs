use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use crate::analyzers::cache::RepositoryAnalysisCacheKey;
use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::RepositoryAnalysisOutput;

type RepositoryAnalysisCache = BTreeMap<RepositoryAnalysisCacheKey, RepositoryAnalysisOutput>;

static REPOSITORY_ANALYSIS_CACHE: OnceLock<Mutex<RepositoryAnalysisCache>> = OnceLock::new();

fn repository_analysis_cache() -> &'static Mutex<RepositoryAnalysisCache> {
    REPOSITORY_ANALYSIS_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Loads a cached analysis result if available.
///
/// # Errors
///
/// Returns an error when the in-memory cache lock is poisoned.
pub fn load_cached_repository_analysis(
    key: &RepositoryAnalysisCacheKey,
) -> Result<Option<RepositoryAnalysisOutput>, RepoIntelligenceError> {
    repository_analysis_cache()
        .lock()
        .map_err(|_| RepoIntelligenceError::AnalysisFailed {
            message: "repository analysis cache lock is poisoned".to_string(),
        })
        .map(|cache| cache.get(key).cloned())
}

/// Stores an analysis result in the cache.
///
/// # Errors
///
/// Returns an error when the in-memory cache lock is poisoned.
pub fn store_cached_repository_analysis(
    key: RepositoryAnalysisCacheKey,
    output: &RepositoryAnalysisOutput,
) -> Result<(), RepoIntelligenceError> {
    repository_analysis_cache()
        .lock()
        .map_err(|_| RepoIntelligenceError::AnalysisFailed {
            message: "repository analysis cache lock is poisoned".to_string(),
        })
        .map(|mut cache| {
            cache.insert(key, output.clone());
        })
}
