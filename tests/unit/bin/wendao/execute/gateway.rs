use super::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
fn test_default_port() {
    assert_eq!(DEFAULT_PORT, 9517);
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

#[tokio::test]
async fn test_health_endpoint() {
    let result = health().await;
    assert_eq!(result.0, "ok");
}

#[tokio::test]
async fn test_stats_endpoint_no_index() {
    let state = Arc::new(AppState::new(None, None));
    let result = stats(State(state)).await;
    assert_eq!(result.0["error"], "no index loaded");
}

#[tokio::test]
async fn test_notify_status_endpoint() {
    let (tx, _rx) = mpsc::unbounded_channel();
    let state = Arc::new(AppState::new(None, Some(tx)));
    let result = notify_status(State(state)).await;
    assert_eq!(result.0["notification_worker"], "active");
}

#[tokio::test]
async fn test_notify_status_no_channel() {
    let state = Arc::new(AppState::new(None, None));
    let result = notify_status(State(state)).await;
    assert_eq!(result.0["notification_worker"], "inactive");
}

#[tokio::test]
async fn test_gateway_server_bind() {
    // Test that we can create the router and bind to a port
    let (tx, _rx) = mpsc::unbounded_channel();
    let app_state = Arc::new(AppState::new(None, Some(tx)));
    let app: Router<Arc<AppState>> = Router::new()
        .route("/api/health", get(health))
        .route("/api/stats", get(stats))
        .route("/api/notify", get(notify_status))
        .with_state(app_state);

    // Bind to a random available port
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr).await;
    assert!(listener.is_ok(), "Should be able to bind to random port");

    // Prevent unused variable warning
    let _ = app;
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
