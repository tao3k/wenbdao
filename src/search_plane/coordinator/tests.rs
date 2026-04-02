use std::path::PathBuf;

use crate::search_plane::{
    SearchCorpusKind, SearchCorpusStatus, SearchMaintenancePolicy, SearchManifestKeyspace,
    SearchPlanePhase,
};

use super::{BeginBuildDecision, SearchCompactionReason, SearchPlaneCoordinator};

fn coordinator_with_policy(policy: SearchMaintenancePolicy) -> SearchPlaneCoordinator {
    SearchPlaneCoordinator::new(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/storage"),
        SearchManifestKeyspace::new("xiuxian:test:search_plane"),
        policy,
    )
}

#[test]
fn begin_build_marks_corpus_as_indexing() {
    let coordinator = coordinator_with_policy(SearchMaintenancePolicy::default());
    let decision = coordinator.begin_build(SearchCorpusKind::LocalSymbol, "fingerprint-a", 3);
    let BeginBuildDecision::Started(lease) = decision else {
        panic!("expected started build lease");
    };

    let status = coordinator.status_for(SearchCorpusKind::LocalSymbol);
    assert_eq!(status.phase, SearchPlanePhase::Indexing);
    assert_eq!(status.staging_epoch, Some(lease.epoch));
    assert_eq!(status.schema_version, 3);
    assert_eq!(status.fingerprint.as_deref(), Some("fingerprint-a"));
    assert_eq!(status.progress, Some(0.0));
}

#[test]
fn stale_publish_is_discarded_after_newer_build_starts() {
    let coordinator = coordinator_with_policy(SearchMaintenancePolicy::default());
    let old = match coordinator.begin_build(SearchCorpusKind::LocalSymbol, "fingerprint-a", 1) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };
    let next = match coordinator.begin_build(SearchCorpusKind::LocalSymbol, "fingerprint-b", 1) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };

    assert!(!coordinator.publish_ready(&old, 10, 2));
    assert!(coordinator.publish_ready(&next, 20, 4));

    let status = coordinator.status_for(SearchCorpusKind::LocalSymbol);
    assert_eq!(status.phase, SearchPlanePhase::Ready);
    assert_eq!(status.active_epoch, Some(next.epoch));
    assert_eq!(status.row_count, Some(20));
}

#[test]
fn schema_version_mismatch_forces_rebuild_even_with_same_fingerprint() {
    let coordinator = coordinator_with_policy(SearchMaintenancePolicy::default());
    let old = match coordinator.begin_build(SearchCorpusKind::LocalSymbol, "fingerprint-a", 1) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };
    assert!(coordinator.publish_ready(&old, 10, 2));

    let next = match coordinator.begin_build(SearchCorpusKind::LocalSymbol, "fingerprint-a", 2) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("expected rebuild for schema mismatch, got {other:?}"),
    };

    assert_ne!(next.epoch, old.epoch);
    let status = coordinator.status_for(SearchCorpusKind::LocalSymbol);
    assert_eq!(status.phase, SearchPlanePhase::Indexing);
    assert_eq!(status.schema_version, 2);
    assert_eq!(status.staging_epoch, Some(next.epoch));
}

#[test]
fn maintenance_policy_marks_compaction_pending_after_threshold() {
    let coordinator = coordinator_with_policy(SearchMaintenancePolicy {
        publish_count_threshold: 2,
        row_delta_ratio_threshold: 0.90,
    });
    let first = match coordinator.begin_build(SearchCorpusKind::RepoEntity, "fp-1", 1) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };
    assert!(coordinator.publish_ready(&first, 100, 5));
    assert!(
        !coordinator
            .status_for(SearchCorpusKind::RepoEntity)
            .maintenance
            .compaction_pending
    );

    let second = match coordinator.begin_build(SearchCorpusKind::RepoEntity, "fp-2", 1) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin result: {other:?}"),
    };
    assert!(coordinator.publish_ready(&second, 110, 6));
    assert!(
        coordinator
            .status_for(SearchCorpusKind::RepoEntity)
            .maintenance
            .compaction_pending
    );

    assert!(coordinator.mark_compaction_complete(
        SearchCorpusKind::RepoEntity,
        second.epoch,
        110,
        2,
        SearchCompactionReason::PublishThreshold
    ));
    let status = coordinator.status_for(SearchCorpusKind::RepoEntity);
    assert!(!status.maintenance.compaction_pending);
    assert_eq!(status.maintenance.publish_count_since_compaction, 0);
    assert_eq!(
        status.maintenance.last_compaction_reason.as_deref(),
        Some("publish_threshold")
    );
}

#[test]
fn replace_status_updates_runtime_snapshot() {
    let coordinator = coordinator_with_policy(SearchMaintenancePolicy::default());
    let mut status = SearchCorpusStatus::new(SearchCorpusKind::RepoEntity);
    status.phase = SearchPlanePhase::Ready;
    status.active_epoch = Some(77);
    status.staging_epoch = Some(79);
    status.row_count = Some(42);
    status.fingerprint = Some("repo-fingerprint".to_string());

    coordinator.replace_status(status.clone());

    assert_eq!(coordinator.status_for(SearchCorpusKind::RepoEntity), status);
    let snapshot = coordinator.status();
    let stored = snapshot
        .corpora
        .iter()
        .find(|entry| entry.corpus == SearchCorpusKind::RepoEntity)
        .unwrap_or_else(|| panic!("repo entity status should be present"));
    assert_eq!(stored.active_epoch, Some(77));
    assert_eq!(stored.staging_epoch, Some(79));
    assert_eq!(stored.fingerprint.as_deref(), Some("repo-fingerprint"));
}
