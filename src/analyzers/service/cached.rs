use std::path::Path;

use crate::analyzers::cache::{
    ValkeyAnalysisCache, build_repository_analysis_cache_key, load_cached_repository_analysis,
    store_cached_repository_analysis,
};
use crate::analyzers::config::RegisteredRepository;
use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::{AnalysisContext, RepositoryAnalysisOutput};
use crate::analyzers::registry::PluginRegistry;
use crate::git::checkout::{
    CheckoutSyncMode, discover_checkout_metadata, resolve_repository_source,
};

/// Ready cached repository analysis plus its stable cache identity.
#[derive(Clone)]
pub struct CachedRepositoryAnalysis {
    /// Stable cache identity for the resolved repository snapshot.
    pub cache_key: crate::analyzers::cache::RepositoryAnalysisCacheKey,
    /// Cached repository analysis output.
    pub analysis: RepositoryAnalysisOutput,
}

/// Load repository analysis from ready caches only.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::PendingRepositoryIndex`] when no ready cache exists yet.
pub fn analyze_registered_repository_cached_with_registry(
    repository: &RegisteredRepository,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    analyze_registered_repository_cached_bundle_with_registry(repository, cwd, registry)
        .map(|cached| cached.analysis)
}

/// Load repository analysis from ready caches only and preserve the stable cache key.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::PendingRepositoryIndex`] when no ready cache exists yet.
pub fn analyze_registered_repository_cached_bundle_with_registry(
    repository: &RegisteredRepository,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<CachedRepositoryAnalysis, RepoIntelligenceError> {
    if repository.plugins.is_empty() {
        return Err(RepoIntelligenceError::MissingRequiredPlugin {
            repo_id: repository.id.clone(),
            plugin_id: "any".to_string(),
        });
    }

    let repository_source = resolve_repository_source(repository, cwd, CheckoutSyncMode::Status)?;
    let repository_root = repository_source.checkout_root.clone();
    let analysis_context = AnalysisContext {
        repository: repository.clone(),
        repository_root: repository_root.clone(),
    };
    for plugin in registry.resolve_for_repository(repository)? {
        plugin.preflight_repository(&analysis_context, repository_root.as_path())?;
    }

    let checkout_metadata = discover_checkout_metadata(repository_root.as_path());
    let cache_key = build_repository_analysis_cache_key(
        repository,
        &repository_source,
        checkout_metadata.as_ref(),
    );
    if let Some(cached) = load_cached_repository_analysis(&cache_key)? {
        return Ok(CachedRepositoryAnalysis {
            cache_key,
            analysis: cached,
        });
    }

    let valkey_cache = ValkeyAnalysisCache::new()?;
    if let Some(ref cache) = valkey_cache {
        if let Some(cached) = cache.get(&cache_key) {
            store_cached_repository_analysis(cache_key.clone(), &cached)?;
            return Ok(CachedRepositoryAnalysis {
                cache_key,
                analysis: cached,
            });
        }
    }

    Err(RepoIntelligenceError::PendingRepositoryIndex {
        repo_id: repository.id.clone(),
    })
}
