use crate::search_plane::SearchCorpusKind;
use crate::search_plane::coordinator::SearchCompactionReason;
use crate::search_plane::service::core::RepoMaintenanceTask;
use crate::search_plane::service::core::maintenance::tests::{
    make_compaction_task, make_prewarm_task, make_service,
};

#[test]
fn register_repo_maintenance_task_prioritizes_prewarm_before_compaction() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = make_service(&temp_dir, "xiuxian:test:repo-maintenance-priority");
    let compaction = make_compaction_task(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        "publication-1",
        "repo_entity_alpha",
        12,
        SearchCompactionReason::PublishThreshold,
    );
    let prewarm = make_prewarm_task(
        SearchCorpusKind::RepoEntity,
        "beta/repo",
        "repo_entity_beta",
        &["path"],
    );

    let (_, compaction_enqueued, start_compaction_worker) =
        service.register_repo_maintenance_task(compaction.clone(), false);
    let (_, prewarm_enqueued, start_prewarm_worker) =
        service.register_repo_maintenance_task(prewarm.clone(), false);

    assert!(compaction_enqueued);
    assert!(prewarm_enqueued);
    assert!(start_compaction_worker);
    assert!(!start_prewarm_worker);

    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(runtime.queue.len(), 2);
    assert!(matches!(
        runtime.queue.front().map(|queued| &queued.task),
        Some(RepoMaintenanceTask::Prewarm(_))
    ));
    assert!(matches!(
        runtime.queue.back().map(|queued| &queued.task),
        Some(RepoMaintenanceTask::Compaction(_))
    ));
}

#[test]
fn register_repo_maintenance_task_replaces_queued_stale_compaction_for_same_repo() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = make_service(&temp_dir, "xiuxian:test:repo-maintenance-stale-compaction");
    let first = make_compaction_task(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        "publication-1",
        "repo_entity_alpha_v1",
        12,
        SearchCompactionReason::PublishThreshold,
    );
    let second = make_compaction_task(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        "publication-2",
        "repo_entity_alpha_v2",
        8,
        SearchCompactionReason::PublishThreshold,
    );

    let (_, first_enqueued, first_start_worker) =
        service.register_repo_maintenance_task(first.clone(), false);
    let (_, second_enqueued, second_start_worker) =
        service.register_repo_maintenance_task(second.clone(), false);

    assert!(first_enqueued);
    assert!(second_enqueued);
    assert!(first_start_worker);
    assert!(!second_start_worker);

    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(runtime.queue.len(), 1);
    assert!(runtime.in_flight.contains(&second.task_key()));
    assert!(!runtime.in_flight.contains(&first.task_key()));
    assert!(matches!(
        runtime.queue.front().map(|queued| &queued.task),
        Some(RepoMaintenanceTask::Compaction(task))
            if task.publication_id == "publication-2"
    ));
}

#[test]
fn register_repo_maintenance_task_prioritizes_publish_threshold_before_row_delta_ratio() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = make_service(&temp_dir, "xiuxian:test:repo-maintenance-priority-reason");
    let row_delta = make_compaction_task(
        SearchCorpusKind::RepoContentChunk,
        "beta/repo",
        "publication-beta",
        "repo_content_chunk_beta",
        8,
        SearchCompactionReason::RowDeltaRatio,
    );
    let publish_threshold = make_compaction_task(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        "publication-alpha",
        "repo_entity_alpha",
        64,
        SearchCompactionReason::PublishThreshold,
    );

    let _ = service.register_repo_maintenance_task(row_delta, false);
    let _ = service.register_repo_maintenance_task(publish_threshold, false);

    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(runtime.queue.len(), 2);
    assert!(matches!(
        runtime.queue.front().map(|queued| &queued.task),
        Some(RepoMaintenanceTask::Compaction(task))
            if task.repo_id == "alpha/repo"
    ));
}

#[test]
fn register_repo_maintenance_task_prioritizes_smaller_row_count_within_same_reason() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = make_service(
        &temp_dir,
        "xiuxian:test:repo-maintenance-priority-row-count",
    );
    let large = make_compaction_task(
        SearchCorpusKind::RepoContentChunk,
        "beta/repo",
        "publication-beta",
        "repo_content_chunk_beta",
        64,
        SearchCompactionReason::RowDeltaRatio,
    );
    let small = make_compaction_task(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        "publication-alpha",
        "repo_entity_alpha",
        8,
        SearchCompactionReason::RowDeltaRatio,
    );

    let _ = service.register_repo_maintenance_task(large, false);
    let _ = service.register_repo_maintenance_task(small, false);

    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(runtime.queue.len(), 2);
    assert!(matches!(
        runtime.queue.front().map(|queued| &queued.task),
        Some(RepoMaintenanceTask::Compaction(task))
            if task.repo_id == "alpha/repo"
    ));
}

#[test]
fn register_repo_maintenance_task_ages_row_delta_ratio_ahead_of_new_publish_thresholds() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = make_service(&temp_dir, "xiuxian:test:repo-maintenance-aging");
    let aged_row_delta = make_compaction_task(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        "publication-alpha",
        "repo_entity_alpha",
        16,
        SearchCompactionReason::RowDeltaRatio,
    );
    let publish_one = make_compaction_task(
        SearchCorpusKind::RepoContentChunk,
        "beta/repo",
        "publication-beta",
        "repo_content_chunk_beta",
        64,
        SearchCompactionReason::PublishThreshold,
    );
    let publish_two = make_compaction_task(
        SearchCorpusKind::RepoEntity,
        "gamma/repo",
        "publication-gamma",
        "repo_entity_gamma",
        64,
        SearchCompactionReason::PublishThreshold,
    );
    let publish_three = make_compaction_task(
        SearchCorpusKind::RepoContentChunk,
        "delta/repo",
        "publication-delta",
        "repo_content_chunk_delta",
        64,
        SearchCompactionReason::PublishThreshold,
    );

    let _ = service.register_repo_maintenance_task(aged_row_delta, false);
    let _ = service.register_repo_maintenance_task(publish_one, false);
    let _ = service.register_repo_maintenance_task(publish_two, false);
    let _ = service.register_repo_maintenance_task(publish_three, false);

    let runtime = service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(runtime.queue.len(), 4);
    assert!(matches!(
        runtime.queue.get(2).map(|queued| &queued.task),
        Some(RepoMaintenanceTask::Compaction(task))
            if task.repo_id == "alpha/repo"
    ));
    assert!(matches!(
        runtime.queue.get(3).map(|queued| &queued.task),
        Some(RepoMaintenanceTask::Compaction(task))
            if task.repo_id == "delta/repo"
    ));
}
