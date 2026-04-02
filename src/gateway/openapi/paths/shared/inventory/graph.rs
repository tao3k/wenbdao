use crate::gateway::openapi::paths::*;

pub(super) const TOPOLOGY_3D: RouteContract = RouteContract {
    axum_path: API_TOPOLOGY_3D_AXUM_PATH,
    openapi_path: API_TOPOLOGY_3D_OPENAPI_PATH,
    methods: &["get"],
    path_params: &[],
};
