use crate::search_plane::SearchCorpusKind;
use crate::search_plane::coordinator::SearchCompactionReason;
use crate::search_plane::service::core::types::{LocalMaintenanceRuntime, SearchPlaneService};

#[test]
fn enqueue_local_compaction_task_replaces_queued_stale_task_for_same_corpus() {
    let mut runtime = LocalMaintenanceRuntime::default();
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        crate::search_plane::coordinator::SearchCompactionTask {
            corpus: SearchCorpusKind::LocalSymbol,
            active_epoch: 7,
            row_count: 12,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        crate::search_plane::coordinator::SearchCompactionTask {
            corpus: SearchCorpusKind::LocalSymbol,
            active_epoch: 9,
            row_count: 15,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );

    assert_eq!(runtime.compaction_queue.len(), 1);
    assert!(
        runtime
            .running_compactions
            .contains(&SearchCorpusKind::LocalSymbol)
    );
    let queued = runtime
        .compaction_queue
        .front()
        .unwrap_or_else(|| panic!("queued task should exist"));
    assert_eq!(queued.task.corpus, SearchCorpusKind::LocalSymbol);
    assert_eq!(queued.task.active_epoch, 9);
    assert_eq!(queued.task.row_count, 15);
    assert_eq!(queued.task.reason, SearchCompactionReason::RowDeltaRatio);
}

#[test]
fn enqueue_local_compaction_task_prioritizes_publish_threshold_before_row_delta_ratio() {
    let mut runtime = LocalMaintenanceRuntime::default();
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        crate::search_plane::coordinator::SearchCompactionTask {
            corpus: SearchCorpusKind::KnowledgeSection,
            active_epoch: 4,
            row_count: 8,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        crate::search_plane::coordinator::SearchCompactionTask {
            corpus: SearchCorpusKind::LocalSymbol,
            active_epoch: 9,
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );

    assert_eq!(runtime.compaction_queue.len(), 2);
    assert_eq!(
        runtime.compaction_queue[0].task.corpus,
        SearchCorpusKind::LocalSymbol
    );
    assert_eq!(
        runtime.compaction_queue[1].task.corpus,
        SearchCorpusKind::KnowledgeSection
    );
}

#[test]
fn enqueue_local_compaction_task_prioritizes_smaller_row_count_within_same_reason() {
    let mut runtime = LocalMaintenanceRuntime::default();
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        crate::search_plane::coordinator::SearchCompactionTask {
            corpus: SearchCorpusKind::KnowledgeSection,
            active_epoch: 4,
            row_count: 64,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        crate::search_plane::coordinator::SearchCompactionTask {
            corpus: SearchCorpusKind::LocalSymbol,
            active_epoch: 9,
            row_count: 8,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );

    assert_eq!(runtime.compaction_queue.len(), 2);
    assert_eq!(
        runtime.compaction_queue[0].task.corpus,
        SearchCorpusKind::LocalSymbol
    );
    assert_eq!(
        runtime.compaction_queue[1].task.corpus,
        SearchCorpusKind::KnowledgeSection
    );
}

#[test]
fn enqueue_local_compaction_task_ages_row_delta_ratio_ahead_of_new_publish_thresholds() {
    let mut runtime = LocalMaintenanceRuntime::default();
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        crate::search_plane::coordinator::SearchCompactionTask {
            corpus: SearchCorpusKind::LocalSymbol,
            active_epoch: 1,
            row_count: 16,
            reason: SearchCompactionReason::RowDeltaRatio,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        crate::search_plane::coordinator::SearchCompactionTask {
            corpus: SearchCorpusKind::KnowledgeSection,
            active_epoch: 2,
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        crate::search_plane::coordinator::SearchCompactionTask {
            corpus: SearchCorpusKind::Attachment,
            active_epoch: 3,
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );
    SearchPlaneService::enqueue_local_compaction_task(
        &mut runtime,
        crate::search_plane::coordinator::SearchCompactionTask {
            corpus: SearchCorpusKind::ReferenceOccurrence,
            active_epoch: 4,
            row_count: 64,
            reason: SearchCompactionReason::PublishThreshold,
        },
    );

    assert_eq!(runtime.compaction_queue.len(), 4);
    assert_eq!(
        runtime.compaction_queue[2].task.corpus,
        SearchCorpusKind::LocalSymbol
    );
    assert_eq!(
        runtime.compaction_queue[3].task.corpus,
        SearchCorpusKind::ReferenceOccurrence
    );
}
