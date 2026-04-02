use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::config::UiPluginArtifact;
use crate::link_graph::plugin_runtime::{
    render_plugin_artifact_toml_for_selector, resolve_plugin_artifact_for_selector,
};
use crate::zhenfa_router::native::WendaoPluginArtifactOutputFormat;
use xiuxian_wendao_core::artifacts::PluginArtifactSelector;

use crate::gateway::studio::router::handlers::capabilities::types::{
    PluginArtifactPath, PluginArtifactQuery,
};

fn render_plugin_artifact_json_response(
    selector: &PluginArtifactSelector,
) -> Result<Response, StudioApiError> {
    let artifact = resolve_plugin_artifact_for_selector(selector).ok_or_else(|| {
        StudioApiError::internal(
            "PLUGIN_ARTIFACT_RESOLVE_FAILED",
            "Failed to resolve plugin artifact",
            None,
        )
    })?;

    Ok(Json(UiPluginArtifact::from(artifact)).into_response())
}

fn render_plugin_artifact_toml_response(
    selector: &PluginArtifactSelector,
) -> Result<Response, StudioApiError> {
    let body = render_plugin_artifact_toml_for_selector(selector)
        .map_err(|error| {
            StudioApiError::internal(
                "PLUGIN_ARTIFACT_EXPORT_FAILED",
                "Failed to render plugin artifact as TOML",
                Some(error.to_string()),
            )
        })?
        .ok_or_else(|| {
            StudioApiError::internal(
                "PLUGIN_ARTIFACT_EXPORT_FAILED",
                "Failed to render plugin artifact as TOML",
                None,
            )
        })?;

    Ok((
        StatusCode::OK,
        [(
            CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )],
        body,
    )
        .into_response())
}

/// Read the currently resolved generic plugin artifact used by runtime config.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn get_plugin_artifact(
    State(_state): State<Arc<GatewayState>>,
    Path(path): Path<PluginArtifactPath>,
    Query(query): Query<PluginArtifactQuery>,
) -> Result<Response, StudioApiError> {
    let selector = PluginArtifactSelector::from(path);

    match query
        .format
        .unwrap_or(WendaoPluginArtifactOutputFormat::Json)
    {
        WendaoPluginArtifactOutputFormat::Json => render_plugin_artifact_json_response(&selector),
        WendaoPluginArtifactOutputFormat::Toml => render_plugin_artifact_toml_response(&selector),
    }
}

#[cfg(test)]
mod tests {
    use crate::gateway::studio::router::handlers::capabilities::types::{
        PluginArtifactPath, PluginArtifactQuery,
    };
    use crate::gateway::studio::router::{GatewayState, StudioState};
    use crate::gateway::studio::types::config::UiPluginArtifact;
    use crate::set_link_graph_wendao_config_override;
    use crate::zhenfa_router::native::WendaoPluginArtifactOutputFormat;
    use axum::body::to_bytes;
    use axum::extract::{Path, Query, State};
    use serial_test::serial;
    use std::fs;
    use std::sync::Arc;

    #[tokio::test]
    #[serial]
    async fn generic_plugin_artifact_handler_returns_plugin_artifact() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:18080"
route = "/rerank"
schema_version = "v1"
service_mode = "stream"
"#,
        )
        .unwrap_or_else(|error| panic!("write config: {error}"));
        let config_path_string = config_path.to_string_lossy().to_string();
        set_link_graph_wendao_config_override(&config_path_string);

        let state = Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(StudioState::new()),
        });

        let response = super::get_plugin_artifact(
            State(Arc::clone(&state)),
            Path(PluginArtifactPath {
                plugin_id: "xiuxian-wendao-julia".to_string(),
                artifact_id: "deployment".to_string(),
            }),
            Query(PluginArtifactQuery {
                format: Some(WendaoPluginArtifactOutputFormat::Json),
            }),
        )
        .await
        .unwrap_or_else(|error| {
            panic!("generic deployment artifact handler should resolve: {error:?}")
        });

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("read json body: {error}"));
        let artifact: UiPluginArtifact = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("decode artifact json: {error}"));

        assert_eq!(artifact.plugin_id, "xiuxian-wendao-julia");
        assert_eq!(artifact.artifact_id, "deployment");
        assert_eq!(artifact.schema_version.as_deref(), Some("v1"));
        assert_eq!(artifact.route.as_deref(), Some("/rerank"));
    }
}
