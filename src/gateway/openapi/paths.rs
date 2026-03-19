//! Route inventory shared by the Wendao gateway runtime and `OpenAPI` contract tests.

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
/// Axum runtime path for the VFS root endpoint.
pub const API_VFS_ROOT_AXUM_PATH: &str = "/api/vfs";
/// `OpenAPI` path for the VFS root endpoint.
pub const API_VFS_ROOT_OPENAPI_PATH: &str = "/api/vfs";
/// Axum runtime path for the VFS scan endpoint.
pub const API_VFS_SCAN_AXUM_PATH: &str = "/api/vfs/scan";
/// `OpenAPI` path for the VFS scan endpoint.
pub const API_VFS_SCAN_OPENAPI_PATH: &str = "/api/vfs/scan";
/// Axum runtime path for the VFS cat endpoint.
pub const API_VFS_CAT_AXUM_PATH: &str = "/api/vfs/cat";
/// `OpenAPI` path for the VFS cat endpoint.
pub const API_VFS_CAT_OPENAPI_PATH: &str = "/api/vfs/cat";
/// Axum runtime path for the VFS wildcard entry endpoint.
pub const API_VFS_ENTRY_AXUM_PATH: &str = "/api/vfs/{*path}";
/// `OpenAPI` path for the VFS entry endpoint.
pub const API_VFS_ENTRY_OPENAPI_PATH: &str = "/api/vfs/{path}";
/// Axum runtime path for the legacy neighbors endpoint.
pub const API_NEIGHBORS_AXUM_PATH: &str = "/api/neighbors/{*id}";
/// `OpenAPI` path for the legacy neighbors endpoint.
pub const API_NEIGHBORS_OPENAPI_PATH: &str = "/api/neighbors/{id}";
/// Axum runtime path for the graph neighbors endpoint.
pub const API_GRAPH_NEIGHBORS_AXUM_PATH: &str = "/api/graph/neighbors/{*id}";
/// `OpenAPI` path for the graph neighbors endpoint.
pub const API_GRAPH_NEIGHBORS_OPENAPI_PATH: &str = "/api/graph/neighbors/{id}";
/// Axum runtime path for the 3D topology endpoint.
pub const API_TOPOLOGY_3D_AXUM_PATH: &str = "/api/topology/3d";
/// `OpenAPI` path for the 3D topology endpoint.
pub const API_TOPOLOGY_3D_OPENAPI_PATH: &str = "/api/topology/3d";
/// Axum runtime path for the search endpoint.
pub const API_SEARCH_AXUM_PATH: &str = "/api/search";
/// `OpenAPI` path for the search endpoint.
pub const API_SEARCH_OPENAPI_PATH: &str = "/api/search";
/// Axum runtime path for the attachment search endpoint.
pub const API_SEARCH_ATTACHMENTS_AXUM_PATH: &str = "/api/search/attachments";
/// `OpenAPI` path for the attachment search endpoint.
pub const API_SEARCH_ATTACHMENTS_OPENAPI_PATH: &str = "/api/search/attachments";
/// Axum runtime path for the AST search endpoint.
pub const API_SEARCH_AST_AXUM_PATH: &str = "/api/search/ast";
/// `OpenAPI` path for the AST search endpoint.
pub const API_SEARCH_AST_OPENAPI_PATH: &str = "/api/search/ast";
/// Axum runtime path for the definition search endpoint.
pub const API_SEARCH_DEFINITION_AXUM_PATH: &str = "/api/search/definition";
/// `OpenAPI` path for the definition search endpoint.
pub const API_SEARCH_DEFINITION_OPENAPI_PATH: &str = "/api/search/definition";
/// Axum runtime path for the references search endpoint.
pub const API_SEARCH_REFERENCES_AXUM_PATH: &str = "/api/search/references";
/// `OpenAPI` path for the references search endpoint.
pub const API_SEARCH_REFERENCES_OPENAPI_PATH: &str = "/api/search/references";
/// Axum runtime path for the symbols search endpoint.
pub const API_SEARCH_SYMBOLS_AXUM_PATH: &str = "/api/search/symbols";
/// `OpenAPI` path for the symbols search endpoint.
pub const API_SEARCH_SYMBOLS_OPENAPI_PATH: &str = "/api/search/symbols";
/// Axum runtime path for the autocomplete search endpoint.
pub const API_SEARCH_AUTOCOMPLETE_AXUM_PATH: &str = "/api/search/autocomplete";
/// `OpenAPI` path for the autocomplete search endpoint.
pub const API_SEARCH_AUTOCOMPLETE_OPENAPI_PATH: &str = "/api/search/autocomplete";
/// Axum runtime path for the markdown analysis endpoint.
pub const API_ANALYSIS_MARKDOWN_AXUM_PATH: &str = "/api/analysis/markdown";
/// `OpenAPI` path for the markdown analysis endpoint.
pub const API_ANALYSIS_MARKDOWN_OPENAPI_PATH: &str = "/api/analysis/markdown";
/// Axum runtime path for the UI config endpoint.
pub const API_UI_CONFIG_AXUM_PATH: &str = "/api/ui/config";
/// `OpenAPI` path for the UI config endpoint.
pub const API_UI_CONFIG_OPENAPI_PATH: &str = "/api/ui/config";

/// Stable inventory for the current Wendao gateway route surface.
pub const WENDAO_GATEWAY_ROUTE_CONTRACTS: &[RouteContract] = &[
    RouteContract {
        axum_path: API_HEALTH_AXUM_PATH,
        openapi_path: API_HEALTH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_STATS_AXUM_PATH,
        openapi_path: API_STATS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_NOTIFY_AXUM_PATH,
        openapi_path: API_NOTIFY_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_VFS_ROOT_AXUM_PATH,
        openapi_path: API_VFS_ROOT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_VFS_SCAN_AXUM_PATH,
        openapi_path: API_VFS_SCAN_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_VFS_CAT_AXUM_PATH,
        openapi_path: API_VFS_CAT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_VFS_ENTRY_AXUM_PATH,
        openapi_path: API_VFS_ENTRY_OPENAPI_PATH,
        methods: &["get"],
        path_params: &["path"],
    },
    RouteContract {
        axum_path: API_NEIGHBORS_AXUM_PATH,
        openapi_path: API_NEIGHBORS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &["id"],
    },
    RouteContract {
        axum_path: API_GRAPH_NEIGHBORS_AXUM_PATH,
        openapi_path: API_GRAPH_NEIGHBORS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &["id"],
    },
    RouteContract {
        axum_path: API_TOPOLOGY_3D_AXUM_PATH,
        openapi_path: API_TOPOLOGY_3D_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_AXUM_PATH,
        openapi_path: API_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_ATTACHMENTS_AXUM_PATH,
        openapi_path: API_SEARCH_ATTACHMENTS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_AST_AXUM_PATH,
        openapi_path: API_SEARCH_AST_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_DEFINITION_AXUM_PATH,
        openapi_path: API_SEARCH_DEFINITION_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_REFERENCES_AXUM_PATH,
        openapi_path: API_SEARCH_REFERENCES_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_SYMBOLS_AXUM_PATH,
        openapi_path: API_SEARCH_SYMBOLS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_AUTOCOMPLETE_AXUM_PATH,
        openapi_path: API_SEARCH_AUTOCOMPLETE_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_ANALYSIS_MARKDOWN_AXUM_PATH,
        openapi_path: API_ANALYSIS_MARKDOWN_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_UI_CONFIG_AXUM_PATH,
        openapi_path: API_UI_CONFIG_OPENAPI_PATH,
        methods: &["get", "post"],
        path_params: &[],
    },
];
