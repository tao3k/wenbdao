use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use git2::{IndexAddOption, Repository, Signature, Time};

use crate::analyzers::analyze_registered_repository_with_registry;
use crate::analyzers::cache::{
    build_repository_analysis_cache_key, store_cached_repository_analysis,
};
use crate::analyzers::{
    ExampleRecord, ImportKind, ImportRecord, ModuleRecord, RepoSymbolKind,
    RepositoryAnalysisOutput, SymbolRecord, bootstrap_builtin_registry,
};
use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::gateway::studio::router::StudioApiError;
use crate::gateway::studio::router::configured_repository;
use crate::gateway::studio::router::handlers::repo::analysis::search::cache::with_cached_repo_search_result;
use crate::gateway::studio::router::handlers::repo::analysis::search::service::run_repo_import_search;
use crate::gateway::studio::router::{GatewayState, StudioState};
use crate::gateway::studio::test_support::assert_studio_json_snapshot;
use crate::gateway::studio::types::{UiConfig, UiRepoProjectConfig};
use crate::git::checkout::{
    CheckoutSyncMode, discover_checkout_metadata, resolve_repository_source,
};
use crate::query_core::{
    query_repo_entity_example_results_if_published, query_repo_entity_import_results_if_published,
    query_repo_entity_module_results_if_published, query_repo_entity_symbol_results_if_published,
};
use crate::search_plane::{
    SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneService, publish_repo_entities,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct CachedRepoSearchProbe {
    value: String,
}

#[tokio::test]
async fn cached_repo_search_result_reuses_hot_query_payload() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let keyspace = SearchManifestKeyspace::new("xiuxian:test:repo_gateway_cache");
    let search_plane = SearchPlaneService::with_test_cache(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        keyspace,
        SearchMaintenancePolicy::default(),
    );
    let load_count = Arc::new(AtomicUsize::new(0));

    let first = with_cached_repo_search_result(
        &search_plane,
        "repo.symbol-search",
        "alpha/repo",
        "solve",
        5,
        {
            let load_count = Arc::clone(&load_count);
            || async move {
                load_count.fetch_add(1, Ordering::SeqCst);
                Ok(CachedRepoSearchProbe {
                    value: "first".to_string(),
                })
            }
        },
    )
    .await
    .unwrap_or_else(|error| panic!("first cached search result: {error:?}"));

    let second = with_cached_repo_search_result(
        &search_plane,
        "repo.symbol-search",
        "alpha/repo",
        "solve",
        5,
        {
            let load_count = Arc::clone(&load_count);
            || async move {
                load_count.fetch_add(1, Ordering::SeqCst);
                Err(StudioApiError::internal(
                    "UNEXPECTED_RELOAD",
                    "cached repo search should not execute loader twice",
                    None,
                ))
            }
        },
    )
    .await
    .unwrap_or_else(|error| panic!("cached repo search hit should succeed: {error:?}"));

    assert_eq!(first, second);
    assert_eq!(first.value, "first");
    assert_eq!(load_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn repo_entity_query_core_returns_none_when_publication_is_not_ready() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = SearchPlaneService::with_test_cache(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo_entity_module_not_ready"),
        SearchMaintenancePolicy::default(),
    );

    let result = query_repo_entity_module_results_if_published(
        &service,
        "alpha/repo",
        "BaseModelica",
        5,
        false,
    )
    .await
    .unwrap_or_else(|error| panic!("query helper should return none: {error:?}"));

    assert!(result.is_none());
}

#[tokio::test]
async fn repo_entity_query_core_module_payload_snapshot() {
    let (_temp_dir, service) =
        sample_repo_entity_service("xiuxian:test:repo_entity_module_payload").await;

    let result = query_repo_entity_module_results_if_published(
        &service,
        "alpha/repo",
        "BaseModelica",
        5,
        true,
    )
    .await
    .unwrap_or_else(|error| panic!("query helper should return module payload: {error:?}"));

    assert_studio_json_snapshot("repo_analysis_module_search_plane_payload", result);
}

#[tokio::test]
async fn repo_entity_query_core_symbol_payload_snapshot() {
    let (_temp_dir, service) =
        sample_repo_entity_service("xiuxian:test:repo_entity_symbol_payload").await;

    let result =
        query_repo_entity_symbol_results_if_published(&service, "alpha/repo", "solve", 5, true)
            .await
            .unwrap_or_else(|error| panic!("query helper should return symbol payload: {error:?}"));

    assert_studio_json_snapshot("repo_analysis_symbol_search_plane_payload", result);
}

#[tokio::test]
async fn repo_entity_query_core_example_payload_snapshot() {
    let (_temp_dir, service) =
        sample_repo_entity_service("xiuxian:test:repo_entity_example_payload").await;

    let result =
        query_repo_entity_example_results_if_published(&service, "alpha/repo", "solve", 5, true)
            .await
            .unwrap_or_else(|error| {
                panic!("query helper should return example payload: {error:?}")
            });

    assert_studio_json_snapshot("repo_analysis_example_search_plane_payload", result);
}

#[tokio::test]
async fn repo_entity_query_core_import_payload_snapshot() {
    let (_temp_dir, service) =
        sample_repo_entity_service("xiuxian:test:repo_entity_import_payload").await;

    let result = query_repo_entity_import_results_if_published(
        &service,
        "alpha/repo",
        Some("SciMLBase".to_string()),
        Some("BaseModelica".to_string()),
        5,
        true,
    )
    .await
    .unwrap_or_else(|error| panic!("query helper should return import payload: {error:?}"));

    assert_studio_json_snapshot("repo_analysis_import_query_core_payload", result);
}

#[tokio::test]
async fn repo_import_search_uses_repo_entity_fast_path_when_publication_ready() {
    let fixture = sample_repo_entity_gateway_fixture("xiuxian:test:repo_import_fast_path").await;

    let result = run_repo_import_search(
        Arc::clone(&fixture.state),
        "alpha/repo".to_string(),
        Some("SciMLBase".to_string()),
        Some("BaseModelica".to_string()),
        5,
    )
    .await
    .unwrap_or_else(|error| {
        panic!("repo import search should resolve through repo entity fast path: {error:?}")
    });

    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.imports[0].target_package, "SciMLBase");
    assert_eq!(result.imports[0].source_module, "BaseModelica");
}

