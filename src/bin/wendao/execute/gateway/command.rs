//! Gateway command execution.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
#[cfg(feature = "julia")]
use arrow_flight::flight_service_server::FlightServiceServer;
use axum::Json;
use axum::error_handling::HandleErrorLayer;
use axum::http::StatusCode;
#[cfg(feature = "julia")]
use axum::routing::any_service;
use axum::routing::{Router, get};
use log::info;
use tokio::sync::mpsc;
#[cfg(feature = "julia")]
use tonic_web::GrpcWebLayer;
#[cfg(feature = "julia")]
use tower::Layer;
use tower::{BoxError, ServiceBuilder};

use crate::execute::gateway::{
    config::{resolve_config_path, resolve_port, resolve_webhook_config},
    health::health,
    registry::build_plugin_registry,
    shared::AppState,
    status::{notify_status, stats},
};
use crate::types::{Cli, GatewayArgs, GatewayCommand, GatewayStartArgs};
use xiuxian_wendao::LinkGraphIndex;
use xiuxian_wendao::gateway::{openapi::paths as openapi_paths, studio::studio_routes};
#[cfg(feature = "julia")]
use xiuxian_wendao::link_graph::plugin_runtime::build_search_plane_studio_flight_service_with_weights;
#[cfg(feature = "julia")]
use xiuxian_wendao::link_graph::resolve_link_graph_rerank_flight_runtime_settings;
#[cfg(feature = "julia")]
use xiuxian_wendao_runtime::transport::{
    EffectiveRerankFlightHostSettings, rerank_score_weights_from_env,
    resolve_effective_rerank_flight_host_settings as resolve_runtime_effective_rerank_flight_host_settings,
};
use xiuxian_zhenfa::{NotificationService, ZhenfaSignal, notification_worker};

const GATEWAY_LISTEN_BACKLOG_ENV: &str = "XIUXIAN_WENDAO_GATEWAY_LISTEN_BACKLOG";
const GATEWAY_STUDIO_CONCURRENCY_LIMIT_ENV: &str =
    "XIUXIAN_WENDAO_GATEWAY_STUDIO_CONCURRENCY_LIMIT";
const GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS_ENV: &str =
    "XIUXIAN_WENDAO_GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS";
const DEFAULT_GATEWAY_LISTEN_BACKLOG: u32 = 2048;
const MIN_GATEWAY_LISTEN_BACKLOG: u32 = 128;
const MAX_GATEWAY_LISTEN_BACKLOG: u32 = 8192;
const DEFAULT_GATEWAY_STUDIO_CONCURRENCY_FALLBACK: usize = 8;
const MIN_GATEWAY_STUDIO_CONCURRENCY_LIMIT: usize = 32;
const MAX_GATEWAY_STUDIO_CONCURRENCY_LIMIT: usize = 128;
const DEFAULT_GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS: u64 = 15;
const MIN_GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS: u64 = 5;
const MAX_GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS: u64 = 60;
pub(crate) const GATEWAY_FLIGHT_SERVICE_AXUM_PATH: &str =
    "/arrow.flight.protocol.FlightService/{*grpc_method}";
const DEFAULT_GATEWAY_SEARCH_FLIGHT_REPO_ID: &str = "alpha/repo";
#[cfg(feature = "julia")]
const DEFAULT_GATEWAY_SEARCH_FLIGHT_RERANK_DIMENSION: usize = 3;

/// Handle the gateway command.
pub(crate) async fn handle(
    cli: &Cli,
    args: &GatewayArgs,
    index: Option<&LinkGraphIndex>,
) -> Result<()> {
    match &args.command {
        GatewayCommand::Start(start_args) => handle_start(cli, start_args, index).await,
    }
}

