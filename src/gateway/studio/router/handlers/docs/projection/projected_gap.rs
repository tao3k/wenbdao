use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::DocsProjectedGapReportQuery;
use crate::gateway::studio::router::handlers::docs::service::run_docs_projected_gap_report;
use crate::gateway::studio::router::handlers::docs::types::DocsProjectedGapReportApiQuery;
use crate::gateway::studio::router::handlers::repo::required_repo_id;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Docs projected gap report endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup or analysis
/// fails, or the background task panics.
pub async fn projected_gap_report(
    Query(query): Query<DocsProjectedGapReportApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsProjectedGapReportResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result =
        run_docs_projected_gap_report(Arc::clone(&state), DocsProjectedGapReportQuery { repo_id })
            .await?;
    Ok(Json(result))
}
