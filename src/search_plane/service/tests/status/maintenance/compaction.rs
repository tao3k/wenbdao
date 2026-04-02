use crate::search_plane::service::tests::support::*;

#[test]
fn status_marks_ready_corpus_with_pending_compaction_reason() {
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
        "fp-compaction-pending",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(service.publish_ready_and_maintain(&lease, 10, 3));

    let snapshot = service.status();
    let status = corpus_status(
        &snapshot,
        SearchCorpusKind::LocalSymbol,
        "local symbol status should exist",
    );
    assert_eq!(status.phase, SearchPlanePhase::Ready);
    assert!(!status.maintenance.compaction_running);
    assert!(status.maintenance.compaction_pending);
    assert_status_reason(
        status,
        SearchCorpusStatusReasonCode::CompactionPending,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        true,
    );
}

#[test]
fn status_marks_ready_corpus_with_running_compaction_reason() {
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
        "fp-compacting",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(service.publish_ready_and_maintain(&lease, 10, 3));
    service
        .local_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .active_compaction = Some(SearchCorpusKind::LocalSymbol);

    let snapshot = service.status();
    let status = corpus_status(
        &snapshot,
        SearchCorpusKind::LocalSymbol,
        "local symbol status should exist",
    );
    assert_eq!(status.phase, SearchPlanePhase::Ready);
    assert!(status.maintenance.compaction_running);
    assert_eq!(status.maintenance.compaction_queue_depth, 0);
    assert_eq!(status.maintenance.compaction_queue_position, None);
    assert!(status.maintenance.compaction_pending);
    assert_status_reason(
        status,
        SearchCorpusStatusReasonCode::Compacting,
        SearchCorpusStatusSeverity::Info,
        SearchCorpusStatusAction::Wait,
        true,
    );
}
