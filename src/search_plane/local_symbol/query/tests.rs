use std::path::PathBuf;

use crate::gateway::studio::types::{AstSearchHit, StudioNavigationTarget};
use crate::search_plane::local_symbol::query::autocomplete::autocomplete_local_symbols;
use crate::search_plane::local_symbol::query::search::search_local_symbols;
use crate::search_plane::local_symbol::query::shared::{
    decode_local_symbol_hits, execute_local_symbol_search, retained_window,
};
use crate::search_plane::{
    BeginBuildDecision, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
    SearchPlaneService,
};

use crate::search_plane::local_symbol::schema::{local_symbol_batches, local_symbol_schema};
use xiuxian_vector::ColumnarScanOptions;

fn fixture_service(temp_dir: &tempfile::TempDir) -> SearchPlaneService {
    SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:local_symbol"),
        SearchMaintenancePolicy::default(),
    )
}

fn sample_hit(name: &str, path: &str, line_start: usize) -> AstSearchHit {
    AstSearchHit {
        name: name.to_string(),
        signature: format!("fn {name}()"),
        path: path.to_string(),
        language: "rust".to_string(),
        crate_name: "kernel".to_string(),
        project_name: None,
        root_label: None,
        node_kind: None,
        owner_title: None,
        navigation_target: StudioNavigationTarget {
            path: path.to_string(),
            category: "symbol".to_string(),
            project_name: None,
            root_label: None,
            line: Some(line_start),
            line_end: Some(line_start),
            column: Some(1),
        },
        line_start,
        line_end: line_start,
        score: 0.0,
    }
}

fn sample_markdown_hit(
    name: &str,
    node_kind: Option<&str>,
    owner_title: Option<&str>,
) -> AstSearchHit {
    AstSearchHit {
        name: name.to_string(),
        signature: format!("## {name}"),
        path: "docs/alpha.md".to_string(),
        language: "markdown".to_string(),
        crate_name: "docs".to_string(),
        project_name: None,
        root_label: None,
        node_kind: node_kind.map(ToOwned::to_owned),
        owner_title: owner_title.map(ToOwned::to_owned),
        navigation_target: StudioNavigationTarget {
            path: "docs/alpha.md".to_string(),
            category: "symbol".to_string(),
            project_name: None,
            root_label: None,
            line: Some(1),
            line_end: Some(1),
            column: Some(1),
        },
        line_start: 1,
        line_end: 1,
        score: 0.0,
    }
}

