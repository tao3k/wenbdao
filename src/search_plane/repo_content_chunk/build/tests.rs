use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::search_plane::repo_content_chunk::build::orchestration::publish_repo_content_chunks;
use crate::search_plane::repo_content_chunk::build::plan::{
    plan_repo_content_chunk_build, versioned_repo_content_table_name,
};
use crate::search_plane::repo_content_chunk::build::types::{
    REPO_CONTENT_CHUNK_EXTRACTOR_VERSION, RepoContentChunkBuildAction,
};
use crate::search_plane::{
    SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneService,
    SearchPublicationStorageFormat, SearchRepoPublicationInput, SearchRepoPublicationRecord,
};

fn repo_document(
    path: &str,
    contents: &str,
    size_bytes: u64,
    modified_unix_ms: u64,
) -> RepoCodeDocument {
    RepoCodeDocument {
        path: path.to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from(contents),
        size_bytes,
        modified_unix_ms,
    }
}

#[test]
fn plan_repo_content_chunk_build_only_rewrites_changed_files() {
    let first_documents = vec![
        repo_document("src/lib.rs", "fn alpha() {}\n", 14, 10),
        repo_document("src/util.rs", "fn beta() {}\n", 13, 10),
    ];
    let first_plan = plan_repo_content_chunk_build(
        "alpha/repo",
        &first_documents,
        Some("rev-1"),
        None,
        BTreeMap::new(),
    );
    let previous_publication = match first_plan.action {
        RepoContentChunkBuildAction::ReplaceAll { ref table_name, .. } => {
            SearchRepoPublicationRecord::new(
                SearchCorpusKind::RepoContentChunk,
                "alpha/repo",
                SearchRepoPublicationInput {
                    table_name: table_name.clone(),
                    schema_version: SearchCorpusKind::RepoContentChunk.schema_version(),
                    source_revision: Some("rev-1".to_string()),
                    table_version_id: 1,
                    row_count: 2,
                    fragment_count: 1,
                    published_at: "2026-03-24T12:00:00Z".to_string(),
                },
            )
        }
        other => panic!("unexpected first build action: {other:?}"),
    };

    let second_documents = vec![
        repo_document("src/lib.rs", "fn gamma() {}\n", 14, 20),
        repo_document("src/util.rs", "fn beta() {}\n", 13, 10),
    ];
    let second_plan = plan_repo_content_chunk_build(
        "alpha/repo",
        &second_documents,
        Some("rev-2"),
        Some(&previous_publication),
        first_plan.file_fingerprints.clone(),
    );

    match second_plan.action {
        RepoContentChunkBuildAction::CloneAndMutate {
            base_table_name,
            target_table_name,
            replaced_paths,
            changed_payload: changed_documents,
        } => {
            assert_eq!(base_table_name, previous_publication.table_name);
            assert_ne!(target_table_name, previous_publication.table_name);
            assert_eq!(
                replaced_paths.into_iter().collect::<Vec<_>>(),
                vec!["src/lib.rs".to_string()]
            );
            assert_eq!(changed_documents.len(), 1);
            assert_eq!(changed_documents[0].path, "src/lib.rs");
        }
        other => panic!("unexpected second build action: {other:?}"),
    }
}

