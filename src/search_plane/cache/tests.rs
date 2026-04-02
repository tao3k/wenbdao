use std::collections::BTreeMap;

use crate::search_plane::cache::SearchPlaneCache;
use crate::search_plane::{
    SearchCorpusKind, SearchFileFingerprint, SearchManifestKeyspace, SearchRepoCorpusRecord,
    SearchRepoCorpusSnapshotRecord,
};

#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct TestCacheShadow {
    pub(crate) repo_corpus_records: BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord>,
    pub(crate) repo_corpus_snapshot: Option<SearchRepoCorpusSnapshotRecord>,
    pub(crate) corpus_file_fingerprints:
        BTreeMap<SearchCorpusKind, BTreeMap<String, SearchFileFingerprint>>,
    pub(crate) repo_corpus_file_fingerprints:
        BTreeMap<(SearchCorpusKind, String), BTreeMap<String, SearchFileFingerprint>>,
}

#[cfg(test)]
impl SearchPlaneCache {
    pub(crate) fn clear_repo_shadow_for_tests(&self, repo_id: &str) {
        let mut shadow = self
            .shadow
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        shadow
            .repo_corpus_records
            .retain(|(_, candidate_repo_id), _| candidate_repo_id != repo_id);
        if let Some(snapshot) = shadow.repo_corpus_snapshot.as_mut() {
            snapshot.records.retain(|record| record.repo_id != repo_id);
            if snapshot.records.is_empty() {
                shadow.repo_corpus_snapshot = None;
            }
        }
        shadow
            .repo_corpus_file_fingerprints
            .retain(|(_, candidate_repo_id), _| candidate_repo_id != repo_id);
    }
}

#[cfg(test)]
fn required_cache_key(key: Option<String>, context: &str) -> String {
    key.unwrap_or_else(|| panic!("{context}"))
}

#[cfg(test)]
fn cache_for_tests() -> SearchPlaneCache {
    SearchPlaneCache::for_tests(SearchManifestKeyspace::new("xiuxian:test:search_plane"))
}

#[cfg(test)]
#[test]
fn autocomplete_key_is_stable_for_epoch_prefix_and_limit() {
    let cache = cache_for_tests();
    let key = required_cache_key(
        cache.autocomplete_cache_key(" Alpha Handler ", 8, 7),
        "autocomplete key",
    );
    assert_eq!(
        key,
        required_cache_key(
            cache.autocomplete_cache_key("alpha    handler", 8, 7),
            "stable autocomplete key",
        )
    );
    assert_ne!(
        key,
        required_cache_key(
            cache.autocomplete_cache_key("alpha handler", 8, 8),
            "epoch-specific autocomplete key",
        )
    );
}

#[cfg(test)]
#[test]
fn search_query_key_tracks_scope_epochs_and_query_shape() {
    let cache = cache_for_tests();
    let key = required_cache_key(
        cache.search_query_cache_key(
            "intent",
            &[
                (SearchCorpusKind::KnowledgeSection, 3),
                (SearchCorpusKind::LocalSymbol, 11),
            ],
            "  alpha_handler  ",
            10,
            Some("semantic_lookup"),
            None,
        ),
        "search query key",
    );
    assert_eq!(
        key,
        required_cache_key(
            cache.search_query_cache_key(
                "intent",
                &[
                    (SearchCorpusKind::KnowledgeSection, 3),
                    (SearchCorpusKind::LocalSymbol, 11),
                ],
                "alpha_handler",
                10,
                Some("semantic_lookup"),
                None,
            ),
            "stable search query key",
        )
    );
    assert_ne!(
        key,
        required_cache_key(
            cache.search_query_cache_key(
                "intent",
                &[
                    (SearchCorpusKind::KnowledgeSection, 3),
                    (SearchCorpusKind::LocalSymbol, 12),
                ],
                "alpha_handler",
                10,
                Some("semantic_lookup"),
                None,
            ),
            "epoch-specific search query key",
        )
    );
}

#[cfg(test)]
#[test]
fn search_query_key_tracks_repo_versions_and_sorts_components() {
    let cache = cache_for_tests();
    let key = required_cache_key(
        cache.search_query_cache_key_from_versions(
            "intent_code",
            &[
                "repo_entity:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                    .to_string(),
                "knowledge_section:schema:1:epoch:3".to_string(),
                "repo_content_chunk:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                    .to_string(),
            ],
            " lang:julia reexport ",
            10,
            Some("debug_lookup"),
            Some("alpha"),
        ),
        "repo search query key",
    );
    assert_eq!(
        key,
        required_cache_key(
            cache.search_query_cache_key_from_versions(
                "intent_code",
                &[
                    "repo_content_chunk:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                        .to_string(),
                    "knowledge_section:schema:1:epoch:3".to_string(),
                    "repo_entity:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                        .to_string(),
                ],
                "lang:julia   reexport",
                10,
                Some("debug_lookup"),
                Some("alpha"),
            ),
            "stable repo search query key",
        )
    );
    assert_ne!(
        key,
        required_cache_key(
            cache.search_query_cache_key_from_versions(
                "intent_code",
                &[
                    "repo_entity:schema:1:repo:alpha:phase:ready:revision:def:updated:2026-03-23t09:00:00z"
                        .to_string(),
                    "knowledge_section:schema:1:epoch:3".to_string(),
                    "repo_content_chunk:schema:1:repo:alpha:phase:ready:revision:def:updated:2026-03-23t09:00:00z"
                        .to_string(),
                ],
                "lang:julia reexport",
                10,
                Some("debug_lookup"),
                Some("alpha"),
            ),
            "repo-specific search query key",
        )
    );
}

#[cfg(test)]
#[test]
fn disabled_cache_skips_key_generation() {
    let cache = SearchPlaneCache::disabled(SearchManifestKeyspace::new("xiuxian:test"));
    assert!(cache.autocomplete_cache_key("alpha", 8, 1).is_none());
    assert!(
        cache
            .search_query_cache_key(
                "knowledge",
                &[(SearchCorpusKind::KnowledgeSection, 1)],
                "alpha",
                10,
                None,
                None,
            )
            .is_none()
    );
}
