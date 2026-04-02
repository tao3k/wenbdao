use crate::search_plane::RepoSearchPublicationState;

#[cfg(test)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ParsedCodeSearchQuery {
    pub(crate) query: String,
    pub(crate) repo: Option<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) kinds: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ParsedRepoCodeSearchQuery {
    pub(crate) language_filters: std::collections::HashSet<String>,
    pub(crate) kind_filters: std::collections::HashSet<String>,
    pub(crate) search_term: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RepoSearchTarget {
    pub(crate) repo_id: String,
    pub(crate) publication_state: RepoSearchPublicationState,
}

#[derive(Debug, Default)]
pub(crate) struct RepoSearchDispatch {
    pub(crate) searchable_repos: Vec<RepoSearchTarget>,
    pub(crate) pending_repos: Vec<String>,
    pub(crate) skipped_repos: Vec<String>,
}

impl ParsedRepoCodeSearchQuery {
    pub(crate) fn search_term(&self) -> Option<&str> {
        self.search_term.as_deref()
    }
}
