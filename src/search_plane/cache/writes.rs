use std::collections::BTreeMap;

use redis::AsyncCommands;
use serde::Serialize;

use crate::search_plane::cache::{SearchPlaneCache, SearchPlaneCacheTtl};
use crate::search_plane::{
    SearchCorpusKind, SearchFileFingerprint, SearchRepoCorpusRecord, SearchRepoCorpusSnapshotRecord,
};

impl SearchPlaneCache {
    pub(crate) async fn set_json<T>(&self, key: &str, ttl: SearchPlaneCacheTtl, value: &T)
    where
        T: Serialize,
    {
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let ttl_seconds = ttl.as_seconds(&self.config);
        if ttl_seconds == 0 {
            return;
        }
        let Ok(payload) = serde_json::to_string(value) else {
            return;
        };
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.set_ex(key, payload, ttl_seconds).await;
    }

    pub(crate) async fn set_repo_corpus_record(&self, record: &SearchRepoCorpusRecord) {
        #[cfg(test)]
        self.shadow
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_records
            .insert((record.corpus, record.repo_id.clone()), record.clone());
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let Ok(payload) = serde_json::to_string(record) else {
            return;
        };
        let key = self
            .keyspace
            .repo_corpus_record_key(record.corpus, record.repo_id.as_str());
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.set(key, payload).await;
    }

    pub(crate) async fn set_repo_corpus_snapshot(&self, record: &SearchRepoCorpusSnapshotRecord) {
        #[cfg(test)]
        {
            self.shadow
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .repo_corpus_snapshot = Some(record.clone());
        }
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let Ok(payload) = serde_json::to_string(record) else {
            return;
        };
        let key = self.keyspace.repo_corpus_snapshot_key();
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.set(key, payload).await;
    }

    pub(crate) async fn set_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        fingerprints: &BTreeMap<String, SearchFileFingerprint>,
    ) {
        #[cfg(test)]
        {
            self.shadow
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .corpus_file_fingerprints
                .insert(corpus, fingerprints.clone());
        }
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let Ok(payload) = serde_json::to_string(fingerprints) else {
            return;
        };
        let key = self.keyspace.corpus_file_fingerprints_key(corpus);
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.set(key, payload).await;
    }

    pub(crate) async fn set_repo_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
        fingerprints: &BTreeMap<String, SearchFileFingerprint>,
    ) {
        #[cfg(test)]
        {
            self.shadow
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .repo_corpus_file_fingerprints
                .insert((corpus, repo_id.to_string()), fingerprints.clone());
        }
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let Ok(payload) = serde_json::to_string(fingerprints) else {
            return;
        };
        let key = self
            .keyspace
            .repo_corpus_file_fingerprints_key(corpus, repo_id);
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.set(key, payload).await;
    }

    pub(crate) async fn delete_repo_corpus_record(&self, corpus: SearchCorpusKind, repo_id: &str) {
        #[cfg(test)]
        self.shadow
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_records
            .remove(&(corpus, repo_id.to_string()));
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let key = self.keyspace.repo_corpus_record_key(corpus, repo_id);
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.del(key).await;
    }

    pub(crate) async fn delete_repo_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) {
        #[cfg(test)]
        self.shadow
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_file_fingerprints
            .remove(&(corpus, repo_id.to_string()));
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let key = self
            .keyspace
            .repo_corpus_file_fingerprints_key(corpus, repo_id);
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.del(key).await;
    }

    pub(crate) async fn delete_repo_corpus_snapshot(&self) {
        #[cfg(test)]
        {
            self.shadow
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .repo_corpus_snapshot = None;
        }
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let key = self.keyspace.repo_corpus_snapshot_key();
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.del(key).await;
    }
}
