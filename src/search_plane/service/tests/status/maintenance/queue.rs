use crate::search_plane::service::tests::support::*;

#[test]
fn status_surfaces_local_compaction_queue_backlog_for_queued_corpus() {
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
        "fp-local-queue",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(service.publish_ready_and_maintain(&lease, 10, 3));
    {
        let mut runtime = service
            .local_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime.compaction_queue.push_back(
            crate::search_plane::service::core::QueuedLocalCompactionTask {
                task: crate::search_plane::coordinator::SearchCompactionTask {
                    corpus: SearchCorpusKind::KnowledgeSection,
                    active_epoch: 1,
                    row_count: 8,
                    reason:
                        crate::search_plane::coordinator::SearchCompactionReason::PublishThreshold,
                },
                enqueue_sequence: 0,
            },
        );
        runtime.compaction_queue.push_back(
            crate::search_plane::service::core::QueuedLocalCompactionTask {
                task: crate::search_plane::coordinator::SearchCompactionTask {
                    corpus: SearchCorpusKind::LocalSymbol,
                    active_epoch: 1,
                    row_count: 10,
                    reason:
                        crate::search_plane::coordinator::SearchCompactionReason::PublishThreshold,
                },
                enqueue_sequence: 1,
            },
        );
        runtime
            .running_compactions
            .insert(SearchCorpusKind::KnowledgeSection);
        runtime
            .running_compactions
            .insert(SearchCorpusKind::LocalSymbol);
        runtime.worker_running = true;
    }

    let snapshot = service.status();
    let status = corpus_status(
        &snapshot,
        SearchCorpusKind::LocalSymbol,
        "local symbol status should exist",
    );
    assert_eq!(status.phase, SearchPlanePhase::Ready);
    assert!(!status.maintenance.compaction_running);
    assert_eq!(status.maintenance.compaction_queue_depth, 2);
    assert_eq!(status.maintenance.compaction_queue_position, Some(2));
    assert!(!status.maintenance.compaction_queue_aged);
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
fn status_surfaces_local_compaction_queue_aging_for_aged_row_delta_task() {
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
        "fp-local-aged-queue",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        crate::search_plane::coordinator::BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(service.publish_ready_and_maintain(&lease, 10, 3));
    {
        let mut runtime = service
            .local_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime.next_enqueue_sequence = 4;
        runtime.compaction_queue.push_back(
            crate::search_plane::service::core::QueuedLocalCompactionTask {
                task: crate::search_plane::coordinator::SearchCompactionTask {
                    corpus: SearchCorpusKind::LocalSymbol,
                    active_epoch: 1,
                    row_count: 10,
                    reason: crate::search_plane::coordinator::SearchCompactionReason::RowDeltaRatio,
                },
                enqueue_sequence: 0,
            },
        );
        runtime
            .running_compactions
            .insert(SearchCorpusKind::LocalSymbol);
        runtime.worker_running = true;
    }

    let snapshot = service.status();
    let status = corpus_status(
        &snapshot,
        SearchCorpusKind::LocalSymbol,
        "local symbol status should exist",
    );
    assert_eq!(status.phase, SearchPlanePhase::Ready);
    assert_eq!(status.maintenance.compaction_queue_depth, 1);
    assert_eq!(status.maintenance.compaction_queue_position, Some(1));
    assert!(status.maintenance.compaction_queue_aged);
}
