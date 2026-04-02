use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::MarkdownAnalysisResponse;

pub(crate) async fn load_markdown_analysis_response(
    state: &GatewayState,
    path: &str,
) -> Result<MarkdownAnalysisResponse, StudioApiError> {
    crate::gateway::studio::analysis::analyze_markdown(state.studio.as_ref(), path)
        .await
        .map_err(|error| {
            StudioApiError::internal("MARKDOWN_ANALYSIS_FAILED", error.to_string(), None)
        })
}
