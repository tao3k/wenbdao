use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::analyzers::RepositoryAnalysisOutput;
use crate::analyzers::query::{RepoSourceKind, RepoSyncResult};
use crate::gateway::studio::repo_index::state::fingerprint::timestamp_now;
use crate::gateway::studio::repo_index::state::tests::new_coordinator;
use crate::gateway::studio::repo_index::types::RepoIndexEntryStatus;
use crate::search_plane::{
    RepoSearchAvailability, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
    SearchPlanePhase, SearchPlaneService, SearchPublicationStorageFormat,
    SearchRepoPublicationInput,
};
use chrono::Utc;

#[tokio::test]
async fn refresh_status_snapshot_synchronizes_search_plane_runtime() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-runtime-sync"),
        SearchMaintenancePolicy::default(),
    );
    let coordinator = new_coordinator(search_plane.clone());
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "pending".to_string(),
        phase: crate::gateway::studio::repo_index::types::RepoIndexPhase::Queued,
        queue_position: None,
        last_error: None,
        last_revision: None,
        updated_at: Some(timestamp_now()),
        attempt_count: 1,
    });
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "skipped".to_string(),
        phase: crate::gateway::studio::repo_index::types::RepoIndexPhase::Failed,
        queue_position: None,
        last_error: Some("boom".to_string()),
        last_revision: None,
        updated_at: Some(timestamp_now()),
        attempt_count: 1,
    });

    let pending = search_plane.repo_search_publication_state("pending").await;
    let skipped = search_plane.repo_search_publication_state("skipped").await;

    assert_eq!(pending.availability, RepoSearchAvailability::Pending);
    assert_eq!(skipped.availability, RepoSearchAvailability::Skipped);
}

#[tokio::test]
async fn stop_releases_background_runner_arc() {
    let coordinator = Arc::new(new_coordinator(SearchPlaneService::new(PathBuf::from("."))));
    let weak = Arc::downgrade(&coordinator);

    coordinator.start();
    tokio::task::yield_now().await;
    coordinator.stop();
    drop(coordinator);

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if weak.upgrade().is_none() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("runner arc should be released after stop: {error}"));
}

#[tokio::test]
async fn refresh_status_snapshot_synchronizes_repo_backed_corpus_statuses() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-status-sync"),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![
        crate::gateway::studio::repo_index::types::RepoCodeDocument {
            path: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("fn alpha() {}\n"),
            size_bytes: 14,
            modified_unix_ms: 0,
        },
    ];
    search_plane
        .publish_repo_entities_with_revision(
            "alpha/repo",
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: "alpha/repo".to_string(),
                    module_id: "module:alpha".to_string(),
                    qualified_name: "Alpha".to_string(),
                    path: "src/lib.rs".to_string(),
                }],
                ..RepositoryAnalysisOutput::default()
            },
            &documents,
            Some("rev-1"),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    search_plane
        .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));
    let coordinator = new_coordinator(search_plane.clone());
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "alpha/repo".to_string(),
        phase: crate::gateway::studio::repo_index::types::RepoIndexPhase::Ready,
        queue_position: None,
        last_error: None,
        last_revision: Some("rev-1".to_string()),
        updated_at: Some(timestamp_now()),
        attempt_count: 1,
    });

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let snapshot = search_plane.status();
            let Some(repo_entity) = snapshot
                .corpora
                .iter()
                .find(|entry| entry.corpus == SearchCorpusKind::RepoEntity)
            else {
                panic!("repo entity row");
            };
            let Some(repo_content) = snapshot
                .corpora
                .iter()
                .find(|entry| entry.corpus == SearchCorpusKind::RepoContentChunk)
            else {
                panic!("repo content row");
            };
            if repo_entity.phase == SearchPlanePhase::Ready
                && repo_content.phase == SearchPlanePhase::Ready
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("repo-backed corpus status should synchronize: {error}"));
}

#[tokio::test]
async fn managed_remote_skips_reindex_when_repo_publications_already_match_revision() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-current-publications"),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![
        crate::gateway::studio::repo_index::types::RepoCodeDocument {
            path: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("fn alpha() {}\n"),
            size_bytes: 14,
            modified_unix_ms: 0,
        },
    ];
    search_plane
        .publish_repo_entities_with_revision(
            "alpha/repo",
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: "alpha/repo".to_string(),
                    module_id: "module:alpha".to_string(),
                    qualified_name: "Alpha".to_string(),
                    path: "src/lib.rs".to_string(),
                }],
                ..RepositoryAnalysisOutput::default()
            },
            &documents,
            Some("rev-1"),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    search_plane
        .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));
    let coordinator = new_coordinator(search_plane);

    assert!(
        coordinator
            .repo_publications_are_current(
                "alpha/repo",
                &RepoSyncResult {
                    repo_id: "alpha/repo".to_string(),
                    source_kind: RepoSourceKind::ManagedRemote,
                    revision: Some("rev-1".to_string()),
                    ..RepoSyncResult::default()
                },
            )
            .await
    );
}

#[tokio::test]
async fn local_checkout_does_not_short_circuit_on_revision_match() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-local-checkout-short-circuit"),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![
        crate::gateway::studio::repo_index::types::RepoCodeDocument {
            path: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("fn alpha() {}\n"),
            size_bytes: 14,
            modified_unix_ms: 0,
        },
    ];
    search_plane
        .publish_repo_entities_with_revision(
            "alpha/repo",
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: "alpha/repo".to_string(),
                    module_id: "module:alpha".to_string(),
                    qualified_name: "Alpha".to_string(),
                    path: "src/lib.rs".to_string(),
                }],
                ..RepositoryAnalysisOutput::default()
            },
            &documents,
            Some("rev-1"),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    search_plane
        .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));
    let coordinator = new_coordinator(search_plane);

    assert!(
        !coordinator
            .repo_publications_are_current(
                "alpha/repo",
                &RepoSyncResult {
                    repo_id: "alpha/repo".to_string(),
                    source_kind: RepoSourceKind::LocalCheckout,
                    revision: Some("rev-1".to_string()),
                    ..RepoSyncResult::default()
                },
            )
            .await
    );
}

#[tokio::test]
async fn managed_remote_requires_both_repo_corpora_to_be_current_parquet_publications() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-missing-parquet-short-circuit"),
        SearchMaintenancePolicy::default(),
    );
    let published_at = Utc::now().to_rfc3339();
    search_plane
        .record_repo_publication_input_with_storage_format(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: "repo_entity_alpha_repo".to_string(),
                schema_version: SearchCorpusKind::RepoEntity.schema_version(),
                source_revision: Some("rev-1".to_string()),
                table_version_id: 1,
                row_count: 1,
                fragment_count: 1,
                published_at: published_at.clone(),
            },
            SearchPublicationStorageFormat::Lance,
        )
        .await;
    search_plane
        .record_repo_publication_input_with_storage_format(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: "repo_content_chunk_alpha_repo".to_string(),
                schema_version: SearchCorpusKind::RepoContentChunk.schema_version(),
                source_revision: Some("rev-1".to_string()),
                table_version_id: 1,
                row_count: 1,
                fragment_count: 1,
                published_at,
            },
            SearchPublicationStorageFormat::Parquet,
        )
        .await;
    let coordinator = new_coordinator(search_plane);

    assert!(
        !coordinator
            .repo_publications_are_current(
                "alpha/repo",
                &RepoSyncResult {
                    repo_id: "alpha/repo".to_string(),
                    source_kind: RepoSourceKind::ManagedRemote,
                    revision: Some("rev-1".to_string()),
                    ..RepoSyncResult::default()
                },
            )
            .await
    );
}
