use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, Query, State};

use crate::analyzers::bootstrap_builtin_registry;
use crate::gateway::studio::repo_index::RepoIndexPhase;
use crate::gateway::studio::router::tests::repo_project;
use crate::gateway::studio::router::{GatewayState, StudioState};
use crate::gateway::studio::symbol_index::SymbolIndexPhase;
use crate::gateway::studio::types::{UiConfig, UiPluginArtifact, UiProjectConfig, VfsScanResult};
use crate::set_link_graph_wendao_config_override;
use crate::unified_symbol::UnifiedSymbolIndex;
use chrono::DateTime;
use serial_test::serial;
use std::fs;
use xiuxian_wendao_julia::compatibility::link_graph::{
    DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH, DEFAULT_JULIA_RERANK_FLIGHT_ROUTE,
};

#[test]
fn set_ui_config_preserves_cached_state_when_effectively_unchanged() {
    let studio = StudioState::new();
    let config = UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: vec![repo_project("sciml")],
    };
    studio.set_ui_config(config.clone());

    *studio
        .symbol_index
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some(Arc::new(UnifiedSymbolIndex::new()));
    *studio
        .vfs_scan
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(VfsScanResult {
        entries: Vec::new(),
        file_count: 0,
        dir_count: 0,
        scan_duration_ms: 0,
    });

    studio.set_ui_config(config);

    assert!(
        studio
            .vfs_scan
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    );
    let repo_status = studio.repo_index.status_response(None);
    assert_eq!(repo_status.total, 1);
    assert_ne!(repo_status.repos[0].phase, RepoIndexPhase::Idle);
    assert_ne!(
        studio.symbol_index_coordinator.status().phase,
        SymbolIndexPhase::Idle
    );
}

#[test]
fn apply_ui_config_without_eager_background_indexing_keeps_indexes_idle() {
    let studio = StudioState::new();

    studio.apply_ui_config(
        UiConfig {
            projects: vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            }],
            repo_projects: vec![repo_project("sciml")],
        },
        false,
    );

    let repo_status = studio.repo_index.status_response(None);
    assert_eq!(repo_status.total, 0);
    assert!(repo_status.repos.is_empty());
    assert_eq!(
        studio.symbol_index_coordinator.status().phase,
        SymbolIndexPhase::Idle
    );
}

#[tokio::test]
async fn set_ui_config_still_eagerly_enqueues_background_indexes() {
    let studio = StudioState::new();

    studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: vec![repo_project("sciml")],
    });

    let repo_status = studio.repo_index.status_response(None);
    assert_eq!(repo_status.total, 1);
    assert_ne!(repo_status.repos[0].phase, RepoIndexPhase::Idle);
    assert_ne!(
        studio.symbol_index_coordinator.status().phase,
        SymbolIndexPhase::Idle
    );
}

#[tokio::test]
async fn repo_index_status_bootstraps_deferred_repo_indexing() {
    let studio = StudioState::new();

    studio.apply_ui_config(
        UiConfig {
            projects: vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            }],
            repo_projects: vec![repo_project("sciml")],
        },
        false,
    );

    assert_eq!(studio.repo_index.status_response(None).total, 0);
    assert_eq!(
        studio.bootstrap_background_indexing_deferred_activation_source(),
        None
    );

    let repo_status = studio.repo_index_status(None);

    assert_eq!(repo_status.total, 1);
    assert_eq!(repo_status.repos[0].repo_id, "sciml");
    assert_ne!(repo_status.repos[0].phase, RepoIndexPhase::Idle);
    assert_eq!(
        studio.bootstrap_background_indexing_deferred_activation_source(),
        Some("repo_index_status".to_string())
    );
}

#[tokio::test]
async fn ui_capabilities_reports_builtin_plugin_languages() {
    let registry = bootstrap_builtin_registry()
        .unwrap_or_else(|error| panic!("builtin registry should bootstrap: {error:?}"));
    let expected = registry
        .plugin_ids()
        .into_iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();
    let studio = StudioState::new_with_bootstrap_ui_config(Arc::new(registry));
    studio.set_ui_config(UiConfig {
        projects: Vec::new(),
        repo_projects: vec![repo_project("kernel"), repo_project("sciml")],
    });
    let state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(studio),
    });

    let response =
        crate::gateway::studio::router::handlers::get_ui_capabilities(State(Arc::clone(&state)))
            .await
            .unwrap_or_else(|error| panic!("ui capabilities should resolve: {error:?}"))
            .0;

    assert_eq!(response.languages, expected);
    assert_eq!(response.repositories, vec!["kernel", "sciml"]);
    assert_eq!(
        response.kinds,
        crate::gateway::studio::router::state::supported_code_kinds()
    );
    assert!(!response.studio_bootstrap_background_indexing_enabled);
    assert_eq!(
        response.studio_bootstrap_background_indexing_mode,
        "deferred"
    );
    assert!(!response.studio_bootstrap_background_indexing_deferred_activation_observed);
}

