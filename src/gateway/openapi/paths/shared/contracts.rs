/// One declared route contract in the Wendao gateway surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteContract {
    /// The Axum runtime path pattern.
    pub axum_path: &'static str,
    /// The normalized `OpenAPI` path pattern.
    pub openapi_path: &'static str,
    /// Supported lowercase HTTP methods.
    pub methods: &'static [&'static str],
    /// Required `OpenAPI` path parameter names for this route.
    pub path_params: &'static [&'static str],
}

/// Axum runtime path for the health endpoint.
pub const API_HEALTH_AXUM_PATH: &str = "/api/health";
/// `OpenAPI` path for the health endpoint.
pub const API_HEALTH_OPENAPI_PATH: &str = "/api/health";
/// Axum runtime path for the stats endpoint.
pub const API_STATS_AXUM_PATH: &str = "/api/stats";
/// `OpenAPI` path for the stats endpoint.
pub const API_STATS_OPENAPI_PATH: &str = "/api/stats";
/// Axum runtime path for the notify endpoint.
pub const API_NOTIFY_AXUM_PATH: &str = "/api/notify";
/// `OpenAPI` path for the notify endpoint.
pub const API_NOTIFY_OPENAPI_PATH: &str = "/api/notify";
