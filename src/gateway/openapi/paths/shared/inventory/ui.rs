use crate::gateway::openapi::paths::*;

pub(super) const UI_CONFIG: RouteContract = RouteContract {
    axum_path: API_UI_CONFIG_AXUM_PATH,
    openapi_path: API_UI_CONFIG_OPENAPI_PATH,
    methods: &["get", "post"],
    path_params: &[],
};

pub(super) const UI_CAPABILITIES: RouteContract = RouteContract {
    axum_path: API_UI_CAPABILITIES_AXUM_PATH,
    openapi_path: API_UI_CAPABILITIES_OPENAPI_PATH,
    methods: &["get"],
    path_params: &[],
};

pub(crate) const UI_PLUGIN_ARTIFACT: RouteContract = RouteContract {
    axum_path: API_UI_PLUGIN_ARTIFACT_AXUM_PATH,
    openapi_path: API_UI_PLUGIN_ARTIFACT_OPENAPI_PATH,
    methods: &["get"],
    path_params: &["plugin_id", "artifact_id"],
};
