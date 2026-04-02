use std::path::PathBuf;

use crate::gateway::studio::repo_index::RepoIndexPhase;
use crate::search_plane::service::core::types::{RepoMaintenanceTaskKind, SearchPlaneService};
use crate::search_plane::{
    SearchCorpusKind, SearchCorpusStatusAction, SearchCorpusStatusReasonCode,
    SearchCorpusStatusSeverity, SearchMaintenancePolicy, SearchMaintenanceStatus,
    SearchManifestKeyspace, SearchRepoCorpusRecord, SearchRepoPublicationInput,
    SearchRepoPublicationRecord, SearchRepoRuntimeRecord,
};

#[test]
fn annotate_runtime_status_preserves_repo_compaction_running_from_record_maintenance() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:search-plane:repo-compaction"),
        SearchMaintenancePolicy::default(),
    );
    let publication = SearchRepoPublicationRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: "repo_entity_repo_alpha".to_string(),
            schema_version: SearchCorpusKind::RepoEntity.schema_version(),
            source_revision: Some("rev-1".to_string()),
            table_version_id: 7,
            row_count: 12,
            fragment_count: 4,
            published_at: "2026-03-24T12:34:56Z".to_string(),
        },
    );
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord {
            repo_id: "alpha/repo".to_string(),
            phase: RepoIndexPhase::Ready,
            last_revision: Some("rev-1".to_string()),
            last_error: None,
            updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        }),
        Some(publication),
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        compaction_running: true,
        compaction_pending: true,
        publish_count_since_compaction: 1,
        ..SearchMaintenanceStatus::default()
    }));

    let mut status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);
    service.annotate_runtime_status(&mut status);

    assert!(status.maintenance.compaction_running);
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::Compacting);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(reason.readable);
}

#[test]
fn annotate_runtime_status_marks_repo_compaction_running_from_active_task() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:search-plane:repo-compaction-active"),
        SearchMaintenancePolicy::default(),
    );
    let publication = SearchRepoPublicationRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: "repo_entity_repo_alpha".to_string(),
            schema_version: SearchCorpusKind::RepoEntity.schema_version(),
            source_revision: Some("rev-1".to_string()),
            table_version_id: 7,
            row_count: 12,
            fragment_count: 4,
            published_at: "2026-03-24T12:34:56Z".to_string(),
        },
    );
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord {
            repo_id: "alpha/repo".to_string(),
            phase: RepoIndexPhase::Ready,
            last_revision: Some("rev-1".to_string()),
            last_error: None,
            updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        }),
        Some(publication),
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        compaction_running: false,
        compaction_pending: true,
        publish_count_since_compaction: 1,
        ..SearchMaintenanceStatus::default()
    }));
    service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .active_task = Some((
        SearchCorpusKind::RepoEntity,
        "alpha/repo".to_string(),
        "publication-alpha".to_string(),
        RepoMaintenanceTaskKind::Compaction,
    ));

    let mut status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);
    service.annotate_runtime_status(&mut status);

    assert!(status.maintenance.compaction_running);
    assert_eq!(status.maintenance.compaction_queue_depth, 0);
    assert_eq!(status.maintenance.compaction_queue_position, None);
    assert!(!status.maintenance.compaction_queue_aged);
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::Compacting);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(reason.readable);
}