#[tokio::test]
async fn symbol_index_status_records_first_deferred_bootstrap_activation() {
    let studio = StudioState::new();
    studio.apply_ui_config(
        UiConfig {
            projects: vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            }],
            repo_projects: vec![repo_project("sciml")],
        },
        false,
    );

    assert_eq!(
        studio.bootstrap_background_indexing_deferred_activation_at(),
        None
    );
    assert_eq!(
        studio.bootstrap_background_indexing_deferred_activation_source(),
        None
    );

    let _ = studio
        .symbol_index_status()
        .unwrap_or_else(|error| panic!("symbol index status should resolve: {error:?}"));

    let activated_at = studio
        .bootstrap_background_indexing_deferred_activation_at()
        .unwrap_or_else(|| panic!("deferred activation should record a timestamp"));
    DateTime::parse_from_rfc3339(&activated_at)
        .unwrap_or_else(|error| panic!("parse deferred activation timestamp: {error}"));
    assert!(
        studio
            .bootstrap_background_indexing_telemetry()
            .deferred_activation_observed()
    );
    assert_eq!(
        studio.bootstrap_background_indexing_deferred_activation_source(),
        Some("symbol_index_status".to_string())
    );
}

#[tokio::test]
#[serial]
async fn plugin_artifact_handler_returns_resolved_artifact() {
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:18080"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
analyzer_strategy = "similarity_only"
"#,
    )
    .unwrap_or_else(|error| panic!("write config: {error}"));
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(StudioState::new()),
    });

    let response = crate::gateway::studio::router::handlers::get_plugin_artifact(
        State(Arc::clone(&state)),
        Path(
            crate::gateway::studio::router::handlers::capabilities::PluginArtifactPath {
                plugin_id: "xiuxian-wendao-julia".to_string(),
                artifact_id: "deployment".to_string(),
            },
        ),
        Query(
            crate::gateway::studio::router::handlers::capabilities::PluginArtifactQuery {
                format: None,
            },
        ),
    )
    .await
    .unwrap_or_else(|error| panic!("deployment artifact handler should resolve: {error:?}"));

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_else(|error| panic!("read json body: {error}"));
    let artifact: UiPluginArtifact = serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("decode artifact json: {error}"));

    assert_eq!(artifact.plugin_id, "xiuxian-wendao-julia");
    assert_eq!(artifact.artifact_id, "deployment");
    assert_eq!(artifact.artifact_schema_version, "v1");
    DateTime::parse_from_rfc3339(&artifact.generated_at)
        .unwrap_or_else(|error| panic!("parse artifact generated_at: {error}"));
    assert_eq!(artifact.base_url.as_deref(), Some("http://127.0.0.1:18080"));
    assert_eq!(
        artifact.route.as_deref(),
        Some(DEFAULT_JULIA_RERANK_FLIGHT_ROUTE)
    );
    assert_eq!(artifact.schema_version.as_deref(), Some("v1"));
    assert_eq!(
        artifact.selected_transport,
        Some(crate::gateway::studio::types::config::UiPluginTransportKind::ArrowFlight)
    );
    assert_eq!(artifact.fallback_from, None);
    assert_eq!(artifact.fallback_reason, None);
    assert_eq!(
        artifact
            .launch
            .as_ref()
            .map(|launch| launch.launcher_path.as_str()),
        Some(DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH)
    );
}

#[tokio::test]
#[serial]
async fn plugin_artifact_handler_returns_canonical_json_shape() {
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:18080"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
"#,
    )
    .unwrap_or_else(|error| panic!("write config: {error}"));
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(StudioState::new()),
    });

    let response = crate::gateway::studio::router::handlers::get_plugin_artifact(
        State(Arc::clone(&state)),
        Path(
            crate::gateway::studio::router::handlers::capabilities::PluginArtifactPath {
                plugin_id: "xiuxian-wendao-julia".to_string(),
                artifact_id: "deployment".to_string(),
            },
        ),
        Query(
            crate::gateway::studio::router::handlers::capabilities::PluginArtifactQuery {
                format: None,
            },
        ),
    )
    .await
    .unwrap_or_else(|error| panic!("deployment artifact handler should resolve: {error:?}"));

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_else(|error| panic!("read json body: {error}"));
    let artifact: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("decode artifact json: {error}"));

    assert_eq!(artifact["pluginId"], "xiuxian-wendao-julia");
    assert_eq!(artifact["artifactId"], "deployment");
    assert_eq!(artifact["selectedTransport"], "arrow_flight");
    assert_eq!(
        artifact["launch"]["launcherPath"],
        DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH
    );
}

#[tokio::test]
#[serial]
async fn plugin_artifact_handler_returns_toml_when_requested() {
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:18080"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
"#,
    )
    .unwrap_or_else(|error| panic!("write config: {error}"));
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(StudioState::new()),
    });

    let response = crate::gateway::studio::router::handlers::get_plugin_artifact(
        State(Arc::clone(&state)),
        Path(
            crate::gateway::studio::router::handlers::capabilities::PluginArtifactPath {
                plugin_id: "xiuxian-wendao-julia".to_string(),
                artifact_id: "deployment".to_string(),
            },
        ),
        Query(
            crate::gateway::studio::router::handlers::capabilities::PluginArtifactQuery {
                format: Some(crate::zhenfa_router::native::WendaoPluginArtifactOutputFormat::Toml),
            },
        ),
    )
    .await
    .unwrap_or_else(|error| panic!("deployment artifact toml handler should resolve: {error:?}"));

    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_else(|error| panic!("read toml body: {error}"));
    let body_text =
        String::from_utf8(body.to_vec()).unwrap_or_else(|error| panic!("utf8 toml body: {error}"));

    assert_eq!(content_type, "text/plain; charset=utf-8");
    assert!(body_text.contains("base_url = \"http://127.0.0.1:18080\""));
    assert!(body_text.contains("route = \"/rerank\""));
    assert!(body_text.contains("selected_transport = \"arrow_flight\""));
}
