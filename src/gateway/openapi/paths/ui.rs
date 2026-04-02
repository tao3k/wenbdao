/// Axum runtime path for the UI config endpoint.
pub const API_UI_CONFIG_AXUM_PATH: &str = "/api/ui/config";
/// `OpenAPI` path for the UI config endpoint.
pub const API_UI_CONFIG_OPENAPI_PATH: &str = "/api/ui/config";
/// Axum runtime path for the UI capabilities endpoint.
pub const API_UI_CAPABILITIES_AXUM_PATH: &str = "/api/ui/capabilities";
/// `OpenAPI` path for the UI capabilities endpoint.
pub const API_UI_CAPABILITIES_OPENAPI_PATH: &str = "/api/ui/capabilities";
/// Axum runtime path for the generic plugin artifact inspection endpoint.
pub const API_UI_PLUGIN_ARTIFACT_AXUM_PATH: &str =
    "/api/ui/plugins/{plugin_id}/artifacts/{artifact_id}";
/// `OpenAPI` path for the generic plugin artifact inspection endpoint.
pub const API_UI_PLUGIN_ARTIFACT_OPENAPI_PATH: &str =
    "/api/ui/plugins/{plugin_id}/artifacts/{artifact_id}";