/// Handle the `gateway start` subcommand.
async fn handle_start(
    cli: &Cli,
    args: &GatewayStartArgs,
    index: Option<&LinkGraphIndex>,
) -> Result<()> {
    let config_path = resolve_config_path(cli.config_file.as_deref());

    // Resolve port: CLI arg > config file > default
    let port = resolve_port(args.port, config_path.as_deref());

    // 1. Start Webhook notification sidecar
    let (signal_tx, signal_rx) = mpsc::unbounded_channel::<ZhenfaSignal>();

    // Configure webhook: TOML > env var > defaults
    let webhook_config = resolve_webhook_config(config_path.as_deref());

    let notification_service = Arc::new(NotificationService::new(webhook_config));

    // Spawn the notification worker as a background task
    tokio::spawn(notification_worker(
        signal_rx,
        Arc::clone(&notification_service),
    ));
    info!(
        "Gateway: Notification worker started (id={})",
        notification_service.id()
    );

    // 2. Create app state with index and signal channel
    // Note: Julia/Modelica plugins should be registered here if this crate
    // depended on them. Since it doesn't (to avoid circular dependency),
    // they are currently empty. A separate aggregator crate would be needed
    // to provide a pre-populated registry.
    let app_state = Arc::new(AppState::new(
        index.map(|i| Arc::new(i.clone())),
        Some(signal_tx),
        build_plugin_registry()?,
    ));

    let listen_backlog = gateway_listen_backlog();
    let studio_concurrency_limit = gateway_studio_concurrency_limit();
    let studio_request_timeout = gateway_studio_request_timeout();

    // 3. Build the Axum router
    let app = build_gateway_router(
        app_state.clone(),
        studio_concurrency_limit,
        studio_request_timeout,
    )?;

    // 4. Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Starting Wendao Gateway on port {port}");
    info!(
        "Gateway listener backlog={listen_backlog}, studio concurrency limit={studio_concurrency_limit}, studio request timeout={}s",
        studio_request_timeout.as_secs()
    );
    info!("Endpoints:");
    info!(
        "  - GET {}  - Health check",
        openapi_paths::API_HEALTH_AXUM_PATH
    );
    info!(
        "  - GET {}   - Graph statistics",
        openapi_paths::API_STATS_AXUM_PATH
    );
    info!(
        "  - GET {}  - Notification service status",
        openapi_paths::API_NOTIFY_AXUM_PATH
    );
    #[cfg(feature = "julia")]
    info!(
        "  - POST {}  - Arrow Flight business plane",
        GATEWAY_FLIGHT_SERVICE_AXUM_PATH
    );

    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.bind(addr)?;
    let listener = socket.listen(listen_backlog)?;
    Ok(axum::serve(listener, app).await?)
}

pub(crate) fn build_gateway_router(
    app_state: Arc<AppState>,
    studio_concurrency_limit: usize,
    studio_request_timeout: Duration,
) -> Result<Router> {
    let studio_app = studio_routes().layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_gateway_service_error))
            .load_shed()
            .timeout(studio_request_timeout)
            .concurrency_limit(studio_concurrency_limit),
    );
    let app = Router::new()
        .route(openapi_paths::API_HEALTH_AXUM_PATH, get(health))
        .route(openapi_paths::API_STATS_AXUM_PATH, get(stats))
        .route(openapi_paths::API_NOTIFY_AXUM_PATH, get(notify_status))
        .merge(studio_app)
        .with_state(app_state.clone());

    #[cfg(feature = "julia")]
    let app = mount_gateway_flight_service(app, app_state)?;

    Ok(app)
}

#[cfg(feature = "julia")]
fn mount_gateway_flight_service(app: Router, app_state: Arc<AppState>) -> Result<Router> {
    let effective_settings = resolve_gateway_effective_search_host_settings()?;
    let flight_service = build_search_plane_studio_flight_service_with_weights(
        Arc::new(app_state.studio.search_plane_service()),
        DEFAULT_GATEWAY_SEARCH_FLIGHT_REPO_ID,
        app_state,
        effective_settings.expected_schema_version,
        effective_settings.rerank_dimension,
        effective_settings.rerank_weights,
    )
    .map_err(anyhow::Error::msg)?;
    let flight_service = GrpcWebLayer::new().layer(FlightServiceServer::new(flight_service));
    Ok(app.route(
        GATEWAY_FLIGHT_SERVICE_AXUM_PATH,
        any_service(flight_service),
    ))
}