#[test]
fn annotate_runtime_status_surfaces_repo_compaction_queue_backlog() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:search-plane:repo-compaction-queue"),
        SearchMaintenancePolicy::default(),
    );
    let publication = SearchRepoPublicationRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: "repo_entity_repo_alpha".to_string(),
            schema_version: SearchCorpusKind::RepoEntity.schema_version(),
            source_revision: Some("rev-1".to_string()),
            table_version_id: 7,
            row_count: 12,
            fragment_count: 4,
            published_at: "2026-03-24T12:34:56Z".to_string(),
        },
    );
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord {
            repo_id: "alpha/repo".to_string(),
            phase: RepoIndexPhase::Ready,
            last_revision: Some("rev-1".to_string()),
            last_error: None,
            updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        }),
        Some(publication),
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        compaction_pending: true,
        publish_count_since_compaction: 1,
        ..SearchMaintenanceStatus::default()
    }));
    {
        let mut runtime = service
            .repo_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime.queue.push_back(
            crate::search_plane::service::core::types::QueuedRepoMaintenanceTask {
                task: crate::search_plane::service::core::types::RepoMaintenanceTask::Prewarm(
                    crate::search_plane::service::core::types::RepoPrewarmTask {
                        corpus: SearchCorpusKind::RepoEntity,
                        repo_id: "beta/repo".to_string(),
                        table_name: "repo_entity_repo_beta".to_string(),
                        projected_columns: vec!["name".to_string()],
                    },
                ),
                enqueue_sequence: 0,
            },
        );
        runtime.queue.push_back(
            crate::search_plane::service::core::types::QueuedRepoMaintenanceTask {
                task: crate::search_plane::service::core::types::RepoMaintenanceTask::Compaction(
                    crate::search_plane::service::core::types::RepoCompactionTask {
                        corpus: SearchCorpusKind::RepoEntity,
                        repo_id: "alpha/repo".to_string(),
                        publication_id: "publication-alpha".to_string(),
                        table_name: "repo_entity_repo_alpha".to_string(),
                        row_count: 12,
                        reason:
                            crate::search_plane::coordinator::SearchCompactionReason::PublishThreshold,
                    },
                ),
                enqueue_sequence: 1,
            },
        );
        runtime.queue.push_back(
            crate::search_plane::service::core::types::QueuedRepoMaintenanceTask {
                task: crate::search_plane::service::core::types::RepoMaintenanceTask::Compaction(
                    crate::search_plane::service::core::types::RepoCompactionTask {
                        corpus: SearchCorpusKind::RepoContentChunk,
                        repo_id: "gamma/repo".to_string(),
                        publication_id: "publication-gamma".to_string(),
                        table_name: "repo_content_chunk_repo_gamma".to_string(),
                        row_count: 12,
                        reason:
                            crate::search_plane::coordinator::SearchCompactionReason::PublishThreshold,
                    },
                ),
                enqueue_sequence: 0,
            },
        );
    }

    let mut status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);
    service.annotate_runtime_status(&mut status);

    assert!(!status.maintenance.compaction_running);
    assert_eq!(status.maintenance.compaction_queue_depth, 1);
    assert_eq!(status.maintenance.compaction_queue_position, Some(2));
    assert!(!status.maintenance.compaction_queue_aged);
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::CompactionPending);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(reason.readable);
}

#[test]
fn annotate_runtime_status_surfaces_repo_compaction_queue_aging() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:search-plane:repo-compaction-aged"),
        SearchMaintenancePolicy::default(),
    );
    let publication = SearchRepoPublicationRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: "repo_entity_repo_alpha".to_string(),
            schema_version: SearchCorpusKind::RepoEntity.schema_version(),
            source_revision: Some("rev-1".to_string()),
            table_version_id: 7,
            row_count: 12,
            fragment_count: 4,
            published_at: "2026-03-24T12:34:56Z".to_string(),
        },
    );
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord {
            repo_id: "alpha/repo".to_string(),
            phase: RepoIndexPhase::Ready,
            last_revision: Some("rev-1".to_string()),
            last_error: None,
            updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        }),
        Some(publication),
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        compaction_pending: true,
        publish_count_since_compaction: 1,
        ..SearchMaintenanceStatus::default()
    }));
    {
        let mut runtime = service
            .repo_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime.next_enqueue_sequence = 4;
        runtime.queue.push_back(
            crate::search_plane::service::core::types::QueuedRepoMaintenanceTask {
                task: crate::search_plane::service::core::types::RepoMaintenanceTask::Compaction(
                    crate::search_plane::service::core::types::RepoCompactionTask {
                        corpus: SearchCorpusKind::RepoEntity,
                        repo_id: "alpha/repo".to_string(),
                        publication_id: "publication-alpha".to_string(),
                        table_name: "repo_entity_repo_alpha".to_string(),
                        row_count: 12,
                        reason:
                            crate::search_plane::coordinator::SearchCompactionReason::RowDeltaRatio,
                    },
                ),
                enqueue_sequence: 0,
            },
        );
    }

    let mut status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);
    service.annotate_runtime_status(&mut status);

    assert_eq!(status.maintenance.compaction_queue_depth, 1);
    assert_eq!(status.maintenance.compaction_queue_position, Some(1));
    assert!(status.maintenance.compaction_queue_aged);
}
