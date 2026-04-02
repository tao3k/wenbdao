use std::sync::Arc;

use axum::{Json, extract::State};

use crate::analyzers::{RefineEntityDocRequest, RefineEntityDocResponse};
use crate::gateway::studio::router::handlers::repo::command_service::run_refine_entity_doc;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Refine documentation for a specific entity using the Trinity loop.
///
/// # Errors
///
/// Returns an error when the requested repository cannot be resolved, analysis
/// fails, the target entity cannot be found, or the background task panics.
pub async fn refine_entity_doc(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<RefineEntityDocRequest>,
) -> Result<Json<RefineEntityDocResponse>, StudioApiError> {
    let result = run_refine_entity_doc(Arc::clone(&state), payload).await?;
    Ok(Json(result))
}
