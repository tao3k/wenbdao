use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use tokio::sync::{OnceCell, RwLock};
use xiuxian_io::PrjDirs;

use crate::link_graph::LinkGraphIndex;
use crate::skill_vfs::SkillVfsResolver;

use super::types::{
    ApiError, GraphNeighborsResponse, NodeNeighbors, UiConfig, VfsContentResponse, VfsEntry,
    VfsScanResult,
};
use super::{graph, vfs};

/// Shared state for the Studio API.
///
/// Contains configuration, VFS roots, and cached graph index.
pub struct StudioState {
    pub(crate) project_root: PathBuf,
    pub(crate) data_root: PathBuf,
    pub(crate) knowledge_root: PathBuf,
    pub(crate) internal_skill_root: PathBuf,
    pub(crate) ui_config: Arc<RwLock<UiConfig>>,
    pub(crate) graph_index: OnceCell<Arc<LinkGraphIndex>>,
}

impl StudioState {
    /// Create a new StudioState with default configuration.
    #[must_use]
    pub fn new() -> Self {
        let project_root = PrjDirs::project_root();
        let data_root = PrjDirs::data_home();
        let knowledge_root = data_root.join("knowledge");
        let internal_skill_root = SkillVfsResolver::resolve_runtime_internal_root_with(
            project_root.as_path(),
            std::env::var("PRJ_INTERNAL_SKILLS_DIR").ok().as_deref(),
        );
        Self {
            project_root,
            data_root,
            knowledge_root,
            internal_skill_root,
            ui_config: Arc::new(RwLock::new(UiConfig {
                index_paths: Vec::new(),
            })),
            graph_index: OnceCell::new(),
        }
    }

    pub(crate) fn ui_config(&self) -> UiConfig {
        self.ui_config.blocking_read().clone()
    }

    pub(crate) fn set_ui_config(&self, config: UiConfig) {
        let sanitized = sanitize_index_paths(config.index_paths);
        let mut guard = self.ui_config.blocking_write();
        guard.index_paths = sanitized;
    }

    pub(crate) async fn graph_index(&self) -> Result<Arc<LinkGraphIndex>, StudioApiError> {
        let knowledge_root = self.knowledge_root.clone();
        let index = self
            .graph_index
            .get_or_try_init(|| async move {
                let build = tokio::task::spawn_blocking(move || {
                    LinkGraphIndex::build(knowledge_root.as_path())
                })
                .await
                .map_err(|error| {
                    StudioApiError::internal(
                        "LINK_GRAPH_BUILD_PANIC",
                        "Failed to build link graph index",
                        Some(error.to_string()),
                    )
                })?;
                let index = build.map_err(|error| {
                    StudioApiError::internal(
                        "LINK_GRAPH_BUILD_FAILED",
                        "Failed to build link graph index",
                        Some(error),
                    )
                })?;
                Ok::<Arc<LinkGraphIndex>, StudioApiError>(Arc::new(index))
            })
            .await?;
        Ok(Arc::clone(index))
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
/// - `GET/POST /api/ui/config` - UI configuration
pub fn studio_router(state: Arc<StudioState>) -> Router {
    Router::new()
        .route("/api/vfs", get(vfs_root_entries))
        .route("/api/vfs/scan", get(vfs_scan))
        .route("/api/vfs/cat", get(vfs_cat))
        .route("/api/vfs/{*path}", get(vfs_entry))
        .route("/api/neighbors/{*id}", get(node_neighbors))
        .route("/api/graph/neighbors/{*id}", get(graph_neighbors))
        .route("/api/ui/config", get(get_ui_config).post(set_ui_config))
        .with_state(state)
}

async fn vfs_root_entries(
    State(state): State<Arc<StudioState>>,
) -> Result<Json<Vec<VfsEntry>>, StudioApiError> {
    let entries = vfs::list_root_entries(state.as_ref())?;
    Ok(Json(entries))
}

async fn vfs_scan(
    State(state): State<Arc<StudioState>>,
) -> Result<Json<VfsScanResult>, StudioApiError> {
    let result = vfs::scan_roots(state.as_ref())?;
    Ok(Json(result))
}

async fn vfs_entry(
    AxumPath(path): AxumPath<String>,
    State(state): State<Arc<StudioState>>,
) -> Result<Json<VfsEntry>, StudioApiError> {
    let entry = vfs::get_entry(state.as_ref(), path.as_str())?;
    Ok(Json(entry))
}

async fn vfs_cat(
    Query(query): Query<VfsCatQuery>,
    State(state): State<Arc<StudioState>>,
) -> Result<Json<VfsContentResponse>, StudioApiError> {
    let path = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PATH", "`path` is required"))?;
    let payload = vfs::read_content(state.as_ref(), path).await?;
    Ok(Json(payload))
}

async fn node_neighbors(
    AxumPath(id): AxumPath<String>,
    State(state): State<Arc<StudioState>>,
) -> Result<Json<NodeNeighbors>, StudioApiError> {
    let payload = graph::node_neighbors(state.as_ref(), id.as_str()).await?;
    Ok(Json(payload))
}

async fn graph_neighbors(
    AxumPath(id): AxumPath<String>,
    Query(query): Query<GraphNeighborsQuery>,
    State(state): State<Arc<StudioState>>,
) -> Result<Json<GraphNeighborsResponse>, StudioApiError> {
    let direction = query.direction.unwrap_or_else(|| "both".to_string());
    let hops = query.hops.unwrap_or(2).clamp(1, 5);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let payload =
        graph::graph_neighbors(state.as_ref(), id.as_str(), direction.as_str(), hops, limit)
            .await?;
    Ok(Json(payload))
}

async fn get_ui_config(
    State(state): State<Arc<StudioState>>,
) -> Result<Json<UiConfig>, StudioApiError> {
    Ok(Json(state.ui_config()))
}

async fn set_ui_config(
    State(state): State<Arc<StudioState>>,
    Json(config): Json<UiConfig>,
) -> Result<Json<UiConfig>, StudioApiError> {
    state.set_ui_config(config);
    Ok(Json(state.ui_config()))
}

fn sanitize_index_paths(raw: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in raw {
        let trimmed = path.trim().trim_matches('/');
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

#[derive(Debug)]
pub(crate) struct StudioApiError {
    status: StatusCode,
    error: ApiError,
}

impl StudioApiError {
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
