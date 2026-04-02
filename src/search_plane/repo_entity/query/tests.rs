use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::analyzers::{
    ExampleRecord, ImportKind, ImportRecord, ModuleRecord, RepoSymbolKind,
    RepositoryAnalysisOutput, SymbolRecord,
};
use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::gateway::studio::test_support::assert_wendao_json_snapshot;
use crate::search_plane::ranking::trim_ranked_vec;
use crate::search_plane::repo_entity::publish_repo_entities;
use crate::search_plane::repo_entity::query::execution::{compare_candidates, retained_window};
use crate::search_plane::repo_entity::query::search::{
    search_repo_entity_example_results, search_repo_entity_import_results,
    search_repo_entity_module_results, search_repo_entity_symbol_results,
};
use crate::search_plane::{
    SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneService,
    SearchPublicationStorageFormat,
};

use crate::search_plane::repo_entity::query::types::RepoEntityCandidate;

#[test]
fn trim_candidates_keeps_highest_ranked_entries() {
    let mut candidates = vec![
        RepoEntityCandidate {
            id: "example:1".to_string(),
            score: 0.50,
            entity_kind: "example".to_string(),
            name: "zeta".to_string(),
            path: "src/zeta.rs".to_string(),
        },
        RepoEntityCandidate {
            id: "symbol:1".to_string(),
            score: 0.93,
            entity_kind: "symbol".to_string(),
            name: "beta".to_string(),
            path: "src/beta.rs".to_string(),
        },
        RepoEntityCandidate {
            id: "module:1".to_string(),
            score: 0.93,
            entity_kind: "module".to_string(),
            name: "alpha".to_string(),
            path: "src/alpha.rs".to_string(),
        },
    ];

    trim_ranked_vec(&mut candidates, 2, compare_candidates);

    assert_eq!(candidates.len(), 2);
    assert!(
        candidates
            .windows(2)
            .all(|pair| compare_candidates(&pair[0], &pair[1]).is_le())
    );
    assert_eq!(candidates[0].entity_kind, "symbol");
    assert_eq!(candidates[1].entity_kind, "module");
}

#[test]
fn retained_window_scales_with_limit() {
    assert_eq!(retained_window(0).target, 256);
    assert_eq!(retained_window(4).target, 256);
    assert_eq!(retained_window(64).target, 512);
}

#[tokio::test]
async fn typed_repo_entity_search_reconstructs_module_symbol_and_example_results() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-entity-query"),
        SearchMaintenancePolicy::default(),
    );
    let analysis = sample_analysis("alpha/repo", "solve", "Shows solve");
    let documents = sample_documents("solve", 10);
    publish_repo_entities(&service, "alpha/repo", &analysis, &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    let record = service
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoEntity, "alpha/repo")
        .await
        .unwrap_or_else(|| panic!("repo entity record"));
    let publication = record
        .publication
        .unwrap_or_else(|| panic!("repo entity publication"));
    assert_eq!(
        publication.storage_format,
        SearchPublicationStorageFormat::Parquet
    );
    assert!(
        service
            .repo_publication_parquet_path(
                SearchCorpusKind::RepoEntity,
                publication.table_name.as_str(),
            )
            .exists()
    );

    let module_result =
        search_repo_entity_module_results(&service, "alpha/repo", "BaseModelica", 5)
            .await
            .unwrap_or_else(|error| panic!("module result: {error}"));
    assert_eq!(module_result.modules.len(), 1);
    assert_eq!(module_result.modules[0].qualified_name, "BaseModelica");
    assert!(
        module_result.module_hits[0]
            .projection_page_ids
            .as_ref()
            .is_some_and(|ids| ids.contains(
                &"repo:alpha/repo:projection:reference:module:module:BaseModelica".to_string()
            ))
    );

    let symbol_result = search_repo_entity_symbol_results(&service, "alpha/repo", "solve", 5)
        .await
        .unwrap_or_else(|error| panic!("symbol result: {error}"));
    assert_eq!(symbol_result.symbols.len(), 1);
    assert_eq!(
        symbol_result.symbols[0].module_id.as_deref(),
        Some("module:BaseModelica")
    );
    assert_eq!(
        symbol_result.symbol_hits[0].audit_status.as_deref(),
        Some("verified")
    );
    assert_eq!(
        symbol_result.symbol_hits[0]
            .symbol
            .attributes
            .get("arity")
            .map(String::as_str),
        Some("0")
    );

    let example_result = search_repo_entity_example_results(&service, "alpha/repo", "solve", 5)
        .await
        .unwrap_or_else(|error| panic!("example result: {error}"));
    assert_eq!(example_result.examples.len(), 1);
    assert_eq!(
        example_result.examples[0].summary.as_deref(),
        Some("Shows solve")
    );
    let import_result = search_repo_entity_import_results(
        &service,
        &crate::analyzers::ImportSearchQuery {
            repo_id: "alpha/repo".to_string(),
            package: Some("SciMLBase".to_string()),
            module: Some("BaseModelica".to_string()),
            limit: 5,
        },
    )
    .await
    .unwrap_or_else(|error| panic!("import result: {error}"));
    assert_eq!(import_result.imports.len(), 1);
    assert_eq!(import_result.imports[0].target_package, "SciMLBase");
    assert_eq!(import_result.imports[0].source_module, "BaseModelica");
    assert_wendao_json_snapshot(
        "search_plane_repo_entity_typed_results",
        serde_json::json!({
            "module_result": module_result,
            "symbol_result": symbol_result,
            "example_result": example_result,
            "import_result": import_result,
        }),
    );
}

