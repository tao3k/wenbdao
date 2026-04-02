use std::collections::BTreeMap;

use super::types::SearchPlaneService;
use crate::search_plane::{SearchCorpusKind, SearchFileFingerprint};

impl SearchPlaneService {
    pub(crate) async fn corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
    ) -> BTreeMap<String, SearchFileFingerprint> {
        let fingerprints: BTreeMap<String, SearchFileFingerprint> = self
            .cache
            .get_corpus_file_fingerprints(corpus)
            .await
            .unwrap_or_default();
        fingerprints
    }

    pub(crate) async fn set_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        fingerprints: &BTreeMap<String, SearchFileFingerprint>,
    ) {
        self.cache
            .set_corpus_file_fingerprints(corpus, fingerprints)
            .await;
    }

    pub(crate) async fn repo_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> BTreeMap<String, SearchFileFingerprint> {
        self.cache
            .get_repo_corpus_file_fingerprints(corpus, repo_id)
            .await
            .unwrap_or_default()
    }

    pub(crate) async fn set_repo_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
        fingerprints: &BTreeMap<String, SearchFileFingerprint>,
    ) {
        self.cache
            .set_repo_corpus_file_fingerprints(corpus, repo_id, fingerprints)
            .await;
    }
}
