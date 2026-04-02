use crate::gateway::openapi::paths::*;

pub(super) const SEARCH_INDEX_STATUS: RouteContract = RouteContract {
    axum_path: API_SEARCH_INDEX_STATUS_AXUM_PATH,
    openapi_path: API_SEARCH_INDEX_STATUS_OPENAPI_PATH,
    methods: &["get"],
    path_params: &[],
};
