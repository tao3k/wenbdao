use super::types::{RepoRuntimeState, RepoSearchQueryCacheKeyInput, SearchPlaneService};
use crate::search_plane::service::helpers::{
    repo_corpus_cache_version, repo_publication_cache_version,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneCacheTtl};

impl SearchPlaneService {
    #[must_use]
    pub(crate) fn corpus_active_epoch(&self, corpus: SearchCorpusKind) -> Option<u64> {
        self.coordinator.status_for(corpus).active_epoch
    }

    #[must_use]
    pub(crate) fn autocomplete_cache_key(&self, prefix: &str, limit: usize) -> Option<String> {
        let epoch = self.corpus_active_epoch(SearchCorpusKind::LocalSymbol)?;
        self.cache.autocomplete_cache_key(prefix, limit, epoch)
    }

    #[must_use]
    pub(crate) fn search_query_cache_key(
        &self,
        scope: &str,
        corpora: &[SearchCorpusKind],
        query: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Option<String> {
        let epochs = corpora
            .iter()
            .map(|corpus| {
                self.corpus_active_epoch(*corpus)
                    .map(|epoch| (*corpus, epoch))
            })
            .collect::<Option<Vec<_>>>()?;
        self.cache
            .search_query_cache_key(scope, epochs.as_slice(), query, limit, intent, repo_hint)
    }

    #[must_use]
    pub(crate) async fn repo_search_query_cache_key(
        &self,
        input: RepoSearchQueryCacheKeyInput<'_>,
    ) -> Option<String> {
        let mut versions = input
            .corpora
            .iter()
            .map(|corpus| self.corpus_cache_version(*corpus))
            .collect::<Vec<_>>();
        let mut sorted_repo_ids = input.repo_ids.to_vec();
        sorted_repo_ids.sort_unstable();
        sorted_repo_ids.dedup();
        if sorted_repo_ids.is_empty() {
            versions.push("repo_set:none".to_string());
        }
        for repo_id in sorted_repo_ids {
            for corpus in input.repo_corpora {
                if let Some(record) = self
                    .repo_corpus_record_for_reads(*corpus, repo_id.as_str())
                    .await
                {
                    let runtime = record.runtime.as_ref().map(RepoRuntimeState::from_record);
                    if let Some(publication) = record.publication.as_ref() {
                        versions.push(repo_publication_cache_version(
                            runtime.as_ref(),
                            publication,
                        ));
                    } else {
                        versions.push(repo_corpus_cache_version(
                            *corpus,
                            repo_id.as_str(),
                            runtime.as_ref(),
                        ));
                    }
                } else {
                    versions.push(repo_corpus_cache_version(*corpus, repo_id.as_str(), None));
                }
            }
        }
        self.cache.search_query_cache_key_from_versions(
            input.scope,
            versions.as_slice(),
            input.query,
            input.limit,
            input.intent,
            input.repo_hint,
        )
    }

    pub(crate) async fn cache_get_json<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.cache.get_json(key).await
    }

    pub(crate) async fn cache_set_json<T>(&self, key: &str, ttl: SearchPlaneCacheTtl, value: &T)
    where
        T: serde::Serialize,
    {
        self.cache.set_json(key, ttl, value).await;
    }
}
