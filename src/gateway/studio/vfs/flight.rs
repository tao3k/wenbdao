use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceInt32Array, LanceRecordBatch, LanceSchema, LanceStringArray,
};
use xiuxian_wendao_runtime::transport::{
    VfsResolveFlightRouteProvider, VfsResolveFlightRouteResponse,
};

use crate::gateway::studio::router::{StudioApiError, StudioState};
use crate::gateway::studio::types::StudioNavigationTarget;

use super::resolve_navigation_target;

/// Studio-backed Flight provider for the semantic `/vfs/resolve` route.
#[derive(Clone)]
pub(crate) struct StudioVfsResolveFlightRouteProvider {
    studio: Arc<StudioState>,
}

impl StudioVfsResolveFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(studio: Arc<StudioState>) -> Self {
        Self { studio }
    }
}

impl std::fmt::Debug for StudioVfsResolveFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioVfsResolveFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl VfsResolveFlightRouteProvider for StudioVfsResolveFlightRouteProvider {
    async fn resolve_vfs_navigation_batch(
        &self,
        path: &str,
    ) -> Result<VfsResolveFlightRouteResponse, Status> {
        load_vfs_resolve_flight_response(Arc::clone(&self.studio), path)
            .await
            .map_err(studio_api_error_to_tonic_status)
    }
}

pub(crate) async fn build_vfs_resolve_response(
    studio: &StudioState,
    path: &str,
) -> Result<StudioNavigationTarget, StudioApiError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_PATH",
            "`path` is required",
        ));
    }
    Ok(resolve_navigation_target(studio, path))
}

pub(crate) async fn load_vfs_resolve_flight_response(
    studio: Arc<StudioState>,
    path: &str,
) -> Result<VfsResolveFlightRouteResponse, StudioApiError> {
    let response = build_vfs_resolve_response(studio.as_ref(), path).await?;
    let batch = vfs_navigation_target_batch(&response).map_err(|error| {
        StudioApiError::internal(
            "VFS_RESOLVE_FLIGHT_BATCH_FAILED",
            "Failed to materialize VFS navigation target through the Flight-backed provider",
            Some(error),
        )
    })?;
    let app_metadata =
        vfs_resolve_response_flight_app_metadata(path, &response).map_err(|error| {
            StudioApiError::internal(
                "VFS_RESOLVE_FLIGHT_METADATA_FAILED",
                "Failed to encode VFS resolve Flight app metadata",
                Some(error),
            )
        })?;
    Ok(VfsResolveFlightRouteResponse::new(batch).with_app_metadata(app_metadata))
}

pub(crate) fn vfs_navigation_target_batch(
    target: &StudioNavigationTarget,
) -> Result<LanceRecordBatch, String> {
    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("path", LanceDataType::Utf8, false),
            LanceField::new("category", LanceDataType::Utf8, false),
            LanceField::new("projectName", LanceDataType::Utf8, true),
            LanceField::new("rootLabel", LanceDataType::Utf8, true),
            LanceField::new("line", LanceDataType::Int32, true),
            LanceField::new("lineEnd", LanceDataType::Int32, true),
            LanceField::new("column", LanceDataType::Int32, true),
        ])),
        vec![
            Arc::new(LanceStringArray::from(vec![target.path.as_str()])),
            Arc::new(LanceStringArray::from(vec![target.category.as_str()])),
            Arc::new(LanceStringArray::from(vec![target.project_name.as_deref()])),
            Arc::new(LanceStringArray::from(vec![target.root_label.as_deref()])),
            Arc::new(LanceInt32Array::from(vec![
                target.line.map(line_to_i32).transpose()?,
            ])),
            Arc::new(LanceInt32Array::from(vec![
                target.line_end.map(line_to_i32).transpose()?,
            ])),
            Arc::new(LanceInt32Array::from(vec![
                target.column.map(line_to_i32).transpose()?,
            ])),
        ],
    )
    .map_err(|error| format!("failed to build VFS resolve Flight batch: {error}"))
}

pub(crate) fn vfs_resolve_response_flight_app_metadata(
    requested_path: &str,
    target: &StudioNavigationTarget,
) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&serde_json::json!({
        "path": requested_path.trim(),
        "navigationTarget": target,
    }))
    .map_err(|error| format!("failed to encode VFS resolve Flight app metadata: {error}"))
}

fn line_to_i32(value: usize) -> Result<i32, String> {
    i32::try_from(value)
        .map_err(|error| format!("failed to represent VFS navigation position: {error}"))
}

fn studio_api_error_to_tonic_status(error: StudioApiError) -> Status {
    match error.status() {
        axum::http::StatusCode::BAD_REQUEST => Status::invalid_argument(error.error.message),
        axum::http::StatusCode::NOT_FOUND => Status::not_found(error.error.message),
        axum::http::StatusCode::CONFLICT => Status::failed_precondition(error.error.message),
        _ => Status::internal(error.error.message),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_vfs_resolve_response, vfs_navigation_target_batch};
    use crate::gateway::studio::router::StudioState;
    use crate::gateway::studio::types::{UiConfig, UiProjectConfig};

    #[tokio::test]
    async fn build_vfs_resolve_response_prefixes_project_for_relative_docs_path() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let mut state = StudioState::new();
        state.project_root = temp_dir.path().to_path_buf();
        state.config_root = temp_dir.path().to_path_buf();
        state.set_ui_config(UiConfig {
            projects: vec![UiProjectConfig {
                name: "main".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            }],
            repo_projects: Vec::new(),
        });

        let target = build_vfs_resolve_response(&state, "docs/index.md")
            .await
            .unwrap_or_else(|error| panic!("build VFS resolve response: {error:?}"));

        assert_eq!(target.path, "main/docs/index.md");
        assert_eq!(target.project_name.as_deref(), Some("main"));
    }

    #[tokio::test]
    async fn build_vfs_resolve_response_rejects_blank_path() {
        let error = build_vfs_resolve_response(&StudioState::new(), "   ")
            .await
            .expect_err("blank path should fail");
        assert_eq!(error.error.code, "MISSING_PATH");
    }

    #[test]
    fn vfs_navigation_target_batch_preserves_project_metadata() {
        let batch =
            vfs_navigation_target_batch(&crate::gateway::studio::types::StudioNavigationTarget {
                path: "kernel/docs/index.md".to_string(),
                category: "file".to_string(),
                project_name: Some("kernel".to_string()),
                root_label: Some("project".to_string()),
                line: Some(7),
                line_end: Some(9),
                column: Some(3),
            })
            .unwrap_or_else(|error| panic!("build VFS navigation batch: {error}"));
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 7);
    }
}
