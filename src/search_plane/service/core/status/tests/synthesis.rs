use crate::gateway::studio::repo_index::{RepoIndexEntryStatus, RepoIndexPhase};
use crate::search_plane::service::core::types::SearchPlaneService;
use crate::search_plane::service::helpers::repo_corpus_staging_epoch;
use crate::search_plane::{
    SearchCorpusKind, SearchCorpusStatusAction, SearchCorpusStatusReasonCode,
    SearchCorpusStatusSeverity, SearchMaintenanceStatus, SearchPlanePhase, SearchRepoCorpusRecord,
    SearchRepoRuntimeRecord,
};

#[test]
fn synthesize_repo_status_marks_indexing_corpus_as_prewarming_when_staging_was_prewarmed() {
    let runtime_status = RepoIndexEntryStatus {
        repo_id: "alpha/repo".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("rev-2".to_string()),
        updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        attempt_count: 1,
    };
    let staging_epoch = repo_corpus_staging_epoch(
        SearchCorpusKind::RepoEntity,
        &[runtime_status.clone()],
        None,
    )
    .unwrap_or_else(|| panic!("staging epoch should exist"));
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord::from_status(&runtime_status)),
        None,
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        last_prewarmed_at: Some("2026-03-24T12:34:57Z".to_string()),
        last_prewarmed_epoch: Some(staging_epoch),
        ..SearchMaintenanceStatus::default()
    }));

    let status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);

    assert_eq!(status.phase, SearchPlanePhase::Indexing);
    assert_eq!(status.staging_epoch, Some(staging_epoch));
    assert_eq!(status.maintenance.last_prewarmed_epoch, Some(staging_epoch));
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::Prewarming);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(!reason.readable);
}

#[test]
fn synthesize_repo_status_marks_indexing_corpus_as_prewarming_when_prewarm_is_running() {
    let runtime_status = RepoIndexEntryStatus {
        repo_id: "alpha/repo".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("rev-2".to_string()),
        updated_at: Some("2026-03-24T12:34:56Z".to_string()),
        attempt_count: 1,
    };
    let record = SearchRepoCorpusRecord::new(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        Some(SearchRepoRuntimeRecord::from_status(&runtime_status)),
        None,
    )
    .with_maintenance(Some(SearchMaintenanceStatus {
        prewarm_running: true,
        ..SearchMaintenanceStatus::default()
    }));

    let status =
        SearchPlaneService::synthesize_repo_table_status(&[record], SearchCorpusKind::RepoEntity);

    assert_eq!(status.phase, SearchPlanePhase::Indexing);
    assert!(status.maintenance.prewarm_running);
    let reason = status
        .status_reason
        .as_ref()
        .unwrap_or_else(|| panic!("status reason should exist"));
    assert_eq!(reason.code, SearchCorpusStatusReasonCode::Prewarming);
    assert_eq!(reason.severity, SearchCorpusStatusSeverity::Info);
    assert_eq!(reason.action, SearchCorpusStatusAction::Wait);
    assert!(!reason.readable);
}
