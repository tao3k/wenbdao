use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;

use axum::{
    Json, Router,
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use xiuxian_io::PrjDirs;
use xiuxian_zhenfa::ZhenfaSignal;

use crate::link_graph::LinkGraphIndex;
use crate::unified_symbol::UnifiedSymbolIndex;

use super::types::{
    ApiError, AstSearchHit, GraphNeighborsResponse, NodeNeighbors, UiConfig, UiProjectConfig,
    VfsContentResponse, VfsEntry, VfsScanResult,
};
use super::{graph, search, vfs};

/// Shared state for the Studio API.
///
/// Contains configuration, VFS roots, and cached graph index.
pub struct StudioState {
    pub(crate) project_root: PathBuf,
    pub(crate) ui_config: Arc<RwLock<UiConfig>>,
    pub(crate) graph_index: Arc<RwLock<Option<Arc<LinkGraphIndex>>>>,
    pub(crate) symbol_index: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
    pub(crate) ast_index: Arc<RwLock<Option<Arc<Vec<AstSearchHit>>>>>,
}

/// Shared state used by the top-level gateway process.
#[derive(Clone)]
pub struct GatewayState {
    /// Optional graph index for CLI-powered stats endpoint.
    pub index: Option<Arc<LinkGraphIndex>>,
    /// Signal sender for notification worker.
    pub signal_tx: Option<tokio::sync::mpsc::UnboundedSender<ZhenfaSignal>>,
    /// Studio-specific state for VFS/graph/search APIs.
    pub studio: Arc<StudioState>,
}

impl GatewayState {
    /// Create gateway state shared by the CLI endpoints and Studio router.
    #[must_use]
    pub fn new(
        index: Option<Arc<LinkGraphIndex>>,
        signal_tx: Option<tokio::sync::mpsc::UnboundedSender<ZhenfaSignal>>,
    ) -> Self {
        Self {
            index,
            signal_tx,
            studio: Arc::new(StudioState::new()),
        }
    }

    pub(crate) async fn link_graph_index(&self) -> Result<Arc<LinkGraphIndex>, StudioApiError> {
        self.studio.graph_index().await
    }
}

impl StudioState {
    /// Create a new `StudioState` with default configuration.
    #[must_use]
    pub fn new() -> Self {
        let project_root = PrjDirs::project_root();
        Self {
            project_root,
            ui_config: Arc::new(RwLock::new(UiConfig {
                projects: Vec::new(),
            })),
            graph_index: Arc::new(RwLock::new(None)),
            symbol_index: Arc::new(RwLock::new(None)),
            ast_index: Arc::new(RwLock::new(None)),
        }
    }

    pub(crate) fn ui_config(&self) -> UiConfig {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn set_ui_config(&self, config: UiConfig) {
        let sanitized = sanitize_projects(config.projects);
        let mut guard = self
            .ui_config
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.projects = sanitized;
        drop(guard);

        let mut graph_guard = self
            .graph_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *graph_guard = None;
        drop(graph_guard);

        let mut symbol_guard = self
            .symbol_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *symbol_guard = None;
        drop(symbol_guard);

        let mut ast_guard = self
            .ast_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *ast_guard = None;
    }

    pub(crate) fn configured_projects(&self) -> Vec<UiProjectConfig> {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .projects
            .clone()
    }

    pub(crate) async fn graph_index(&self) -> Result<Arc<LinkGraphIndex>, StudioApiError> {
        if let Some(index) = self
            .graph_index
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            return Ok(Arc::clone(index));
        }

        let project_root = self.project_root.clone();
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio graph access requires configured link_graph.projects",
            ));
        }

        let build = tokio::task::spawn_blocking(move || {
            let include_dirs = graph_include_dirs(project_root.as_path(), &configured_projects);
            if include_dirs.is_empty() {
                Err(
                    "configured link_graph.projects did not produce any graph include dirs"
                        .to_string(),
                )
            } else {
                LinkGraphIndex::build_with_filters(project_root.as_path(), &include_dirs, &[])
            }
        })
        .await
        .map_err(|error| {
            StudioApiError::internal(
                "LINK_GRAPH_BUILD_PANIC",
                "Failed to build link graph index",
                Some(error.to_string()),
            )
        })?;
        let index = Arc::new(build.map_err(|error| {
            StudioApiError::internal(
                "LINK_GRAPH_BUILD_FAILED",
                "Failed to build link graph index",
                Some(error),
            )
        })?);

        let mut guard = self
            .graph_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = guard.as_ref() {
            return Ok(Arc::clone(existing));
        }
        *guard = Some(Arc::clone(&index));
        Ok(index)
    }

    pub(crate) async fn symbol_index(&self) -> Result<Arc<UnifiedSymbolIndex>, StudioApiError> {
        let project_root = self.project_root.clone();
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio symbol search requires configured link_graph.projects",
            ));
        }

        if let Some(index) = self
            .symbol_index
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            return Ok(Arc::clone(index));
        }

        let build = tokio::task::spawn_blocking(move || {
            search::build_symbol_index(project_root.as_path(), &configured_projects)
        })
        .await
        .map_err(|error| {
            StudioApiError::internal(
                "SYMBOL_INDEX_BUILD_PANIC",
                "Failed to build studio symbol index",
                Some(error.to_string()),
            )
        })?;
        let index = Arc::new(build.map_err(|error| {
            StudioApiError::internal(
                "SYMBOL_INDEX_BUILD_FAILED",
                "Failed to build studio symbol index",
                Some(error),
            )
        })?);

        let mut guard = self
            .symbol_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = guard.as_ref() {
            return Ok(Arc::clone(existing));
        }
        *guard = Some(Arc::clone(&index));
        Ok(index)
    }

    pub(crate) async fn ast_index(&self) -> Result<Arc<Vec<AstSearchHit>>, StudioApiError> {
        let project_root = self.project_root.clone();
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio AST search requires configured link_graph.projects",
            ));
        }

        if let Some(index) = self
            .ast_index
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            return Ok(Arc::clone(index));
        }

        let build = tokio::task::spawn_blocking(move || {
            search::build_ast_index(project_root.as_path(), &configured_projects)
        })
        .await
        .map_err(|error| {
            StudioApiError::internal(
                "AST_INDEX_BUILD_PANIC",
                "Failed to build studio AST index",
                Some(error.to_string()),
            )
        })?;
        let index = Arc::new(build.map_err(|error| {
            StudioApiError::internal(
                "AST_INDEX_BUILD_FAILED",
                "Failed to build studio AST index",
                Some(error),
            )
        })?);

        let mut guard = self
            .ast_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = guard.as_ref() {
            return Ok(Arc::clone(existing));
        }
        *guard = Some(Arc::clone(&index));
        Ok(index)
    }
}

