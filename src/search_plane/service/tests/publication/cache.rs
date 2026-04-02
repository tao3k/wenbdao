use crate::search_plane::service::tests::support::*;

#[tokio::test]
async fn repo_search_query_cache_key_uses_synchronized_runtime_state() {
    let temp_dir = temp_dir();
    let keyspace = service_test_manifest_keyspace();
    let service = SearchPlaneService::with_runtime(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        keyspace.clone(),
        SearchMaintenancePolicy::default(),
        SearchPlaneCache::for_tests(keyspace),
    );

    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 1,
        active: 0,
        queued: 0,
        checking: 0,
        syncing: 0,
        indexing: 0,
        ready: 1,
        unsupported: 0,
        failed: 0,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: None,
        active_repo_ids: Vec::new(),
        repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Ready)],
    });

    let ready_key = some_or_panic(
        service
            .repo_search_query_cache_key(RepoSearchQueryCacheKeyInput {
                scope: "code_search",
                corpora: &[],
                repo_corpora: &[SearchCorpusKind::RepoEntity],
                repo_ids: &[String::from("alpha/repo")],
                query: "alpha",
                limit: 10,
                intent: Some("code_search"),
                repo_hint: Some("alpha/repo"),
            })
            .await,
        "cache key should exist",
    );

    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 1,
        active: 1,
        queued: 0,
        checking: 0,
        syncing: 0,
        indexing: 1,
        ready: 0,
        unsupported: 0,
        failed: 0,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: Some("alpha/repo".to_string()),
        active_repo_ids: vec!["alpha/repo".to_string()],
        repos: vec![RepoIndexEntryStatus {
            last_revision: Some("rev-2".to_string()),
            ..repo_status_entry("alpha/repo", RepoIndexPhase::Indexing)
        }],
    });

    let refreshing_key = some_or_panic(
        service
            .repo_search_query_cache_key(RepoSearchQueryCacheKeyInput {
                scope: "code_search",
                corpora: &[],
                repo_corpora: &[SearchCorpusKind::RepoEntity],
                repo_ids: &[String::from("alpha/repo")],
                query: "alpha",
                limit: 10,
                intent: Some("code_search"),
                repo_hint: Some("alpha/repo"),
            })
            .await,
        "cache key should exist",
    );

    assert_ne!(ready_key, refreshing_key);
}
