use crate::gateway::studio::types::{AstSearchHit, SearchHit};

#[derive(Debug, Clone, Default)]
pub(crate) struct IntentSearchTransportMetadata {
    #[cfg(test)]
    pub(crate) repo_content_transport: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub(crate) struct IntentIndexState {
    pub(crate) knowledge_config_missing: bool,
    pub(crate) symbol_config_missing: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct IntentSourceHits {
    pub(crate) knowledge_hits: Vec<SearchHit>,
    pub(crate) local_symbol_hits: Vec<AstSearchHit>,
    pub(crate) knowledge_indexing: bool,
    pub(crate) local_symbol_indexing: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct IntentMergedResults {
    pub(crate) hits: Vec<SearchHit>,
    pub(crate) knowledge_hit_count: usize,
    pub(crate) local_symbol_hit_count: usize,
    pub(crate) repo_hit_count: usize,
    pub(crate) transport: IntentSearchTransportMetadata,
    pub(crate) partial: bool,
    pub(crate) pending_repos: Vec<String>,
    pub(crate) skipped_repos: Vec<String>,
}
