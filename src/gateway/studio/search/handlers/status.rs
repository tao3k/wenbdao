use std::sync::Arc;

use axum::{Json, extract::State};
use serde::Serialize;

use crate::gateway::studio::router::{
    GatewayState, StudioApiError, StudioBootstrapBackgroundIndexingTelemetry,
};
use crate::gateway::studio::types::SearchIndexStatusResponse;

/// Search-index status payload enriched with bootstrap-indexing telemetry.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexStatusEnvelope {
    #[serde(flatten)]
    status: SearchIndexStatusResponse,
    #[serde(flatten)]
    telemetry: StudioBootstrapBackgroundIndexingTelemetry,
}

/// Studio search-plane status endpoint.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn search_index_status(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<SearchIndexStatusEnvelope>, StudioApiError> {
    let telemetry = state.studio.bootstrap_background_indexing_telemetry();
    Ok(Json(SearchIndexStatusEnvelope {
        status: state.studio.search_index_status().await,
        telemetry,
    }))
}
