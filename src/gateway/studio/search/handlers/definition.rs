use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceInt32Array, LanceRecordBatch, LanceSchema,
    LanceStringArray,
};
use xiuxian_wendao_runtime::transport::{
    DefinitionFlightRouteProvider, DefinitionFlightRouteResponse,
};

use crate::gateway::studio::router::{StudioApiError, StudioState};
use crate::gateway::studio::search::definition::resolve::resolve_definition_candidates;
use crate::gateway::studio::search::definition::{
    DefinitionResolveOptions, resolve_best_definition,
};
use crate::gateway::studio::search::observation_hints::definition_observation_hints;
use crate::gateway::studio::types::{DefinitionResolveResponse, DefinitionSearchHit};

/// Studio-backed Flight provider for the semantic `/search/definition` route.
#[derive(Clone)]
pub(crate) struct StudioDefinitionFlightRouteProvider {
    studio: Arc<StudioState>,
}

impl StudioDefinitionFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(studio: Arc<StudioState>) -> Self {
        Self { studio }
    }
}

impl std::fmt::Debug for StudioDefinitionFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioDefinitionFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl DefinitionFlightRouteProvider for StudioDefinitionFlightRouteProvider {
    async fn definition_batch(
        &self,
        query_text: &str,
        source_path: Option<&str>,
        source_line: Option<usize>,
    ) -> Result<DefinitionFlightRouteResponse, Status> {
        load_definition_flight_response(
            Arc::clone(&self.studio),
            query_text,
            source_path,
            source_line,
        )
        .await
        .map_err(studio_api_error_to_tonic_status)
    }
}

pub(crate) async fn build_definition_response(
    studio: &StudioState,
    query_text: &str,
    source_path: Option<&str>,
    source_line: Option<usize>,
) -> Result<DefinitionResolveResponse, StudioApiError> {
    let query_text = query_text.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Definition search requires a non-empty query",
        ));
    }

    let normalized_source_path =
        source_path.map(|path| normalize_source_path(studio.project_root.as_path(), path));
    let source_paths = normalized_source_path
        .as_ref()
        .map(std::slice::from_ref)
        .filter(|paths| !paths.is_empty());
    let observation_hints =
        definition_observation_hints(studio, source_paths, source_line, query_text).await;
    studio.ensure_local_symbol_index_ready().await?;
    let ast_hits = studio.search_local_symbol_hits(query_text, 256).await?;
    let projects = studio.configured_projects();
    let options = DefinitionResolveOptions {
        scope_patterns: observation_hints.as_ref().and_then(|hints| {
            (!hints.scope_patterns.is_empty()).then_some(hints.scope_patterns.clone())
        }),
        languages: observation_hints
            .as_ref()
            .and_then(|hints| (!hints.languages.is_empty()).then_some(hints.languages.clone())),
        preferred_source_path: normalized_source_path.clone(),
        ..DefinitionResolveOptions::default()
    };
    let candidates = resolve_definition_candidates(
        query_text,
        ast_hits.as_slice(),
        studio.project_root.as_path(),
        studio.config_root.as_path(),
        projects.as_slice(),
        &options,
    );
    let Some(definition) = resolve_best_definition(
        query_text,
        ast_hits.as_slice(),
        studio.project_root.as_path(),
        studio.config_root.as_path(),
        projects.as_slice(),
        &options,
    ) else {
        return Err(StudioApiError::not_found("Definition not found"));
    };
    let navigation_target = definition.navigation_target.clone();

    Ok(DefinitionResolveResponse {
        query: query_text.to_string(),
        source_path: normalized_source_path,
        source_line,
        candidate_count: candidates.len(),
        selected_scope: "definition".to_string(),
        navigation_target: navigation_target.clone(),
        definition: definition.clone(),
        resolved_target: Some(navigation_target),
        resolved_hit: Some(definition),
    })
}

pub(crate) async fn load_definition_flight_response(
    studio: Arc<StudioState>,
    query_text: &str,
    source_path: Option<&str>,
    source_line: Option<usize>,
) -> Result<DefinitionFlightRouteResponse, StudioApiError> {
    let response =
        build_definition_response(studio.as_ref(), query_text, source_path, source_line).await?;
    let batch = definition_hit_batch(&response.definition).map_err(|error| {
        StudioApiError::internal(
            "SEARCH_DEFINITION_FLIGHT_BATCH_FAILED",
            "Failed to materialize definition result through the Flight-backed provider",
            Some(error),
        )
    })?;
    let app_metadata = definition_response_flight_app_metadata(&response).map_err(|error| {
        StudioApiError::internal(
            "SEARCH_DEFINITION_FLIGHT_METADATA_FAILED",
            "Failed to encode definition Flight app metadata",
            Some(error),
        )
    })?;
    Ok(DefinitionFlightRouteResponse::new(batch).with_app_metadata(app_metadata))
}