impl Default for StudioState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct VfsCatQuery {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphNeighborsQuery {
    direction: Option<String>,
    hops: Option<usize>,
    limit: Option<usize>,
}

/// Create the Studio API router with all endpoints.
///
/// # Endpoints
///
/// - `GET /api/vfs` - List root entries
/// - `GET /api/vfs/scan` - Scan all VFS roots
/// - `GET /api/vfs/cat?path=` - Read file content
/// - `GET /api/vfs/{*path}` - Get single entry
/// - `GET /api/neighbors/{*id}` - Get node neighbors
/// - `GET /api/graph/neighbors/{*id}` - Get graph neighbors
/// - `GET /api/topology/3d` - Get deterministic graph topology payload
/// - `GET /api/search` - Search knowledge base
/// - `GET /api/search/ast` - Search AST definitions
/// - `GET /api/search/definition` - Resolve the best semantic definition hit
/// - `GET /api/search/references` - Search symbol references and usages
/// - `GET /api/search/symbols` - Search project symbols
/// - `GET /api/search/autocomplete` - Search autocomplete suggestions
/// - `GET/POST /api/ui/config` - UI configuration
pub fn studio_routes() -> Router<Arc<GatewayState>> {
    Router::new()
        .route("/api/vfs", get(vfs_root_entries))
        .route("/api/vfs/scan", get(vfs_scan))
        .route("/api/vfs/cat", get(vfs_cat))
        .route("/api/vfs/{*path}", get(vfs_entry))
        .route("/api/neighbors/{*id}", get(node_neighbors))
        .route("/api/graph/neighbors/{*id}", get(graph_neighbors))
        .route("/api/topology/3d", get(topology_3d))
        .route("/api/search", get(search::search_knowledge))
        .route("/api/search/ast", get(search::search_ast))
        .route("/api/search/definition", get(search::search_definition))
        .route("/api/search/references", get(search::search_references))
        .route("/api/search/symbols", get(search::search_symbols))
        .route("/api/search/autocomplete", get(search::search_autocomplete))
        .route("/api/ui/config", get(get_ui_config).post(set_ui_config))
}

/// Create the Studio API router with state already attached.
pub fn studio_router(state: Arc<GatewayState>) -> Router {
    studio_routes().with_state(state)
}

async fn vfs_root_entries(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<Vec<VfsEntry>>, StudioApiError> {
    let entries = vfs::list_root_entries(state.studio.as_ref());
    Ok(Json(entries))
}

async fn vfs_scan(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<VfsScanResult>, StudioApiError> {
    let result = vfs::scan_roots(state.studio.as_ref());
    Ok(Json(result))
}

async fn vfs_entry(
    AxumPath(path): AxumPath<String>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<VfsEntry>, StudioApiError> {
    let entry = vfs::get_entry(state.studio.as_ref(), path.as_str())?;
    Ok(Json(entry))
}

async fn vfs_cat(
    Query(query): Query<VfsCatQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<VfsContentResponse>, StudioApiError> {
    let path = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PATH", "`path` is required"))?;
    let payload = vfs::read_content(state.studio.as_ref(), path).await?;
    Ok(Json(payload))
}

async fn node_neighbors(
    AxumPath(id): AxumPath<String>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<NodeNeighbors>, StudioApiError> {
    let payload = graph::node_neighbors(state.as_ref(), id.as_str()).await?;
    Ok(Json(payload))
}

async fn graph_neighbors(
    AxumPath(id): AxumPath<String>,
    Query(query): Query<GraphNeighborsQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<GraphNeighborsResponse>, StudioApiError> {
    let direction = query.direction.unwrap_or_else(|| "both".to_string());
    let hops = query.hops.unwrap_or(2).clamp(1, 5);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let payload =
        graph::graph_neighbors(state.as_ref(), id.as_str(), direction.as_str(), hops, limit)
            .await?;
    Ok(Json(payload))
}

async fn topology_3d(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<super::types::Topology3D>, StudioApiError> {
    let payload = graph::topology_3d(state.as_ref()).await?;
    Ok(Json(payload))
}

async fn get_ui_config(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<UiConfig>, StudioApiError> {
    Ok(Json(state.studio.ui_config()))
}

async fn set_ui_config(
    State(state): State<Arc<GatewayState>>,
    Json(config): Json<UiConfig>,
) -> Result<Json<UiConfig>, StudioApiError> {
    state.studio.set_ui_config(config);
    Ok(Json(state.studio.ui_config()))
}

fn sanitize_projects(raw: Vec<UiProjectConfig>) -> Vec<UiProjectConfig> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for project in raw {
        let name = project.name.trim();
        if name.is_empty() {
            continue;
        }
        if !seen.insert(name.to_string()) {
            continue;
        }

        let Some(root) = sanitize_path_like(project.root.as_str()) else {
            continue;
        };

        out.push(UiProjectConfig {
            name: name.to_string(),
            root,
            paths: sanitize_path_list(project.paths),
            watch_patterns: sanitize_pattern_list(project.watch_patterns),
            include_dirs_auto: project.include_dirs_auto,
            include_dirs_auto_candidates: sanitize_path_list(project.include_dirs_auto_candidates),
        });
    }
    out
}

fn sanitize_path_list(raw: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in raw {
        let Some(normalized) = sanitize_path_like(path.as_str()) else {
            continue;
        };
        if seen.insert(normalized.clone()) {
            out.push(normalized);
        }
    }
    out
}

fn sanitize_pattern_list(raw: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for pattern in raw {
        let trimmed = pattern.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = trimmed.replace('\\', "/");
        if seen.insert(normalized.clone()) {
            out.push(normalized);
        }
    }
    out
}

fn sanitize_path_like(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed == "." {
        return Some(".".to_string());
    }
    let normalized = trimmed
        .replace('\\', "/")
        .trim_end_matches('/')
        .trim_start_matches("./")
        .to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn graph_include_dirs(project_root: &Path, projects: &[UiProjectConfig]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut include_dirs = Vec::new();

    for project in projects {
        let project_base = join_relative_path(project_root, project.root.as_str());
        for path in &project.paths {
            let candidate = join_relative_path(project_base.as_path(), path.as_str());
            let Ok(relative) = candidate.strip_prefix(project_root) else {
                continue;
            };
            let normalized = relative
                .to_string_lossy()
                .replace('\\', "/")
                .trim_end_matches('/')
                .to_string();
            let value = if normalized.is_empty() {
                ".".to_string()
            } else {
                normalized
            };
            if seen.insert(value.clone()) {
                include_dirs.push(value);
            }
        }
    }

    include_dirs
}

fn join_relative_path(base: &Path, raw: &str) -> PathBuf {
    if raw == "." {
        return base.to_path_buf();
    }
    base.join(raw.replace('\\', "/"))
}

#[derive(Debug)]
pub(crate) struct StudioApiError {
    status: StatusCode,
    error: ApiError,
}

impl StudioApiError {
    #[cfg(test)]
    pub(crate) fn status(&self) -> StatusCode {
        self.status
    }

    #[cfg(test)]
    pub(crate) fn code(&self) -> &str {
        self.error.code.as_str()
    }

    pub(crate) fn bad_request(code: &str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: ApiError {
                code: code.to_string(),
                message: message.into(),
                details: None,
            },
        }
    }

    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error: ApiError {
                code: "NOT_FOUND".to_string(),
                message: message.into(),
                details: None,
            },
        }
    }

    pub(crate) fn internal(
        code: &str,
        message: impl Into<String>,
        details: Option<String>,
    ) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: ApiError {
                code: code.to_string(),
                message: message.into(),
                details,
            },
        }
    }
}

impl IntoResponse for StudioApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self.error)).into_response()
    }
}

impl From<vfs::VfsError> for StudioApiError {
    fn from(error: vfs::VfsError) -> Self {
        match error {
            vfs::VfsError::NotFound(path) => Self::not_found(format!("Path not found: {path}")),
            vfs::VfsError::UnknownRoot(root) => {
                Self::bad_request("UNKNOWN_ROOT", format!("Unknown VFS root: {root}"))
            }
            vfs::VfsError::Io(e) => {
                Self::internal("IO_ERROR", "IO error occurred", Some(e.to_string()))
            }
        }
    }
}
