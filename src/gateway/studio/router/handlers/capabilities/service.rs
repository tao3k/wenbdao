use std::sync::Arc;

use axum::{Json, extract::State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::UiCapabilities;

/// Read the gateway-reported studio capabilities.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn get(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<UiCapabilities>, StudioApiError> {
    Ok(Json(state.studio.ui_capabilities()))
}
