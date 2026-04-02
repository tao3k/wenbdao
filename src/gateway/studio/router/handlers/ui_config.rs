//! UI configuration endpoint handlers for Studio API.

use std::sync::Arc;

use axum::{Json, extract::State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::UiConfig;

/// Gets the current UI configuration.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn get(State(state): State<Arc<GatewayState>>) -> Result<Json<UiConfig>, StudioApiError> {
    Ok(Json(state.studio.ui_config()))
}

/// Sets and persists the UI configuration.
///
/// # Errors
///
/// Returns an error when persisting the updated configuration into
/// `wendao.toml` fails.
pub async fn set(
    State(state): State<Arc<GatewayState>>,
    Json(config_value): Json<UiConfig>,
) -> Result<Json<UiConfig>, StudioApiError> {
    state
        .studio
        .set_ui_config_and_persist(config_value)
        .map_err(|details| {
            StudioApiError::internal(
                "UI_CONFIG_PERSIST_FAILED",
                "Failed to persist UI config into wendao.toml",
                Some(details),
            )
        })?;
    Ok(Json(state.studio.ui_config()))
}