#[tokio::test]
async fn local_symbol_query_reads_hits_from_published_epoch() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        "fp-1",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin decision: {other:?}"),
    };
    let hits = vec![
        sample_hit("AlphaSymbol", "src/lib.rs", 10),
        sample_hit("BetaThing", "src/beta.rs", 20),
    ];
    let store = service
        .open_store(SearchCorpusKind::LocalSymbol)
        .await
        .unwrap_or_else(|error| panic!("open store: {error}"));
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::LocalSymbol, lease.epoch);
    store
        .replace_record_batches(
            table_name.as_str(),
            local_symbol_schema(),
            local_symbol_batches(&hits).unwrap_or_else(|error| panic!("batches: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("replace record batches: {error}"));
    store
        .write_vector_store_table_to_parquet_file(
            table_name.as_str(),
            service
                .local_epoch_parquet_path(SearchCorpusKind::LocalSymbol, lease.epoch)
                .as_path(),
            ColumnarScanOptions::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("export parquet: {error}"));
    service
        .coordinator()
        .publish_ready(&lease, hits.len() as u64, 1);

    let results = search_local_symbols(&service, "alpha", 5)
        .await
        .unwrap_or_else(|error| panic!("query should succeed: {error}"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "AlphaSymbol");
    assert!(results[0].score > 0.0);

    let snapshot = service.status();
    let corpus = snapshot
        .corpora
        .iter()
        .find(|entry| entry.corpus == SearchCorpusKind::LocalSymbol)
        .unwrap_or_else(|| panic!("local symbol corpus row should exist"));
    let telemetry = corpus
        .last_query_telemetry
        .as_ref()
        .unwrap_or_else(|| panic!("local symbol telemetry should be present"));
    assert_eq!(
        telemetry.source,
        crate::search_plane::SearchQueryTelemetrySource::Scan
    );
    assert_eq!(telemetry.scope.as_deref(), Some("search"));
    assert!(telemetry.rows_scanned >= 1);
    assert!(telemetry.matched_rows >= 1);
}

#[tokio::test]
async fn local_symbol_query_can_rerank_across_multiple_tables() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let store = service
        .open_store(SearchCorpusKind::LocalSymbol)
        .await
        .unwrap_or_else(|error| panic!("open store: {error}"));
    let hits_a = vec![sample_hit("AlphaSymbol", "src/lib.rs", 10)];
    let hits_b = vec![sample_hit("BetaAlphaHelper", "src/beta.rs", 20)];

    store
        .replace_record_batches(
            "local_symbol_project_a",
            local_symbol_schema(),
            local_symbol_batches(&hits_a).unwrap_or_else(|error| panic!("batches a: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("replace record batches a: {error}"));
    store
        .replace_record_batches(
            "local_symbol_project_b",
            local_symbol_schema(),
            local_symbol_batches(&hits_b).unwrap_or_else(|error| panic!("batches b: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("replace record batches b: {error}"));
    store
        .write_vector_store_table_to_parquet_file(
            "local_symbol_project_a",
            service
                .local_table_parquet_path(SearchCorpusKind::LocalSymbol, "local_symbol_project_a")
                .as_path(),
            ColumnarScanOptions::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("export parquet a: {error}"));
    store
        .write_vector_store_table_to_parquet_file(
            "local_symbol_project_b",
            service
                .local_table_parquet_path(SearchCorpusKind::LocalSymbol, "local_symbol_project_b")
                .as_path(),
            ColumnarScanOptions::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("export parquet b: {error}"));
    service
        .search_engine()
        .ensure_parquet_table_registered(
            "local_symbol_project_a",
            service
                .local_table_parquet_path(SearchCorpusKind::LocalSymbol, "local_symbol_project_a")
                .as_path(),
            &[],
        )
        .await
        .unwrap_or_else(|error| panic!("register parquet a: {error}"));
    service
        .search_engine()
        .ensure_parquet_table_registered(
            "local_symbol_project_b",
            service
                .local_table_parquet_path(SearchCorpusKind::LocalSymbol, "local_symbol_project_b")
                .as_path(),
            &[],
        )
        .await
        .unwrap_or_else(|error| panic!("register parquet b: {error}"));

    let execution = execute_local_symbol_search(
        service.search_engine(),
        &[
            "local_symbol_project_a".to_string(),
            "local_symbol_project_b".to_string(),
        ],
        "alpha",
        retained_window(5),
    )
    .await
    .unwrap_or_else(|error| panic!("multi-table query should succeed: {error}"));

    let hits = decode_local_symbol_hits(service.search_engine(), execution.candidates)
        .await
        .unwrap_or_else(|error| panic!("decode hits should succeed: {error}"));
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].name, "AlphaSymbol");
    assert_eq!(hits[1].name, "BetaAlphaHelper");
}

#[tokio::test]
async fn local_symbol_autocomplete_reads_suggestions_from_published_epoch() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        "fp-2",
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin decision: {other:?}"),
    };
    let hits = vec![
        sample_hit("AlphaSymbol", "src/lib.rs", 10),
        sample_markdown_hit("Search Design", Some("section"), None),
        sample_markdown_hit("Search Metadata", Some("property"), Some("Owner")),
    ];
    let store = service
        .open_store(SearchCorpusKind::LocalSymbol)
        .await
        .unwrap_or_else(|error| panic!("open store: {error}"));
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::LocalSymbol, lease.epoch);
    store
        .replace_record_batches(
            table_name.as_str(),
            local_symbol_schema(),
            local_symbol_batches(&hits).unwrap_or_else(|error| panic!("batches: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("replace record batches: {error}"));
    store
        .write_vector_store_table_to_parquet_file(
            table_name.as_str(),
            service
                .local_epoch_parquet_path(SearchCorpusKind::LocalSymbol, lease.epoch)
                .as_path(),
            ColumnarScanOptions::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("export parquet: {error}"));
    service
        .coordinator()
        .publish_ready(&lease, hits.len() as u64, 1);

    let results = autocomplete_local_symbols(&service, "se", 5)
        .await
        .unwrap_or_else(|error| panic!("autocomplete should succeed: {error}"));

    assert_eq!(
        results
            .into_iter()
            .map(|item| (item.text, item.suggestion_type))
            .collect::<Vec<_>>(),
        vec![
            ("Search Design".to_string(), "heading".to_string()),
            ("Search Metadata".to_string(), "metadata".to_string()),
        ]
    );

    let snapshot = service.status();
    let corpus = snapshot
        .corpora
        .iter()
        .find(|entry| entry.corpus == SearchCorpusKind::LocalSymbol)
        .unwrap_or_else(|| panic!("local symbol corpus row should exist"));
    let telemetry = corpus
        .last_query_telemetry
        .as_ref()
        .unwrap_or_else(|| panic!("autocomplete telemetry should be present"));
    assert_eq!(
        telemetry.source,
        crate::search_plane::SearchQueryTelemetrySource::Scan
    );
    assert_eq!(telemetry.scope.as_deref(), Some("autocomplete"));
    assert!(telemetry.rows_scanned >= 1);
    assert!(telemetry.matched_rows >= 2);
}
