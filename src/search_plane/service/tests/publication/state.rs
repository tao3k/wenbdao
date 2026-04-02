use crate::search_plane::service::tests::support::*;

#[tokio::test]
async fn repo_search_publication_state_prefers_publications_over_runtime_phase() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "searchable/repo", &documents, Some("rev-1")).await;
    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 3,
        active: 1,
        queued: 1,
        checking: 0,
        syncing: 0,
        indexing: 1,
        ready: 0,
        unsupported: 0,
        failed: 1,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: Some("searchable/repo".to_string()),
        active_repo_ids: vec!["searchable/repo".to_string()],
        repos: vec![
            RepoIndexEntryStatus {
                last_revision: Some("rev-2".to_string()),
                ..repo_status_entry("searchable/repo", RepoIndexPhase::Indexing)
            },
            repo_status_entry("pending/repo", RepoIndexPhase::Queued),
            repo_status_entry("failed/repo", RepoIndexPhase::Failed),
        ],
    });

    let searchable = service
        .repo_search_publication_state("searchable/repo")
        .await;
    let pending = service.repo_search_publication_state("pending/repo").await;
    let skipped = service.repo_search_publication_state("failed/repo").await;

    assert_eq!(searchable.availability, RepoSearchAvailability::Searchable);
    assert!(searchable.entity_published);
    assert!(searchable.content_published);
    assert_eq!(pending.availability, RepoSearchAvailability::Pending);
    assert!(!pending.entity_published);
    assert!(!pending.content_published);
    assert_eq!(skipped.availability, RepoSearchAvailability::Skipped);
    assert!(!skipped.entity_published);
    assert!(!skipped.content_published);
}

#[tokio::test]
async fn repo_search_publication_states_batches_repo_snapshot_reads() {
    let temp_dir = temp_dir();
    let keyspace = unique_test_manifest_keyspace("batch-publication-state");
    let service = SearchPlaneService::with_runtime(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        keyspace.clone(),
        SearchMaintenancePolicy::default(),
        SearchPlaneCache::for_tests(keyspace),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "searchable/repo", &documents, Some("rev-1")).await;
    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 3,
        active: 0,
        queued: 1,
        checking: 0,
        syncing: 0,
        indexing: 0,
        ready: 1,
        unsupported: 0,
        failed: 1,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: None,
        active_repo_ids: Vec::new(),
        repos: vec![
            repo_status_entry("searchable/repo", RepoIndexPhase::Ready),
            repo_status_entry("pending/repo", RepoIndexPhase::Queued),
            repo_status_entry("failed/repo", RepoIndexPhase::Failed),
        ],
    });
    service.clear_all_in_memory_repo_runtime_for_test();

    let repo_ids = vec![
        "searchable/repo".to_string(),
        "pending/repo".to_string(),
        "failed/repo".to_string(),
    ];
    let states = ok_or_panic(
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let states = service
                    .repo_search_publication_states(repo_ids.as_slice())
                    .await;
                if states.get("failed/repo").map(|state| state.availability)
                    == Some(RepoSearchAvailability::Skipped)
                {
                    break states;
                }
                tokio::task::yield_now().await;
            }
        })
        .await,
        "repo publication states should hydrate",
    );

    assert_eq!(
        states
            .get("searchable/repo")
            .map(|state| state.availability),
        Some(RepoSearchAvailability::Searchable)
    );
    assert_eq!(
        states
            .get("searchable/repo")
            .map(|state| state.entity_published),
        Some(true)
    );
    assert_eq!(
        states.get("pending/repo").map(|state| state.availability),
        Some(RepoSearchAvailability::Pending)
    );
    assert_eq!(
        states.get("failed/repo").map(|state| state.availability),
        Some(RepoSearchAvailability::Skipped)
    );
}

