use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::body::to_bytes;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::routing::Router;
use tokio::sync::mpsc;
use tower::ServiceExt;
use xiuxian_zhenfa::ZhenfaSignal;

use crate::execute::gateway::{
    command::{
        GATEWAY_FLIGHT_SERVICE_AXUM_PATH, build_gateway_router, gateway_listen_backlog_with_lookup,
        gateway_studio_concurrency_limit_with_lookup,
        gateway_studio_request_timeout_secs_with_lookup,
    },
    config::{get_webhook_from_config, resolve_port, resolve_webhook_config},
    health::gateway_health_response,
    registry::build_plugin_registry,
    shared::{AppState, DEFAULT_PORT},
    status::{notify_status, stats},
};

fn write_temp_gateway_config(contents: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "wendao-gateway-config-{}-{unique}.toml",
        std::process::id()
    ));
    if let Err(err) = std::fs::write(&path, contents) {
        panic!("failed to write temp config at {}: {err}", path.display());
    }
    path
}

fn remove_temp_gateway_config(path: &Path) {
    if let Err(err) = std::fs::remove_file(path)
        && path.exists()
    {
        panic!("failed to remove temp config at {}: {err}", path.display());
    }
}

fn write_temp_gateway_pidfile(contents: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "wendao-gateway-pidfile-{}-{unique}.pid",
        std::process::id()
    ));
    if let Err(err) = std::fs::write(&path, contents) {
        panic!("failed to write temp pidfile at {}: {err}", path.display());
    }
    path
}

fn remove_temp_gateway_pidfile(path: &Path) {
    if let Err(err) = std::fs::remove_file(path)
        && path.exists()
    {
        panic!("failed to remove temp pidfile at {}: {err}", path.display());
    }
}

fn mismatched_pid() -> u32 {
    let current = std::process::id();
    if current == u32::MAX {
        current - 1
    } else {
        current + 1
    }
}

fn app_state(signal_tx: Option<tokio::sync::mpsc::UnboundedSender<ZhenfaSignal>>) -> Arc<AppState> {
    Arc::new(AppState::new(None, signal_tx, bootstrap_builtin_registry()))
}

fn bootstrap_builtin_registry() -> Arc<xiuxian_wendao::analyzers::PluginRegistry> {
    build_plugin_registry().unwrap_or_else(|error| panic!("bootstrap builtin registry: {error}"))
}

#[test]
fn test_default_port() {
    assert_eq!(DEFAULT_PORT, 9517);
}

#[test]
fn test_gateway_listen_backlog_defaults_when_env_missing() {
    let backlog = gateway_listen_backlog_with_lookup(&|_| None);
    assert_eq!(backlog, 2048);
}

#[test]
fn test_gateway_listen_backlog_accepts_positive_override() {
    let backlog = gateway_listen_backlog_with_lookup(&|key| {
        if key == "XIUXIAN_WENDAO_GATEWAY_LISTEN_BACKLOG" {
            Some("4096".to_string())
        } else {
            None
        }
    });
    assert_eq!(backlog, 4096);
}

#[test]
fn test_gateway_listen_backlog_clamps_invalid_override() {
    let backlog = gateway_listen_backlog_with_lookup(&|key| {
        if key == "XIUXIAN_WENDAO_GATEWAY_LISTEN_BACKLOG" {
            Some("0".to_string())
        } else {
            None
        }
    });
    assert_eq!(backlog, 2048);
}

#[test]
fn test_gateway_studio_concurrency_limit_defaults_from_parallelism() {
    let limit = gateway_studio_concurrency_limit_with_lookup(&|_| None, Some(8));
    assert_eq!(limit, 32);
}

#[test]
fn test_gateway_studio_concurrency_limit_accepts_positive_override() {
    let limit = gateway_studio_concurrency_limit_with_lookup(
        &|key| {
            if key == "XIUXIAN_WENDAO_GATEWAY_STUDIO_CONCURRENCY_LIMIT" {
                Some("96".to_string())
            } else {
                None
            }
        },
        Some(8),
    );
    assert_eq!(limit, 96);
}

#[test]
fn test_gateway_studio_concurrency_limit_ignores_invalid_override() {
    let limit = gateway_studio_concurrency_limit_with_lookup(
        &|key| {
            if key == "XIUXIAN_WENDAO_GATEWAY_STUDIO_CONCURRENCY_LIMIT" {
                Some("-1".to_string())
            } else {
                None
            }
        },
        Some(8),
    );
    assert_eq!(limit, 32);
}

#[test]
fn test_gateway_studio_concurrency_limit_clamps_large_override() {
    let limit = gateway_studio_concurrency_limit_with_lookup(
        &|key| {
            if key == "XIUXIAN_WENDAO_GATEWAY_STUDIO_CONCURRENCY_LIMIT" {
                Some("320".to_string())
            } else {
                None
            }
        },
        Some(8),
    );
    assert_eq!(limit, 128);
}

