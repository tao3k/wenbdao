use std::collections::HashSet;
use std::fs;

use super::types::SearchPlaneService;
use crate::search_plane::{
    SearchCorpusKind, SearchPublicationStorageFormat, SearchRepoCorpusRecord,
    SearchRepoPublicationInput, SearchRepoPublicationRecord,
};

impl SearchPlaneService {
    pub(crate) async fn record_repo_publication_input_with_storage_format(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
        input: SearchRepoPublicationInput,
        storage_format: SearchPublicationStorageFormat,
    ) {
        let previous_record = self
            .repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(corpus, repo_id.to_string()))
            .cloned();
        let mut maintenance =
            self.next_repo_publication_maintenance(previous_record.as_ref(), input.row_count);
        if matches!(storage_format, SearchPublicationStorageFormat::Parquet) {
            maintenance.compaction_pending = false;
            maintenance.compaction_running = false;
            maintenance.publish_count_since_compaction = 0;
        }
        let record = match storage_format {
            SearchPublicationStorageFormat::Lance => {
                SearchRepoPublicationRecord::new(corpus, repo_id, input)
            }
            SearchPublicationStorageFormat::Parquet => {
                SearchRepoPublicationRecord::new_with_storage_format(
                    corpus,
                    repo_id,
                    input,
                    SearchPublicationStorageFormat::Parquet,
                )
            }
        };
        let runtime = self
            .repo_runtime_state(repo_id)
            .map(|state| Self::runtime_record_from_state(repo_id, &state));
        let corpus_record =
            SearchRepoCorpusRecord::new(corpus, repo_id.to_string(), runtime, Some(record))
                .with_maintenance(Some(maintenance));
        self.repo_corpus_records
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((corpus, repo_id.to_string()), corpus_record.clone());
        self.persist_local_repo_corpus_record(&corpus_record);
        self.cache.set_repo_corpus_record(&corpus_record).await;
        self.persist_local_repo_corpus_snapshot(&self.current_repo_corpus_snapshot_record());
        self.cache
            .set_repo_corpus_snapshot(&self.current_repo_corpus_snapshot_record())
            .await;
        self.schedule_repo_compaction_if_needed(&corpus_record)
            .await;
    }

    pub(crate) async fn publish_repo_content_chunks_with_revision(
        &self,
        repo_id: &str,
        documents: &[crate::gateway::studio::repo_index::RepoCodeDocument],
        source_revision: Option<&str>,
    ) -> Result<(), xiuxian_vector::VectorStoreError> {
        crate::search_plane::repo_content_chunk::publish_repo_content_chunks(
            self,
            repo_id,
            documents,
            source_revision,
        )
        .await
    }

    pub(crate) async fn search_repo_content_chunks(
        &self,
        repo_id: &str,
        search_term: &str,
        language_filters: &HashSet<String>,
        limit: usize,
    ) -> Result<
        Vec<crate::gateway::studio::types::SearchHit>,
        crate::search_plane::repo_content_chunk::RepoContentChunkSearchError,
    > {
        crate::search_plane::repo_content_chunk::search_repo_content_chunks(
            self,
            repo_id,
            search_term,
            language_filters,
            limit,
        )
        .await
    }

    pub(crate) async fn publish_repo_entities_with_revision(
        &self,
        repo_id: &str,
        analysis: &crate::analyzers::RepositoryAnalysisOutput,
        documents: &[crate::gateway::studio::repo_index::RepoCodeDocument],
        source_revision: Option<&str>,
    ) -> Result<(), xiuxian_vector::VectorStoreError> {
        crate::search_plane::repo_entity::publish_repo_entities(
            self,
            repo_id,
            analysis,
            documents,
            source_revision,
        )
        .await
    }

    pub(crate) fn clear_repo_publications(&self, repo_id: &str) {
        for corpus in [
            SearchCorpusKind::RepoEntity,
            SearchCorpusKind::RepoContentChunk,
        ] {
            self.repo_corpus_records
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&(corpus, repo_id.to_string()));
        }
        self.clear_repo_maintenance_for_repo(repo_id);
        #[cfg(test)]
        self.cache.clear_repo_shadow_for_tests(repo_id);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let cache = self.cache.clone();
            let repo_id = repo_id.to_string();
            let corpus_snapshot = self.current_repo_corpus_snapshot_record();
            handle.spawn(async move {
                cache
                    .delete_repo_corpus_record(SearchCorpusKind::RepoEntity, repo_id.as_str())
                    .await;
                cache
                    .delete_repo_corpus_record(SearchCorpusKind::RepoContentChunk, repo_id.as_str())
                    .await;
                cache
                    .delete_repo_corpus_file_fingerprints(
                        SearchCorpusKind::RepoEntity,
                        repo_id.as_str(),
                    )
                    .await;
                cache
                    .delete_repo_corpus_file_fingerprints(
                        SearchCorpusKind::RepoContentChunk,
                        repo_id.as_str(),
                    )
                    .await;
                if corpus_snapshot.records.is_empty() {
                    cache.delete_repo_corpus_snapshot().await;
                } else {
                    cache.set_repo_corpus_snapshot(&corpus_snapshot).await;
                }
            });
        }
    }

    #[cfg(test)]
    pub(crate) async fn has_published_repo_corpus(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> bool {
        self.repo_corpus_record_for_reads(corpus, repo_id)
            .await
            .and_then(|record| record.publication)
            .is_some()
    }

    pub(crate) async fn search_repo_entities(
        &self,
        repo_id: &str,
        search_term: &str,
        language_filters: &HashSet<String>,
        kind_filters: &HashSet<String>,
        limit: usize,
    ) -> Result<
        Vec<crate::gateway::studio::types::SearchHit>,
        crate::search_plane::repo_entity::RepoEntitySearchError,
    > {
        crate::search_plane::repo_entity::search_repo_entities(
            self,
            repo_id,
            search_term,
            language_filters,
            kind_filters,
            limit,
        )
        .await
    }

    pub(crate) fn persist_local_repo_corpus_record(&self, record: &SearchRepoCorpusRecord) {
        let path = self.repo_corpus_record_json_path(record.corpus, record.repo_id.as_str());
        let Some(parent) = path.parent() else {
            return;
        };
        let Ok(payload) = serde_json::to_vec(record) else {
            return;
        };
        let _ = fs::create_dir_all(parent);
        let _ = fs::write(path, payload);
    }

    pub(crate) fn persist_local_repo_corpus_snapshot(
        &self,
        snapshot: &crate::search_plane::SearchRepoCorpusSnapshotRecord,
    ) {
        let path = self.repo_corpus_snapshot_json_path();
        let Some(parent) = path.parent() else {
            return;
        };
        let Ok(payload) = serde_json::to_vec(snapshot) else {
            return;
        };
        let _ = fs::create_dir_all(parent);
        let _ = fs::write(path, payload);
    }
}