#[test]
fn repo_entity_query_core_error_mapping_preserves_gateway_contract() {
    let error = StudioApiError::internal(
        "REPO_MODULE_SEARCH_FAILED",
        "Repo module search task failed",
        Some("broken repo entity payload".to_string()),
    );

    assert_eq!(error.code(), "REPO_MODULE_SEARCH_FAILED");
    assert_eq!(
        error.status(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn repo_import_search_payload_snapshot() {
    let fixture = sample_import_gateway_fixture("xiuxian:test:repo_import_search_payload");

    let result = run_repo_import_search(
        Arc::clone(&fixture.state),
        "sciml/imports".to_string(),
        Some("SciMLBase".to_string()),
        None,
        10,
    )
    .await
    .unwrap_or_else(|error| panic!("repo import search should resolve: {error:?}"));

    assert_studio_json_snapshot("repo_analysis_import_search_payload", result);
}

async fn sample_repo_entity_service(keyspace: &str) -> (tempfile::TempDir, SearchPlaneService) {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new(keyspace),
        SearchMaintenancePolicy::default(),
    );
    let analysis = sample_analysis("alpha/repo", "solve", "Shows solve");
    let documents = sample_documents("solve", 10);
    publish_repo_entities(&service, "alpha/repo", &analysis, &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    (temp_dir, service)
}

fn sample_analysis(
    repo_id: &str,
    symbol_name: &str,
    example_summary: &str,
) -> RepositoryAnalysisOutput {
    let mut attributes = std::collections::BTreeMap::new();
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

struct ImportGatewayFixture {
    _temp_dir: tempfile::TempDir,
    state: Arc<GatewayState>,
}

async fn sample_repo_entity_gateway_fixture(keyspace: &str) -> ImportGatewayFixture {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let registry = Arc::new(
        bootstrap_builtin_registry()
            .unwrap_or_else(|error| panic!("bootstrap builtin registry: {error:?}")),
    );
    let studio = StudioState::new_with_bootstrap_ui_config_and_search_plane_root(
        registry,
        temp_dir.path().join("search_plane").join(keyspace),
    );
    let analysis = sample_analysis("alpha/repo", "solve", "Shows solve");
    let documents = sample_documents("solve", 10);
    let state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(studio),
    });
    publish_repo_entities(
        &state.studio.search_plane,
        "alpha/repo",
        &analysis,
        &documents,
        Some("rev-1"),
    )
    .await
    .unwrap_or_else(|error| panic!("publish repo entities: {error}"));

    ImportGatewayFixture {
        _temp_dir: temp_dir,
        state,
    }
}

fn sample_import_gateway_fixture(keyspace: &str) -> ImportGatewayFixture {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let repo_root = temp_dir.path().join("projectionpkg");
    std::fs::create_dir_all(repo_root.join("src"))
        .unwrap_or_else(|error| panic!("create src dir: {error}"));
    std::fs::create_dir_all(repo_root.join("examples"))
        .unwrap_or_else(|error| panic!("create examples dir: {error}"));
    std::fs::write(
        repo_root.join("Project.toml"),
        r#"name = "ProjectionPkg"
uuid = "12345678-1234-1234-1234-123456789abc"
version = "0.1.0"

[deps]
Reexport = "189a3867-3050-52da-a836-e630ba90ab69"
SciMLBase = "0bca4576-84f4-4d90-8ffe-ffa030f20462"
"#,
    )
    .unwrap_or_else(|error| panic!("write project: {error}"));
    std::fs::write(
        repo_root.join("src").join("ProjectionPkg.jl"),
        r#"module ProjectionPkg

using Reexport
@reexport using SciMLBase

export solve

solve(problem) = problem

end
"#,
    )
    .unwrap_or_else(|error| panic!("write source: {error}"));
    std::fs::write(
        repo_root.join("examples").join("basic.jl"),
        "using ProjectionPkg\nsolve(1)\n",
    )
    .unwrap_or_else(|error| panic!("write example: {error}"));
    initialize_git_repository(&repo_root);

    let registry = Arc::new(
        bootstrap_builtin_registry()
            .unwrap_or_else(|error| panic!("bootstrap builtin registry: {error:?}")),
    );
    let studio = StudioState::new_with_bootstrap_ui_config_and_search_plane_root(
        Arc::clone(&registry),
        temp_dir.path().join("search_plane").join(keyspace),
    );
    studio.set_ui_config(UiConfig {
        projects: Vec::new(),
        repo_projects: vec![UiRepoProjectConfig {
            id: "sciml/imports".to_string(),
            root: Some(repo_root.to_string_lossy().to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["julia".to_string()],
        }],
    });
    prime_import_analysis_cache(&studio, registry);

    ImportGatewayFixture {
        _temp_dir: temp_dir,
        state: Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio),
        }),
    }
}