#[test]
fn test_gateway_studio_request_timeout_defaults_when_env_missing() {
    let timeout = gateway_studio_request_timeout_secs_with_lookup(&|_| None);
    assert_eq!(timeout, 15);
}

#[test]
fn test_gateway_studio_request_timeout_accepts_positive_override() {
    let timeout = gateway_studio_request_timeout_secs_with_lookup(&|key| {
        if key == "XIUXIAN_WENDAO_GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS" {
            Some("25".to_string())
        } else {
            None
        }
    });
    assert_eq!(timeout, 25);
}

#[test]
fn test_gateway_studio_request_timeout_clamps_invalid_override() {
    let timeout = gateway_studio_request_timeout_secs_with_lookup(&|key| {
        if key == "XIUXIAN_WENDAO_GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS" {
            Some("0".to_string())
        } else {
            None
        }
    });
    assert_eq!(timeout, 15);
}

#[test]
fn test_resolve_port_cli_priority() {
    let port = resolve_port(Some(8080), None);
    assert_eq!(port, 8080);
}

#[test]
fn test_resolve_port_default() {
    let port = resolve_port(None, None);
    assert_eq!(port, DEFAULT_PORT);
}

#[test]
fn test_resolve_port_from_cli_config_path() {
    let config_path = write_temp_gateway_config(
        r"
[gateway]
port = 18080
",
    );

    let port = resolve_port(None, Some(config_path.as_path()));
    remove_temp_gateway_config(&config_path);

    assert_eq!(port, 18080);
}

