use crate::analyzers::config::RegisteredRepository;
use crate::git::checkout::{LocalCheckoutMetadata, ResolvedRepositorySource};
use crate::search::FuzzySearchOptions;

/// Cache key for repository analysis results.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepositoryAnalysisCacheKey {
    /// Repository identifier.
    pub repo_id: String,
    /// Root path of the checkout.
    pub checkout_root: String,
    /// Revision of the checkout.
    pub checkout_revision: Option<String>,
    /// Revision of the mirror.
    pub mirror_revision: Option<String>,
    /// Revision being tracked.
    pub tracking_revision: Option<String>,
    /// Sorted list of plugin identifiers used.
    pub plugin_ids: Vec<String>,
}

/// Cache key for final repo-search endpoint payloads.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepositorySearchQueryCacheKey {
    /// The underlying analysis cache identity.
    pub analysis_key: RepositoryAnalysisCacheKey,
    /// Stable endpoint identifier.
    pub endpoint: String,
    /// Raw query text.
    pub query: String,
    /// Optional endpoint-specific filter such as projected-page kind.
    pub filter: Option<String>,
    /// Maximum edit distance for the search profile.
    pub max_distance: u8,
    /// Required shared prefix length for the search profile.
    pub prefix_length: usize,
    /// Whether transpositions are allowed for the search profile.
    pub transposition: bool,
    /// Result limit.
    pub limit: usize,
}

impl RepositorySearchQueryCacheKey {
    /// Build one endpoint cache key from the shared analysis identity plus query settings.
    #[must_use]
    pub fn new(
        analysis_key: &RepositoryAnalysisCacheKey,
        endpoint: &str,
        query: &str,
        filter: Option<String>,
        options: FuzzySearchOptions,
        limit: usize,
    ) -> Self {
        Self {
            analysis_key: analysis_key.clone(),
            endpoint: endpoint.to_string(),
            query: query.to_string(),
            filter,
            max_distance: options.max_distance,
            prefix_length: options.prefix_length,
            transposition: options.transposition,
            limit,
        }
    }
}

/// Builds a cache key from repository configuration and resolved source.
#[must_use]
pub fn build_repository_analysis_cache_key(
    repository: &RegisteredRepository,
    source: &ResolvedRepositorySource,
    metadata: Option<&LocalCheckoutMetadata>,
) -> RepositoryAnalysisCacheKey {
    let mut plugin_ids = repository
        .plugins
        .iter()
        .map(|plugin| plugin.id().to_string())
        .collect::<Vec<_>>();
    plugin_ids.sort_unstable();
    plugin_ids.dedup();

    RepositoryAnalysisCacheKey {
        repo_id: repository.id.clone(),
        checkout_root: source.checkout_root.display().to_string(),
        checkout_revision: metadata.and_then(|item| item.revision.clone()),
        mirror_revision: source.mirror_revision.clone(),
        tracking_revision: source.tracking_revision.clone(),
        plugin_ids,
    }
}