#[cfg(feature = "julia")]
fn resolve_gateway_effective_search_host_settings() -> Result<EffectiveRerankFlightHostSettings> {
    let file_backed_settings = resolve_link_graph_rerank_flight_runtime_settings();
    Ok(resolve_runtime_effective_rerank_flight_host_settings(
        None,
        None,
        file_backed_settings.schema_version,
        file_backed_settings.score_weights,
        DEFAULT_GATEWAY_SEARCH_FLIGHT_RERANK_DIMENSION,
        rerank_score_weights_from_env().map_err(anyhow::Error::msg)?,
    ))
}

async fn handle_gateway_service_error(error: BoxError) -> (StatusCode, Json<serde_json::Value>) {
    if error.is::<tower::timeout::error::Elapsed>() {
        log::warn!("Gateway studio router timed out: {error}");
        return (
            StatusCode::GATEWAY_TIMEOUT,
            Json(serde_json::json!({
                "error": "gateway request timed out",
                "code": "GATEWAY_TIMEOUT",
            })),
        );
    }
    log::warn!("Gateway studio router overloaded: {error}");
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "gateway is overloaded",
            "code": "GATEWAY_OVERLOADED",
        })),
    )
}

pub(crate) fn gateway_listen_backlog() -> u32 {
    gateway_listen_backlog_with_lookup(&|key| std::env::var(key).ok())
}

pub(crate) fn gateway_listen_backlog_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> u32 {
    lookup(GATEWAY_LISTEN_BACKLOG_ENV)
        .and_then(|raw| raw.trim().parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_GATEWAY_LISTEN_BACKLOG)
        .clamp(MIN_GATEWAY_LISTEN_BACKLOG, MAX_GATEWAY_LISTEN_BACKLOG)
}

pub(crate) fn gateway_studio_concurrency_limit() -> usize {
    gateway_studio_concurrency_limit_with_lookup(
        &|key| std::env::var(key).ok(),
        std::thread::available_parallelism()
            .ok()
            .map(std::num::NonZeroUsize::get),
    )
}

pub(crate) fn gateway_studio_concurrency_limit_with_lookup(
    lookup: &dyn Fn(&str) -> Option<String>,
    available_parallelism: Option<usize>,
) -> usize {
    lookup(GATEWAY_STUDIO_CONCURRENCY_LIMIT_ENV)
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(|| default_gateway_studio_concurrency_limit(available_parallelism))
        .clamp(
            MIN_GATEWAY_STUDIO_CONCURRENCY_LIMIT,
            MAX_GATEWAY_STUDIO_CONCURRENCY_LIMIT,
        )
}

fn default_gateway_studio_concurrency_limit(available_parallelism: Option<usize>) -> usize {
    available_parallelism
        .unwrap_or(DEFAULT_GATEWAY_STUDIO_CONCURRENCY_FALLBACK)
        .saturating_mul(4)
        .clamp(
            MIN_GATEWAY_STUDIO_CONCURRENCY_LIMIT,
            MAX_GATEWAY_STUDIO_CONCURRENCY_LIMIT,
        )
}

pub(crate) fn gateway_studio_request_timeout() -> Duration {
    Duration::from_secs(gateway_studio_request_timeout_secs_with_lookup(&|key| {
        std::env::var(key).ok()
    }))
}

pub(crate) fn gateway_studio_request_timeout_secs_with_lookup(
    lookup: &dyn Fn(&str) -> Option<String>,
) -> u64 {
    lookup(GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS_ENV)
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS)
        .clamp(
            MIN_GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS,
            MAX_GATEWAY_STUDIO_REQUEST_TIMEOUT_SECS,
        )
}