fn sample_analysis(
    repo_id: &str,
    symbol_name: &str,
    example_summary: &str,
) -> RepositoryAnalysisOutput {
    let mut attributes = BTreeMap::new();
    attributes.insert("arity".to_string(), "0".to_string());
    RepositoryAnalysisOutput {
        modules: vec![ModuleRecord {
            repo_id: repo_id.to_string(),
            module_id: "module:BaseModelica".to_string(),
            qualified_name: "BaseModelica".to_string(),
            path: "src/BaseModelica.jl".to_string(),
        }],
        symbols: vec![SymbolRecord {
            repo_id: repo_id.to_string(),
            symbol_id: format!("symbol:{symbol_name}"),
            module_id: Some("module:BaseModelica".to_string()),
            name: symbol_name.to_string(),
            qualified_name: format!("BaseModelica.{symbol_name}"),
            kind: RepoSymbolKind::Function,
            path: "src/BaseModelica.jl".to_string(),
            line_start: Some(7),
            line_end: Some(9),
            signature: Some(format!("{symbol_name}()")),
            audit_status: Some("verified".to_string()),
            verification_state: Some("verified".to_string()),
            attributes,
        }],
        examples: vec![ExampleRecord {
            repo_id: repo_id.to_string(),
            example_id: "example:solve".to_string(),
            title: "Solve example".to_string(),
            path: "examples/solve.jl".to_string(),
            summary: Some(example_summary.to_string()),
        }],
        imports: vec![ImportRecord {
            repo_id: repo_id.to_string(),
            module_id: "module:BaseModelica".to_string(),
            import_name: "solve".to_string(),
            target_package: "SciMLBase".to_string(),
            source_module: "BaseModelica".to_string(),
            kind: ImportKind::Reexport,
            resolved_id: Some(format!("symbol:{symbol_name}")),
        }],
        ..RepositoryAnalysisOutput::default()
    }
}

fn sample_documents(symbol_name: &str, source_modified_unix_ms: u64) -> Vec<RepoCodeDocument> {
    vec![
        RepoCodeDocument {
            path: "src/BaseModelica.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from(format!(
                "module BaseModelica\n{symbol_name}() = nothing\nend\n"
            )),
            size_bytes: 48,
            modified_unix_ms: source_modified_unix_ms,
        },
        RepoCodeDocument {
            path: "examples/solve.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from("using BaseModelica\nsolve()\n"),
            size_bytes: 28,
            modified_unix_ms: 10,
        },
    ]
}