pub(crate) fn definition_hit_batch(hit: &DefinitionSearchHit) -> Result<LanceRecordBatch, String> {
    let observation_hints_json = serde_json::to_string(&hit.observation_hints)
        .map_err(|error| format!("failed to encode definition observation hints: {error}"))?;
    let navigation_target_json = serde_json::to_string(&hit.navigation_target)
        .map_err(|error| format!("failed to encode definition navigation target: {error}"))?;

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("name", LanceDataType::Utf8, false),
            LanceField::new("signature", LanceDataType::Utf8, false),
            LanceField::new("path", LanceDataType::Utf8, false),
            LanceField::new("language", LanceDataType::Utf8, false),
            LanceField::new("crateName", LanceDataType::Utf8, false),
            LanceField::new("projectName", LanceDataType::Utf8, true),
            LanceField::new("rootLabel", LanceDataType::Utf8, true),
            LanceField::new("nodeKind", LanceDataType::Utf8, true),
            LanceField::new("ownerTitle", LanceDataType::Utf8, true),
            LanceField::new("navigationTargetJson", LanceDataType::Utf8, false),
            LanceField::new("lineStart", LanceDataType::Int32, false),
            LanceField::new("lineEnd", LanceDataType::Int32, false),
            LanceField::new("score", LanceDataType::Float64, false),
            LanceField::new("observationHintsJson", LanceDataType::Utf8, false),
        ])),
        vec![
            Arc::new(LanceStringArray::from(vec![hit.name.as_str()])),
            Arc::new(LanceStringArray::from(vec![hit.signature.as_str()])),
            Arc::new(LanceStringArray::from(vec![hit.path.as_str()])),
            Arc::new(LanceStringArray::from(vec![hit.language.as_str()])),
            Arc::new(LanceStringArray::from(vec![hit.crate_name.as_str()])),
            Arc::new(LanceStringArray::from(vec![hit.project_name.as_deref()])),
            Arc::new(LanceStringArray::from(vec![hit.root_label.as_deref()])),
            Arc::new(LanceStringArray::from(vec![hit.node_kind.as_deref()])),
            Arc::new(LanceStringArray::from(vec![hit.owner_title.as_deref()])),
            Arc::new(LanceStringArray::from(vec![
                navigation_target_json.as_str(),
            ])),
            Arc::new(LanceInt32Array::from(vec![
                i32::try_from(hit.line_start).map_err(|error| {
                    format!("failed to represent definition line_start: {error}")
                })?,
            ])),
            Arc::new(LanceInt32Array::from(vec![
                i32::try_from(hit.line_end)
                    .map_err(|error| format!("failed to represent definition line_end: {error}"))?,
            ])),
            Arc::new(LanceFloat64Array::from(vec![hit.score])),
            Arc::new(LanceStringArray::from(vec![
                observation_hints_json.as_str(),
            ])),
        ],
    )
    .map_err(|error| format!("failed to build definition Flight batch: {error}"))
}

pub(crate) fn definition_response_flight_app_metadata(
    response: &DefinitionResolveResponse,
) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&serde_json::json!({
        "query": response.query,
        "sourcePath": response.source_path,
        "sourceLine": response.source_line,
        "candidateCount": response.candidate_count,
        "selectedScope": response.selected_scope,
        "navigationTarget": response.navigation_target,
        "resolvedTarget": response.resolved_target,
    }))
    .map_err(|error| format!("failed to encode definition Flight app metadata: {error}"))
}

fn normalize_source_path(project_root: &Path, path: &str) -> String {
    let path = Path::new(path);
    if path.is_absolute() {
        return path.strip_prefix(project_root).map_or_else(
            |_| path.to_string_lossy().replace('\\', "/"),
            |relative| relative.to_string_lossy().replace('\\', "/"),
        );
    }

    path.to_string_lossy().replace('\\', "/")
}

fn studio_api_error_to_tonic_status(error: StudioApiError) -> Status {
    match error.status() {
        axum::http::StatusCode::BAD_REQUEST => Status::invalid_argument(error.error.message),
        axum::http::StatusCode::NOT_FOUND => Status::not_found(error.error.message),
        axum::http::StatusCode::CONFLICT => Status::failed_precondition(error.error.message),
        _ => Status::internal(error.error.message),
    }
}
