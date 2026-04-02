use std::path::PathBuf;

use crate::gateway::studio::types::{ReferenceSearchHit, StudioNavigationTarget};
use crate::search_plane::reference_occurrence::schema::{
    reference_occurrence_batches, reference_occurrence_schema,
};
use crate::search_plane::{
    BeginBuildDecision, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
    SearchPlaneService,
};
use xiuxian_vector::ColumnarScanOptions;

use super::search_reference_occurrences;

fn fixture_service(temp_dir: &tempfile::TempDir) -> SearchPlaneService {
    SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:reference_occurrence"),
        SearchMaintenancePolicy::default(),
    )
}

fn sample_hit(name: &str, path: &str, line: usize) -> ReferenceSearchHit {
    ReferenceSearchHit {
        name: name.to_string(),
        path: path.to_string(),
        language: "rust".to_string(),
        crate_name: "kernel".to_string(),
        project_name: None,
        root_label: None,
        line,
        column: 5,
        line_text: format!("let _value = {name};"),
        navigation_target: StudioNavigationTarget {
            path: path.to_string(),
            category: "doc".to_string(),
            project_name: None,
            root_label: None,
            line: Some(line),
            line_end: Some(line),
            column: Some(5),
        },
        score: 0.0,
    }
}

#[tokio::test]
async fn reference_occurrence_query_reads_hits_from_published_epoch() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::ReferenceOccurrence,
        "fp-1",
        SearchCorpusKind::ReferenceOccurrence.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin decision: {other:?}"),
    };
    let hits = vec![
        sample_hit("AlphaService", "src/lib.rs", 10),
        sample_hit("BetaThing", "src/beta.rs", 20),
    ];
    let store = service
        .open_store(SearchCorpusKind::ReferenceOccurrence)
        .await
        .unwrap_or_else(|error| panic!("open store: {error}"));
    let table_name =
        SearchPlaneService::table_name(SearchCorpusKind::ReferenceOccurrence, lease.epoch);
    store
        .replace_record_batches(
            table_name.as_str(),
            reference_occurrence_schema(),
            reference_occurrence_batches(&hits).unwrap_or_else(|error| panic!("batches: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("replace record batches: {error}"));
    store
        .write_vector_store_table_to_parquet_file(
            table_name.as_str(),
            service
                .local_epoch_parquet_path(SearchCorpusKind::ReferenceOccurrence, lease.epoch)
                .as_path(),
            ColumnarScanOptions::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("export parquet: {error}"));
    service
        .coordinator()
        .publish_ready(&lease, hits.len() as u64, 1);

    let results = search_reference_occurrences(&service, "AlphaService", 5)
        .await
        .unwrap_or_else(|error| panic!("query should succeed: {error}"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "AlphaService");
    assert!(results[0].score > 0.0);

    let snapshot = service.status();
    let corpus = snapshot
        .corpora
        .iter()
        .find(|entry| entry.corpus == SearchCorpusKind::ReferenceOccurrence)
        .unwrap_or_else(|| panic!("reference occurrence corpus row should exist"));
    let telemetry = corpus
        .last_query_telemetry
        .as_ref()
        .unwrap_or_else(|| panic!("reference occurrence telemetry should be present"));
    assert_eq!(
        telemetry.source,
        crate::search_plane::SearchQueryTelemetrySource::Scan
    );
    assert_eq!(telemetry.scope.as_deref(), Some("search"));
    assert!(telemetry.rows_scanned >= 1);
    assert!(telemetry.matched_rows >= 1);
}