#[test]
fn test_health_endpoint_reports_process_id_header() {
    let response = gateway_health_response(None);

    assert_eq!(response.status(), StatusCode::OK);
    let process_id = response
        .headers()
        .get("x-wendao-process-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_else(|| panic!("health response should include a process id header"));
    assert_eq!(
        process_id.parse::<u32>().unwrap_or_else(|error| panic!(
            "health response header should be a valid process id: {error}"
        )),
        std::process::id()
    );
}

#[test]
fn test_health_endpoint_accepts_owned_pidfile() {
    let pidfile = write_temp_gateway_pidfile(&std::process::id().to_string());
    let response = gateway_health_response(Some(pidfile.as_path()));
    remove_temp_gateway_pidfile(&pidfile);

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_endpoint_rejects_mismatched_pidfile() {
    let pidfile = write_temp_gateway_pidfile(&mismatched_pid().to_string());
    let response = gateway_health_response(Some(pidfile.as_path()));
    remove_temp_gateway_pidfile(&pidfile);

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let process_id = response
        .headers()
        .get("x-wendao-process-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_else(|| panic!("health error response should include a process id header"));
    assert_eq!(
        process_id.parse::<u32>().unwrap_or_else(|error| panic!(
            "health error response header should be a valid process id: {error}"
        )),
        std::process::id()
    );
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_else(|error| panic!("health error response should be readable: {error}"));
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("health error response should be valid json: {error}"));
    assert_eq!(payload["error"], "gateway is not ready");
    assert_eq!(payload["expectedPid"], serde_json::json!(mismatched_pid()));
    assert_eq!(payload["processId"], serde_json::json!(std::process::id()));
}

#[tokio::test]
async fn test_stats_endpoint_no_index() {
    let state = app_state(None);
    let result = stats(State(state)).await;
    assert_eq!(result.0["error"], "no index loaded");
}

#[tokio::test]
async fn test_notify_status_endpoint() {
    let (tx, _rx) = mpsc::unbounded_channel();
    let state = app_state(Some(tx));
    let expected_bootstrap_background_indexing =
        state.studio.bootstrap_background_indexing_enabled();
    let expected_bootstrap_background_indexing_mode =
        state.studio.bootstrap_background_indexing_mode();
    let result = notify_status(State(state)).await;
    assert_eq!(result.0["notification_worker"], "active");
    assert_eq!(
        result.0["studio_bootstrap_background_indexing_enabled"],
        serde_json::json!(expected_bootstrap_background_indexing)
    );
    assert_eq!(
        result.0["studio_bootstrap_background_indexing_mode"],
        expected_bootstrap_background_indexing_mode
    );
    assert_eq!(
        result.0["studio_bootstrap_background_indexing_deferred_activation_observed"],
        serde_json::json!(false)
    );
    assert!(result.0["studio_bootstrap_background_indexing_deferred_activation_at"].is_null());
    assert!(result.0["studio_bootstrap_background_indexing_deferred_activation_source"].is_null());
}

#[tokio::test]
async fn test_notify_status_no_channel() {
    let state = app_state(None);
    let expected_bootstrap_background_indexing =
        state.studio.bootstrap_background_indexing_enabled();
    let expected_bootstrap_background_indexing_mode =
        state.studio.bootstrap_background_indexing_mode();
    let result = notify_status(State(state)).await;
    assert_eq!(result.0["notification_worker"], "inactive");
    assert_eq!(
        result.0["studio_bootstrap_background_indexing_enabled"],
        serde_json::json!(expected_bootstrap_background_indexing)
    );
    assert_eq!(
        result.0["studio_bootstrap_background_indexing_mode"],
        expected_bootstrap_background_indexing_mode
    );
    assert_eq!(
        result.0["studio_bootstrap_background_indexing_deferred_activation_observed"],
        serde_json::json!(false)
    );
    assert!(result.0["studio_bootstrap_background_indexing_deferred_activation_at"].is_null());
    assert!(result.0["studio_bootstrap_background_indexing_deferred_activation_source"].is_null());
}

#[tokio::test]
async fn test_gateway_server_bind() {
    // Test that we can create the router and bind to a port
    let (tx, _rx) = mpsc::unbounded_channel();
    let app_state = app_state(Some(tx));
    let app: Router = build_gateway_router(app_state, 32, std::time::Duration::from_secs(15))
        .unwrap_or_else(|error| panic!("gateway router should build: {error}"));

    // Bind to a random available port
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr).await;
    assert!(listener.is_ok(), "Should be able to bind to random port");

    // Prevent unused variable warning
    let _ = app;
}

#[cfg(feature = "julia")]
#[tokio::test]
async fn test_gateway_router_mounts_flight_service_on_same_listener() {
    let router = build_gateway_router(app_state(None), 32, std::time::Duration::from_secs(15))
        .unwrap_or_else(|error| panic!("gateway router should build: {error}"));
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/arrow.flight.protocol.FlightService/GetFlightInfo")
                .body(Body::empty())
                .unwrap_or_else(|error| panic!("request should build: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("router should answer Flight requests: {error}"));

    assert_eq!(
        GATEWAY_FLIGHT_SERVICE_AXUM_PATH,
        "/arrow.flight.protocol.FlightService/{*grpc_method}"
    );
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_webhook_config_from_env() {
    // Test default config
    let config = resolve_webhook_config(None);
    assert!(config.url.is_empty());
    assert!(config.secret.is_none());
    assert_eq!(config.timeout_secs, 10);
    assert!(config.retry_on_failure);
}

#[test]
fn test_resolve_webhook_config_from_cli_config_path() {
    let config_path = write_temp_gateway_config(
        r#"
[gateway]
webhook_url = "http://127.0.0.1:9999"
webhook_secret = "test-secret"
webhook_enabled = true
"#,
    );

    let config = resolve_webhook_config(Some(config_path.as_path()));
    remove_temp_gateway_config(&config_path);

    assert_eq!(config.url, "http://127.0.0.1:9999");
    assert_eq!(config.secret.as_deref(), Some("test-secret"));
    assert_eq!(config.timeout_secs, 10);
}

#[test]
fn test_disabled_webhook_config_is_ignored() {
    let config_path = write_temp_gateway_config(
        r#"
[gateway]
webhook_url = "http://127.0.0.1:9999"
webhook_enabled = false
"#,
    );

    let config = get_webhook_from_config(Some(config_path.as_path()));
    remove_temp_gateway_config(&config_path);

    assert!(config.is_none());
}

#[test]
fn test_build_plugin_registry_bootstraps_builtin_plugins() {
    let registry = bootstrap_builtin_registry();
    assert!(registry.plugin_ids().contains(&"julia"));
    assert!(registry.plugin_ids().contains(&"modelica"));
}

#[tokio::test]
async fn test_notification_channel() {
    let (tx, mut rx) = mpsc::unbounded_channel::<ZhenfaSignal>();

    // Send a test signal
    let signal = ZhenfaSignal::SemanticDrift {
        source_path: "test.rs".to_string(),
        file_stem: "test".to_string(),
        affected_count: 1,
        confidence: "high".to_string(),
        summary: "Test drift".to_string(),
    };
    assert!(tx.send(signal).is_ok());

    // Receive it
    let Some(received) = rx.recv().await else {
        panic!("notification channel should receive the test signal");
    };
    assert!(matches!(received, ZhenfaSignal::SemanticDrift { .. }));
}

#[test]
fn test_parse_port_from_toml_content() {
    let content = r"
[gateway]
port = 8080
";
    // Test that we can parse port from TOML content
    let mut found_port = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("port")
            && let Some(eq_pos) = line.find('=')
        {
            let value = line[eq_pos + 1..].trim().trim_matches('"');
            if let Ok(port) = value.parse::<u16>() {
                assert_eq!(port, 8080);
                found_port = true;
            }
        }
    }
    assert!(found_port);
}
