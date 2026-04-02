use std::collections::BTreeMap;

use redis::AsyncCommands;
use serde::de::DeserializeOwned;

use crate::search_plane::cache::SearchPlaneCache;
use crate::search_plane::{
    SearchCorpusKind, SearchFileFingerprint, SearchRepoCorpusRecord, SearchRepoCorpusSnapshotRecord,
};

impl SearchPlaneCache {
    pub(crate) async fn get_json<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let client = self.client.as_ref()?;
        let mut connection = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
            .ok()?;
        let payload: Option<String> = connection.get(key).await.ok()?;
        serde_json::from_str(payload?.as_str()).ok()
    }

    pub(crate) async fn get_repo_corpus_record(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoCorpusRecord> {
        #[cfg(test)]
        if let Some(record) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_records
            .get(&(corpus, repo_id.to_string()))
            .cloned()
        {
            return Some(record);
        }
        let key = self.keyspace.repo_corpus_record_key(corpus, repo_id);
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn get_repo_corpus_snapshot(&self) -> Option<SearchRepoCorpusSnapshotRecord> {
        #[cfg(test)]
        if let Some(record) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_snapshot
            .clone()
        {
            return Some(record);
        }
        let key = self.keyspace.repo_corpus_snapshot_key();
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn get_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
    ) -> Option<BTreeMap<String, SearchFileFingerprint>> {
        #[cfg(test)]
        if let Some(fingerprints) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .corpus_file_fingerprints
            .get(&corpus)
            .cloned()
        {
            return Some(fingerprints);
        }
        let key = self.keyspace.corpus_file_fingerprints_key(corpus);
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn get_repo_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<BTreeMap<String, SearchFileFingerprint>> {
        #[cfg(test)]
        if let Some(fingerprints) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_file_fingerprints
            .get(&(corpus, repo_id.to_string()))
            .cloned()
        {
            return Some(fingerprints);
        }
        let key = self
            .keyspace
            .repo_corpus_file_fingerprints_key(corpus, repo_id);
        self.get_json(key.as_str()).await
    }
}
