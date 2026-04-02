use crate::search_plane::service::tests::support::*;

#[tokio::test]
async fn stop_local_maintenance_aborts_running_compaction_and_clears_runtime() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let worker_handle = tokio::spawn(async {
        std::future::pending::<()>().await;
    });
    {
        let mut runtime = service
            .local_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime
            .running_compactions
            .insert(SearchCorpusKind::LocalSymbol);
        runtime.active_compaction = Some(SearchCorpusKind::LocalSymbol);
        runtime.worker_running = true;
        runtime.worker_handle = Some(worker_handle);
    }

    service.stop_local_maintenance();

    let runtime = service
        .local_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(runtime.shutdown_requested);
    assert!(runtime.running_compactions.is_empty());
    assert!(runtime.compaction_queue.is_empty());
    assert!(!runtime.worker_running);
    assert!(runtime.worker_handle.is_none());
    assert!(runtime.active_compaction.is_none());
}

#[tokio::test]
async fn prewarm_epoch_table_rejects_after_local_maintenance_shutdown() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );

    service.stop_local_maintenance();

    let error = service
        .prewarm_epoch_table(SearchCorpusKind::LocalSymbol, 1, &["path"])
        .await
        .expect_err("shutdown should reject local prewarm");
    assert!(matches!(
        error,
        xiuxian_vector::VectorStoreError::General(message)
            if message == "search-plane local maintenance runtime was stopped before completing task"
    ));
}

#[tokio::test]
async fn publish_ready_and_maintain_skips_local_compaction_after_shutdown() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy {
            publish_count_threshold: 1,
            row_delta_ratio_threshold: 1.0,
        },
    );
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        "fp-local-maintenance-shutdown",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    service.stop_local_maintenance();
    assert!(service.publish_ready_and_maintain(&lease, 10, 3));

    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol);
    assert!(status.maintenance.compaction_pending);
    assert!(!status.maintenance.compaction_running);
    let runtime = service
        .local_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(runtime.shutdown_requested);
    assert!(runtime.running_compactions.is_empty());
    assert!(runtime.compaction_queue.is_empty());
    assert!(!runtime.worker_running);
    assert!(runtime.worker_handle.is_none());
    assert!(runtime.active_compaction.is_none());
}

#[tokio::test]
async fn service_derives_stable_roots_and_opens_vector_store() {
    let temp_dir = temp_dir();
    let manifest_keyspace = service_test_manifest_keyspace();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        manifest_keyspace.clone(),
        SearchMaintenancePolicy::default(),
    );

    assert_eq!(
        SearchPlaneService::table_name(SearchCorpusKind::LocalSymbol, 7),
        "local_symbol_epoch_7"
    );
    assert_eq!(
        service
            .manifest_keyspace()
            .corpus_manifest_key(SearchCorpusKind::LocalSymbol),
        format!(
            "{}:manifest:local_symbol",
            service.manifest_keyspace().namespace()
        )
    );

    let store = ok_or_panic(
        service.open_store(SearchCorpusKind::LocalSymbol).await,
        "vector store should open",
    );
    assert!(
        store
            .table_path(&SearchPlaneService::table_name(
                SearchCorpusKind::LocalSymbol,
                1
            ))
            .starts_with(service.corpus_root(SearchCorpusKind::LocalSymbol))
    );
}

#[test]
fn service_disables_cache_for_explicit_test_paths() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/project/.data/search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );

    assert!(service.autocomplete_cache_key("alpha", 8).is_none());
    assert!(
        service
            .search_query_cache_key(
                "knowledge",
                &[SearchCorpusKind::KnowledgeSection],
                "alpha",
                10,
                Some("semantic_lookup"),
                None,
            )
            .is_none()
    );
}

#[tokio::test]
async fn compact_pending_corpus_updates_maintenance_status() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy {
            publish_count_threshold: 1,
            row_delta_ratio_threshold: 1.0,
        },
    );

    let hits = vec![sample_hit()];
    ok_or_panic(
        service
            .publish_local_symbol_hits("fp-maintenance", &hits)
            .await,
        "publish local symbol hits",
    );

    ok_or_panic(
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let status = service
                    .coordinator()
                    .status_for(SearchCorpusKind::LocalSymbol);
                if !status.maintenance.compaction_pending
                    && status.maintenance.last_compacted_at.is_some()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await,
        "compaction should complete",
    );

    let status_after = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol);
    assert!(!status_after.maintenance.compaction_pending);
    assert_eq!(status_after.maintenance.publish_count_since_compaction, 0);
    assert!(status_after.maintenance.last_compacted_at.is_some());
    assert_eq!(
        status_after.maintenance.last_compaction_reason.as_deref(),
        Some("publish_threshold")
    );
    assert_eq!(status_after.fragment_count, Some(1));
}

#[tokio::test]
async fn publish_local_symbol_hits_records_staging_prewarm_metadata() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );

    let hits = vec![sample_hit()];
    ok_or_panic(
        service.publish_local_symbol_hits("fp-prewarm", &hits).await,
        "publish local symbol hits",
    );

    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol);
    assert_eq!(status.active_epoch, Some(1));
    assert_eq!(status.maintenance.last_prewarmed_epoch, Some(1));
    assert!(status.maintenance.last_prewarmed_at.is_some());
}