fn prime_import_analysis_cache(
    studio: &StudioState,
    registry: Arc<crate::analyzers::PluginRegistry>,
) {
    let repository = configured_repository(studio, "sciml/imports")
        .unwrap_or_else(|error| panic!("resolve configured repository: {error:?}"));
    let analysis = analyze_registered_repository_with_registry(
        &repository,
        studio.project_root.as_path(),
        &registry,
    )
    .unwrap_or_else(|error| panic!("analyze import fixture repository: {error:?}"));
    let repository_source = resolve_repository_source(
        &repository,
        studio.project_root.as_path(),
        CheckoutSyncMode::Status,
    )
    .unwrap_or_else(|error| panic!("resolve repository source: {error:?}"));
    let checkout_metadata = discover_checkout_metadata(repository_source.checkout_root.as_path());
    let cache_key = build_repository_analysis_cache_key(
        &repository,
        &repository_source,
        checkout_metadata.as_ref(),
    );
    store_cached_repository_analysis(cache_key, &analysis)
        .unwrap_or_else(|error| panic!("store repository analysis cache: {error:?}"));
}

fn initialize_git_repository(repo_root: &std::path::Path) {
    let repository =
        Repository::init(repo_root).unwrap_or_else(|error| panic!("init git repository: {error}"));
    let mut index = repository
        .index()
        .unwrap_or_else(|error| panic!("open git index: {error}"));
    index
        .add_all(["*"], IndexAddOption::DEFAULT, None)
        .unwrap_or_else(|error| panic!("stage git contents: {error}"));
    index
        .write()
        .unwrap_or_else(|error| panic!("write git index: {error}"));
    let tree_id = index
        .write_tree()
        .unwrap_or_else(|error| panic!("write git tree: {error}"));
    let tree = repository
        .find_tree(tree_id)
        .unwrap_or_else(|error| panic!("find git tree: {error}"));
    let signature = Signature::new(
        "Xiuxian Test",
        "test@example.com",
        &Time::new(1_700_000_000, 0),
    )
    .unwrap_or_else(|error| panic!("create git signature: {error}"));
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "initial import",
            &tree,
            &[],
        )
        .unwrap_or_else(|error| panic!("commit git fixture: {error}"));
}
