use std::sync::Arc;

use axum::Json;
use axum::extract::State;

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::Topology3dPayload;

use crate::gateway::studio::router::handlers::graph::service::run_topology_3d;

/// Gets 3D topology.
///
/// # Errors
///
/// Returns an error when the graph index cannot be loaded.
pub async fn topology_3d(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<Topology3dPayload>, StudioApiError> {
    Ok(Json(run_topology_3d(Arc::clone(&state)).await?))
}