#[test]
fn plan_repo_content_chunk_build_reuses_table_for_revision_only_refresh() {
    let documents = vec![repo_document("src/lib.rs", "fn alpha() {}\n", 14, 10)];
    let table_name = versioned_repo_content_table_name(
        "alpha/repo",
        &documents
            .iter()
            .map(|document| {
                (
                    document.path.clone(),
                    document.to_file_fingerprint(
                        REPO_CONTENT_CHUNK_EXTRACTOR_VERSION,
                        SearchCorpusKind::RepoContentChunk.schema_version(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>(),
        Some("rev-1"),
    );
    let publication = SearchRepoPublicationRecord::new(
        SearchCorpusKind::RepoContentChunk,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: table_name.clone(),
            schema_version: SearchCorpusKind::RepoContentChunk.schema_version(),
            source_revision: Some("rev-1".to_string()),
            table_version_id: 1,
            row_count: 1,
            fragment_count: 1,
            published_at: "2026-03-24T12:00:00Z".to_string(),
        },
    );
    let plan = plan_repo_content_chunk_build(
        "alpha/repo",
        &documents,
        Some("rev-2"),
        Some(&publication),
        documents
            .iter()
            .map(|document| {
                (
                    document.path.clone(),
                    document.to_file_fingerprint(
                        REPO_CONTENT_CHUNK_EXTRACTOR_VERSION,
                        SearchCorpusKind::RepoContentChunk.schema_version(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>(),
    );

    match plan.action {
        RepoContentChunkBuildAction::RefreshPublication { table_name } => {
            assert_eq!(table_name, publication.table_name);
        }
        other => panic!("unexpected build action: {other:?}"),
    }
}

#[tokio::test]
async fn repo_content_chunk_incremental_refresh_reuses_unchanged_rows() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-content-build"),
        SearchMaintenancePolicy::default(),
    );
    let first_documents = vec![
        repo_document("src/lib.rs", "fn alpha() {}\n", 14, 10),
        repo_document("src/util.rs", "fn beta() {}\n", 13, 10),
    ];
    publish_repo_content_chunks(&service, "alpha/repo", &first_documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("first publish: {error}"));

    let first_record = service
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, "alpha/repo")
        .await
        .unwrap_or_else(|| panic!("first repo content record"));
    let first_table_name = first_record
        .publication
        .as_ref()
        .unwrap_or_else(|| panic!("first publication"))
        .table_name
        .clone();
    assert!(
        !service
            .corpus_root(SearchCorpusKind::RepoContentChunk)
            .join(format!("{first_table_name}.lance"))
            .exists(),
        "repo content publication should no longer create a Lance table"
    );
    assert!(
        first_record
            .maintenance
            .as_ref()
            .and_then(|maintenance| maintenance.last_prewarmed_at.as_ref())
            .is_some()
    );

    let second_documents = vec![
        repo_document("src/lib.rs", "fn gamma() {}\n", 14, 20),
        repo_document("src/util.rs", "fn beta() {}\n", 13, 10),
    ];
    publish_repo_content_chunks(&service, "alpha/repo", &second_documents, Some("rev-2"))
        .await
        .unwrap_or_else(|error| panic!("second publish: {error}"));

    let second_record = service
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, "alpha/repo")
        .await
        .unwrap_or_else(|| panic!("second repo content record"));
    let second_publication = second_record
        .publication
        .as_ref()
        .unwrap_or_else(|| panic!("second publication"));
    assert_ne!(second_publication.table_name, first_table_name);
    assert!(
        !service
            .corpus_root(SearchCorpusKind::RepoContentChunk)
            .join(format!("{}.lance", second_publication.table_name))
            .exists(),
        "repo content incremental publication should stay parquet-only"
    );
    assert_eq!(second_publication.source_revision.as_deref(), Some("rev-2"));
    assert_eq!(
        second_publication.storage_format,
        SearchPublicationStorageFormat::Parquet
    );
    assert!(
        second_record
            .maintenance
            .as_ref()
            .and_then(|maintenance| maintenance.last_prewarmed_at.as_ref())
            .is_some()
    );
    let parquet_path = service.repo_publication_parquet_path(
        SearchCorpusKind::RepoContentChunk,
        second_publication.table_name.as_str(),
    );
    assert!(parquet_path.exists(), "missing repo content parquet export");

    let beta_hits = service
        .search_repo_content_chunks("alpha/repo", "beta", &Default::default(), 5)
        .await
        .unwrap_or_else(|error| panic!("query beta: {error}"));
    assert_eq!(beta_hits.len(), 1);
    assert_eq!(beta_hits[0].path, "src/util.rs");

    let gamma_hits = service
        .search_repo_content_chunks("alpha/repo", "gamma", &Default::default(), 5)
        .await
        .unwrap_or_else(|error| panic!("query gamma: {error}"));
    assert_eq!(gamma_hits.len(), 1);
    assert_eq!(gamma_hits[0].path, "src/lib.rs");

    let alpha_hits = service
        .search_repo_content_chunks("alpha/repo", "alpha", &Default::default(), 5)
        .await
        .unwrap_or_else(|error| panic!("query alpha: {error}"));
    assert!(alpha_hits.is_empty());

    let fingerprints = service
        .repo_corpus_file_fingerprints(SearchCorpusKind::RepoContentChunk, "alpha/repo")
        .await;
    assert_eq!(fingerprints.len(), 2);
    assert_eq!(
        fingerprints
            .get("src/lib.rs")
            .map(|fingerprint| fingerprint.modified_unix_ms),
        Some(20)
    );
}
