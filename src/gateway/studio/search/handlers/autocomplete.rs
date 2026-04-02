use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;
use xiuxian_vector::{LanceDataType, LanceField, LanceRecordBatch, LanceSchema, LanceStringArray};
use xiuxian_wendao_runtime::transport::{
    AutocompleteFlightRouteProvider, AutocompleteFlightRouteResponse,
};

use crate::gateway::studio::router::{StudioApiError, StudioState};
use crate::gateway::studio::types::{AutocompleteResponse, AutocompleteSuggestion};
use crate::search_plane::SearchPlaneCacheTtl;

/// Studio-backed Flight provider for the semantic `/search/autocomplete` route.
#[derive(Clone)]
pub(crate) struct StudioAutocompleteFlightRouteProvider {
    studio: Arc<StudioState>,
}

impl StudioAutocompleteFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(studio: Arc<StudioState>) -> Self {
        Self { studio }
    }
}

impl std::fmt::Debug for StudioAutocompleteFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioAutocompleteFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl AutocompleteFlightRouteProvider for StudioAutocompleteFlightRouteProvider {
    async fn autocomplete_batch(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<AutocompleteFlightRouteResponse, Status> {
        load_autocomplete_flight_response(Arc::clone(&self.studio), prefix, limit)
            .await
            .map_err(studio_api_error_to_tonic_status)
    }
}

pub(crate) async fn build_autocomplete_response(
    studio: &StudioState,
    prefix: &str,
    limit: usize,
) -> Result<AutocompleteResponse, StudioApiError> {
    let prefix = prefix.trim().to_string();
    let limit = limit.max(1);
    let suggestions = if prefix.is_empty() {
        Vec::new()
    } else {
        studio.ensure_local_symbol_index_ready().await?;
        let cache_key = studio
            .search_plane
            .autocomplete_cache_key(prefix.as_str(), limit);
        if let Some(cache_key) = cache_key.as_ref()
            && let Some(cached) = studio
                .search_plane
                .cache_get_json::<Vec<AutocompleteSuggestion>>(cache_key)
                .await
        {
            return Ok(AutocompleteResponse {
                prefix,
                suggestions: cached,
            });
        }

        let suggestions = studio
            .autocomplete_local_symbols(prefix.as_str(), limit)
            .await?;
        if let Some(cache_key) = cache_key.as_ref() {
            studio
                .search_plane
                .cache_set_json(cache_key, SearchPlaneCacheTtl::Autocomplete, &suggestions)
                .await;
        }
        suggestions
    };

    Ok(AutocompleteResponse {
        prefix,
        suggestions,
    })
}

pub(crate) async fn load_autocomplete_flight_response(
    studio: Arc<StudioState>,
    prefix: &str,
    limit: usize,
) -> Result<AutocompleteFlightRouteResponse, StudioApiError> {
    let response = build_autocomplete_response(studio.as_ref(), prefix, limit).await?;
    let batch = autocomplete_suggestion_batch(&response.suggestions).map_err(|error| {
        StudioApiError::internal(
            "SEARCH_AUTOCOMPLETE_FLIGHT_BATCH_FAILED",
            "Failed to materialize autocomplete suggestions through the Flight-backed provider",
            Some(error),
        )
    })?;
    let app_metadata = autocomplete_response_flight_app_metadata(&response).map_err(|error| {
        StudioApiError::internal(
            "SEARCH_AUTOCOMPLETE_FLIGHT_METADATA_FAILED",
            "Failed to encode autocomplete Flight app metadata",
            Some(error),
        )
    })?;
    Ok(AutocompleteFlightRouteResponse::new(batch).with_app_metadata(app_metadata))
}

pub(crate) fn autocomplete_suggestion_batch(
    suggestions: &[AutocompleteSuggestion],
) -> Result<LanceRecordBatch, String> {
    let texts = suggestions
        .iter()
        .map(|suggestion| suggestion.text.as_str())
        .collect::<Vec<_>>();
    let suggestion_types = suggestions
        .iter()
        .map(|suggestion| suggestion.suggestion_type.as_str())
        .collect::<Vec<_>>();

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("text", LanceDataType::Utf8, false),
            LanceField::new("suggestionType", LanceDataType::Utf8, false),
        ])),
        vec![
            Arc::new(LanceStringArray::from(texts)),
            Arc::new(LanceStringArray::from(suggestion_types)),
        ],
    )
    .map_err(|error| format!("failed to build autocomplete Flight batch: {error}"))
}

pub(crate) fn autocomplete_response_flight_app_metadata(
    response: &AutocompleteResponse,
) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&serde_json::json!({
        "prefix": response.prefix,
    }))
    .map_err(|error| format!("failed to encode autocomplete Flight app metadata: {error}"))
}

fn studio_api_error_to_tonic_status(error: StudioApiError) -> Status {
    match error.status() {
        axum::http::StatusCode::BAD_REQUEST => Status::invalid_argument(error.error.message),
        axum::http::StatusCode::NOT_FOUND => Status::not_found(error.error.message),
        axum::http::StatusCode::CONFLICT => Status::failed_precondition(error.error.message),
        _ => Status::internal(error.error.message),
    }
}
