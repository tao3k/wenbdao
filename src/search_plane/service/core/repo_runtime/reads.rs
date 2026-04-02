use std::collections::BTreeMap;
use std::fs;

use crate::search_plane::service::core::types::{RepoRuntimeState, SearchPlaneService};
use crate::search_plane::{
    SearchCorpusKind, SearchRepoCorpusRecord, SearchRepoCorpusSnapshotRecord,
};

impl SearchPlaneService {
    fn merge_persisted_repo_corpus_record(
        current: &mut SearchRepoCorpusRecord,
        persisted: SearchRepoCorpusRecord,
    ) -> bool {
        let mut changed = false;
        if current.publication.is_none() && persisted.publication.is_some() {
            current.publication = persisted.publication;
            changed = true;
        }
        if current.maintenance.is_none() && persisted.maintenance.is_some() {
            current.maintenance = persisted.maintenance;
            changed = true;
        }
        changed
    }

    async fn recover_persisted_repo_corpus_record_for_reads(
        &self,
        record: SearchRepoCorpusRecord,
    ) -> (SearchRepoCorpusRecord, bool) {
        let (mut record, mut changed) = self.reconcile_repo_corpus_record_for_reads(record);
        if record.publication.is_some() {
            return (record, changed);
        }

        if let Some(cache_record) = self
            .cache
            .get_repo_corpus_record(record.corpus, record.repo_id.as_str())
            .await
        {
            let (cache_record, cache_changed) =
                self.reconcile_repo_corpus_record_for_reads(cache_record);
            changed |= cache_changed;
            changed |= Self::merge_persisted_repo_corpus_record(&mut record, cache_record);
        }

        if record.publication.is_none()
            && let Some(local_record) =
                self.load_local_repo_corpus_record(record.corpus, record.repo_id.as_str())
        {
            let (local_record, local_changed) =
                self.reconcile_repo_corpus_record_for_reads(local_record);
            changed |= local_changed;
            changed |= Self::merge_persisted_repo_corpus_record(&mut record, local_record);
        }

        (record, changed)
    }