#[tokio::test]
async fn repo_search_publication_state_hydrates_from_repo_corpus_snapshot_after_memory_miss() {
    let temp_dir = temp_dir();
    let keyspace = unique_test_manifest_keyspace("runtime-hydrate");
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
        ready: 0,
        unsupported: 0,
        failed: 1,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: None,
        active_repo_ids: Vec::new(),
        repos: vec![repo_status_entry("failed/repo", RepoIndexPhase::Failed)],
    });
    service.clear_in_memory_repo_runtime_for_test("failed/repo");

    ok_or_panic(
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let skipped = service.repo_search_publication_state("failed/repo").await;
                if skipped.availability == RepoSearchAvailability::Skipped {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await,
        "repo-corpus snapshot should hydrate",
    );

    assert_eq!(
        repo_phase(&service, "failed/repo"),
        Some(RepoIndexPhase::Failed)
    );
}

#[tokio::test]
async fn repo_search_publication_state_hydrates_from_repo_corpus_record_after_memory_miss() {
    let temp_dir = temp_dir();
    let keyspace = service_test_manifest_keyspace();
    let service = SearchPlaneService::with_runtime(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        keyspace.clone(),
        SearchMaintenancePolicy::default(),
        SearchPlaneCache::for_tests(keyspace),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "searchable/repo", &documents, Some("rev-1")).await;
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
        repos: vec![repo_status_entry("searchable/repo", RepoIndexPhase::Ready)],
    });
    service.clear_in_memory_repo_runtime_for_test("searchable/repo");
    service.clear_in_memory_repo_publications_for_test("searchable/repo");
    service.clear_all_in_memory_repo_corpus_records_for_test();

    ok_or_panic(
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let searchable = service
                    .repo_search_publication_state("searchable/repo")
                    .await;
                if searchable.availability == RepoSearchAvailability::Searchable
                    && searchable.entity_published
                    && searchable.content_published
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await,
        "repo-corpus record cache should hydrate",
    );
}

#[tokio::test]
async fn repo_search_publication_state_recovers_publication_after_runtime_only_restart_hydration() {
    let temp_dir = temp_dir();
    let keyspace = unique_test_manifest_keyspace("runtime-only-restart-hydration");
    let service = SearchPlaneService::with_runtime(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        keyspace.clone(),
        SearchMaintenancePolicy::default(),
        SearchPlaneCache::for_tests(keyspace),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "searchable/repo", &documents, Some("rev-1")).await;
    service.clear_all_in_memory_repo_corpus_records_for_test();

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
        repos: vec![repo_status_entry("searchable/repo", RepoIndexPhase::Ready)],
    });

    ok_or_panic(
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let searchable = service
                    .repo_search_publication_state("searchable/repo")
                    .await;
                if searchable.availability == RepoSearchAvailability::Searchable
                    && searchable.entity_published
                    && searchable.content_published
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await,
        "runtime-only restart hydration should recover persisted publication",
    );
}

#[tokio::test]
async fn repo_search_publication_state_does_not_hydrate_from_manifest_without_repo_corpus_cache() {
    let temp_dir = temp_dir();
    let keyspace = unique_test_manifest_keyspace("manifest-not-runtime");
    let service = SearchPlaneService::with_runtime(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        keyspace.clone(),
        SearchMaintenancePolicy::default(),
        SearchPlaneCache::for_tests(keyspace),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "searchable/repo", &documents, Some("rev-1")).await;
    let ready_status = RepoIndexStatusResponse {
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
        repos: vec![repo_status_entry("searchable/repo", RepoIndexPhase::Ready)],
    };
    service.synchronize_repo_runtime(&ready_status);
    service
        .clear_persisted_repo_corpus_for_test("searchable/repo")
        .await;
    service.clear_all_in_memory_repo_corpus_records_for_test();
    service.synchronize_repo_runtime(&ready_status);

    ok_or_panic(
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let state = service
                    .repo_search_publication_state("searchable/repo")
                    .await;
                if state.availability == RepoSearchAvailability::Pending {
                    assert!(!state.entity_published);
                    assert!(!state.content_published);
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await,
        "manifest-only fallback should stay disabled",
    );

    assert_eq!(
        repo_phase(&service, "searchable/repo"),
        Some(RepoIndexPhase::Ready)
    );
}
