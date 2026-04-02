use serde::Deserialize;

/// Basic repository query parameters.
#[derive(Debug, Deserialize)]
pub struct RepoApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
}

/// Query parameters for repository-wide search.
#[derive(Debug, Deserialize)]
pub struct RepoSearchApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The search query string.
    pub query: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
}

/// Query parameters for repository import search.
#[derive(Debug, Deserialize)]
pub struct RepoImportSearchApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// Optional target package filter.
    pub package: Option<String>,
    /// Optional source-module filter.
    pub module: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
}

/// Query parameters for projected page lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
}

/// Query parameters for projected page-index node lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageIndexNodeApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// The page-index node identifier.
    pub node_id: Option<String>,
}

/// Query parameters for projected retrieval hit lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedRetrievalHitApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// The page-index node identifier.
    pub node_id: Option<String>,
}

/// Query parameters for projected retrieval context lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedRetrievalContextApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// The page-index node identifier.
    pub node_id: Option<String>,
    /// Maximum number of related hits to return.
    pub related_limit: Option<usize>,
}

/// Query parameters for projected page-family context lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageFamilyContextApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// Maximum number of items per kind to return.
    pub per_kind_limit: Option<usize>,
}

/// Query parameters for projected page-family cluster search.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageFamilySearchApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The search query string.
    pub query: Option<String>,
    /// The projected page kind filter.
    pub kind: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
    /// Maximum number of items per kind to return.
    pub per_kind_limit: Option<usize>,
}

/// Query parameters for projected page-family cluster lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageFamilyClusterApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// The projected page kind filter.
    pub kind: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
}

/// Query parameters for projected page navigation bundle lookup.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageNavigationApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The projected page identifier.
    pub page_id: Option<String>,
    /// The focus node identifier.
    pub node_id: Option<String>,
    /// The family kind filter.
    pub family_kind: Option<String>,
    /// Maximum number of related hits to return.
    pub related_limit: Option<usize>,
    /// Maximum number of family items to return.
    pub family_limit: Option<usize>,
}

/// Query parameters for projected page navigation search.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageNavigationSearchApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The search query string.
    pub query: Option<String>,
    /// The projected page kind filter.
    pub kind: Option<String>,
    /// The family kind filter.
    pub family_kind: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
    /// Maximum number of related hits to return.
    pub related_limit: Option<usize>,
    /// Maximum number of family items to return.
    pub family_limit: Option<usize>,
}

/// Query parameters for projected-page search.
#[derive(Debug, Deserialize)]
pub struct RepoProjectedPageSearchApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The search query string.
    pub query: Option<String>,
    /// The projected page kind filter.
    pub kind: Option<String>,
    /// Maximum number of hits to return.
    pub limit: Option<usize>,
}

/// Query parameters for documentation coverage inspection.
#[derive(Debug, Deserialize)]
pub struct RepoDocCoverageApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// Optional module identifier filter.
    #[serde(rename = "module")]
    pub module_id: Option<String>,
}

/// Query parameters for repository source synchronization.
#[derive(Debug, Deserialize)]
pub struct RepoSyncApiQuery {
    /// The repository identifier.
    pub repo: Option<String>,
    /// The synchronization mode ("ensure", "refresh", or "status").
    pub mode: Option<String>,
}

/// Query parameters for repo index status.
#[derive(Debug, Deserialize)]
pub struct RepoIndexStatusApiQuery {
    /// Optional repository identifier filter.
    pub repo: Option<String>,
}