    #[cfg(test)]
    pub(crate) async fn repo_search_publication_state(
        &self,
        repo_id: &str,
    ) -> crate::search_plane::service::core::types::RepoSearchPublicationState {
        let entity_record = self
            .repo_corpus_record_for_reads(SearchCorpusKind::RepoEntity, repo_id)
            .await;
        let content_record = self
            .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, repo_id)
            .await;
        Self::repo_search_publication_state_from_records(
            entity_record.as_ref(),
            content_record.as_ref(),
        )
    }

    pub(crate) async fn repo_search_publication_states(
        &self,
        repo_ids: &[String],
    ) -> BTreeMap<String, crate::search_plane::service::core::types::RepoSearchPublicationState>
    {
        let records = self.repo_corpus_snapshot_for_reads().await;
        repo_ids
            .iter()
            .map(|repo_id| {
                let entity_record = records.get(&(SearchCorpusKind::RepoEntity, repo_id.clone()));
                let content_record =
                    records.get(&(SearchCorpusKind::RepoContentChunk, repo_id.clone()));
                (
                    repo_id.clone(),
                    Self::repo_search_publication_state_from_records(entity_record, content_record),
                )
            })
            .collect()
    }

    pub(crate) fn repo_runtime_state(&self, repo_id: &str) -> Option<RepoRuntimeState> {
        self.current_repo_runtime_states().remove(repo_id)
    }

    pub(crate) fn cached_repo_publication(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<crate::search_plane::SearchRepoPublicationRecord> {
        self.repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(corpus, repo_id.to_string()))
            .and_then(|record| record.publication.clone())
    }

    fn cached_repo_corpus_record(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoCorpusRecord> {
        self.repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(corpus, repo_id.to_string()))
            .cloned()
    }

    fn reconcile_repo_corpus_record(
        &self,
        mut record: SearchRepoCorpusRecord,
    ) -> (SearchRepoCorpusRecord, bool) {
        let mut changed = false;
        if let Some(runtime) = self.repo_runtime_state(record.repo_id.as_str()) {
            let runtime_record = Self::runtime_record_from_state(record.repo_id.as_str(), &runtime);
            if record.runtime.as_ref() != Some(&runtime_record) {
                record.runtime = Some(runtime_record);
                changed = true;
            }
        }
        if let Some(publication) =
            self.cached_repo_publication(record.corpus, record.repo_id.as_str())
            && record.publication.as_ref() != Some(&publication)
        {
            record.publication = Some(publication);
            changed = true;
        }
        (record, changed)
    }

    fn reconcile_repo_corpus_record_for_reads(
        &self,
        record: SearchRepoCorpusRecord,
    ) -> (SearchRepoCorpusRecord, bool) {
        self.reconcile_repo_corpus_record(record)
    }

    pub(crate) fn current_repo_corpus_snapshot_record(&self) -> SearchRepoCorpusSnapshotRecord {
        let records = self
            .repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .cloned()
            .collect();
        SearchRepoCorpusSnapshotRecord { records }
    }

    pub(crate) async fn repo_corpus_snapshot_for_reads(
        &self,
    ) -> BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord> {
        let current = self
            .repo_corpus_records
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if !current.is_empty() {
            let mut changed_records = Vec::new();
            let mut records = BTreeMap::new();
            for (key, record) in current {
                let (record, changed) = self
                    .recover_persisted_repo_corpus_record_for_reads(record)
                    .await;
                if changed {
                    changed_records.push(record.clone());
                }
                records.insert(key, record);
            }
            if !changed_records.is_empty() {
                *self
                    .repo_corpus_records
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = records.clone();
                for record in &changed_records {
                    self.cache.set_repo_corpus_record(record).await;
                }
                self.cache
                    .set_repo_corpus_snapshot(&SearchRepoCorpusSnapshotRecord {
                        records: records.values().cloned().collect(),
                    })
                    .await;
            }
            return records;
        }
        if let Some(snapshot) = self.cache.get_repo_corpus_snapshot().await {
            let mut records = BTreeMap::new();
            for record in snapshot.records {
                let (record, _) = self
                    .recover_persisted_repo_corpus_record_for_reads(record)
                    .await;
                records.insert((record.corpus, record.repo_id.clone()), record);
            }
            *self
                .repo_corpus_records
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = records.clone();
            return records;
        }
        if let Some(snapshot) = self.load_local_repo_corpus_snapshot() {
            let mut records = BTreeMap::new();
            for record in snapshot.records {
                let (record, _) = self
                    .recover_persisted_repo_corpus_record_for_reads(record)
                    .await;
                records.insert((record.corpus, record.repo_id.clone()), record);
            }
            *self
                .repo_corpus_records
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = records.clone();
            return records;
        }
        BTreeMap::new()
    }

    pub(crate) async fn repo_corpus_record_for_reads(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoCorpusRecord> {
        if let Some(record) = self.cached_repo_corpus_record(corpus, repo_id) {
            let (record, changed) = self
                .recover_persisted_repo_corpus_record_for_reads(record)
                .await;
            if changed {
                self.repo_corpus_records
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert((corpus, repo_id.to_string()), record.clone());
                self.cache.set_repo_corpus_record(&record).await;
            }
            return Some(record);
        }
        if let Some(record) = self.cache.get_repo_corpus_record(corpus, repo_id).await {
            let (record, changed) = self
                .recover_persisted_repo_corpus_record_for_reads(record)
                .await;
            self.repo_corpus_records
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert((corpus, repo_id.to_string()), record.clone());
            if changed {
                self.cache.set_repo_corpus_record(&record).await;
            }
            return Some(record);
        }
        if let Some(record) = self.load_local_repo_corpus_record(corpus, repo_id) {
            let (record, changed) = self
                .recover_persisted_repo_corpus_record_for_reads(record)
                .await;
            self.repo_corpus_records
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert((corpus, repo_id.to_string()), record.clone());
            if changed {
                self.persist_local_repo_corpus_record(&record);
            }
            return Some(record);
        }
        if let Some(record) = self
            .repo_corpus_snapshot_for_reads()
            .await
            .get(&(corpus, repo_id.to_string()))
            .cloned()
        {
            let (record, changed) = self
                .recover_persisted_repo_corpus_record_for_reads(record)
                .await;
            self.repo_corpus_records
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert((corpus, repo_id.to_string()), record.clone());
            if changed {
                self.cache.set_repo_corpus_record(&record).await;
            }
            return Some(record);
        }
        None
    }

    pub(crate) fn load_local_repo_corpus_record(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoCorpusRecord> {
        let payload = fs::read(self.repo_corpus_record_json_path(corpus, repo_id)).ok()?;
        serde_json::from_slice(payload.as_slice()).ok()
    }

    pub(crate) fn load_local_repo_corpus_snapshot(&self) -> Option<SearchRepoCorpusSnapshotRecord> {
        let payload = fs::read(self.repo_corpus_snapshot_json_path()).ok()?;
        serde_json::from_slice(payload.as_slice()).ok()
    }
}
