use std::sync::Arc;

#[cfg(test)]
use async_trait::async_trait;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema, LanceStringArray,
};
use xiuxian_wendao_runtime::transport::SearchFlightRouteResponse;
#[cfg(test)]
use xiuxian_wendao_runtime::transport::{SEARCH_INTENT_ROUTE, SearchFlightRouteProvider};

use super::entry::build_intent_search_response_with_metadata;
use crate::gateway::studio::router::{StudioApiError, StudioState};
use crate::gateway::studio::types::{SearchHit, SearchResponse};

/// Studio-backed Flight provider for the semantic `/search/intent` route.
#[derive(Clone)]
#[cfg(test)]
pub struct StudioIntentSearchFlightRouteProvider {
    studio: Arc<StudioState>,
}

#[cfg(test)]
impl std::fmt::Debug for StudioIntentSearchFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioIntentSearchFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
impl StudioIntentSearchFlightRouteProvider {
    /// Create one Studio-backed intent-search Flight provider.
    #[must_use]
    pub fn new(studio: Arc<StudioState>) -> Self {
        Self { studio }
    }
}

#[async_trait]
#[cfg(test)]
impl SearchFlightRouteProvider for StudioIntentSearchFlightRouteProvider {
    async fn search_batch(
        &self,
        route: &str,
        query_text: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Result<SearchFlightRouteResponse, String> {
        if route != SEARCH_INTENT_ROUTE {
            return Err(format!(
                "studio intent Flight provider only supports route `{SEARCH_INTENT_ROUTE}`, got `{route}`"
            ));
        }

        let (response, _transport_metadata) = build_intent_search_response_with_metadata(
            self.studio.as_ref(),
            query_text,
            query_text,
            repo_hint,
            limit,
            intent.map(ToString::to_string),
        )
        .await
        .map_err(|error| {
            format!(
                "studio intent Flight provider failed to build search response for `{query_text}`: {error:?}"
            )
        })?;

        let batch = search_hit_batch_from_hits(&response.hits)?;
        let app_metadata = search_response_flight_app_metadata(&response)?;
        Ok(SearchFlightRouteResponse::new(batch).with_app_metadata(app_metadata))
    }
}

pub(crate) async fn load_intent_search_flight_response(
    studio: Arc<StudioState>,
    raw_query: &str,
    query_text: &str,
    repo_hint: Option<&str>,
    limit: usize,
    intent: Option<String>,
) -> Result<SearchFlightRouteResponse, StudioApiError> {
    let (response, _transport_metadata) = build_intent_search_response_with_metadata(
        studio.as_ref(),
        raw_query,
        query_text,
        repo_hint,
        limit,
        intent,
    )
    .await?;
    let batch = search_hit_batch_from_hits(&response.hits).map_err(|error| {
        StudioApiError::internal(
            "SEARCH_INTENT_FLIGHT_BATCH_FAILED",
            "Failed to materialize intent hits through the Flight-backed provider",
            Some(error),
        )
    })?;
    let app_metadata = search_response_flight_app_metadata(&response).map_err(|error| {
        StudioApiError::internal(
            "SEARCH_INTENT_FLIGHT_METADATA_FAILED",
            "Failed to encode intent Flight app metadata",
            Some(error),
        )
    })?;
    Ok(SearchFlightRouteResponse::new(batch).with_app_metadata(app_metadata))
}

pub(crate) fn search_hit_batch_from_hits(hits: &[SearchHit]) -> Result<LanceRecordBatch, String> {
    let stems = hits.iter().map(|hit| hit.stem.as_str()).collect::<Vec<_>>();
    let titles = hits
        .iter()
        .map(|hit| hit.title.as_deref())
        .collect::<Vec<_>>();
    let paths = hits.iter().map(|hit| hit.path.as_str()).collect::<Vec<_>>();
    let doc_types = hits
        .iter()
        .map(|hit| hit.doc_type.as_deref())
        .collect::<Vec<_>>();
    let tags_json = hits
        .iter()
        .map(|hit| serde_json::to_string(&hit.tags).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let best_sections = hits
        .iter()
        .map(|hit| hit.best_section.as_deref())
        .collect::<Vec<_>>();
    let match_reasons = hits
        .iter()
        .map(|hit| hit.match_reason.as_deref())
        .collect::<Vec<_>>();
    let hierarchical_uris = hits
        .iter()
        .map(|hit| hit.hierarchical_uri.as_deref())
        .collect::<Vec<_>>();
    let hierarchy_json = hits
        .iter()
        .map(|hit| {
            hit.hierarchy
                .as_ref()
                .map(|value| serde_json::to_string(value).map_err(|error| error.to_string()))
                .transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let saliency_scores = hits
        .iter()
        .map(|hit| hit.saliency_score)
        .collect::<Vec<_>>();
    let audit_statuses = hits
        .iter()
        .map(|hit| hit.audit_status.as_deref())
        .collect::<Vec<_>>();
    let verification_states = hits
        .iter()
        .map(|hit| hit.verification_state.as_deref())
        .collect::<Vec<_>>();
    let implicit_backlinks_json = hits
        .iter()
        .map(|hit| {
            hit.implicit_backlinks
                .as_ref()
                .map(|value| serde_json::to_string(value).map_err(|error| error.to_string()))
                .transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let implicit_backlink_items_json = hits
        .iter()
        .map(|hit| {
            hit.implicit_backlink_items
                .as_ref()
                .map(|value| serde_json::to_string(value).map_err(|error| error.to_string()))
                .transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let navigation_target_json = hits
        .iter()
        .map(|hit| {
            hit.navigation_target
                .as_ref()
                .map(|value| serde_json::to_string(value).map_err(|error| error.to_string()))
                .transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("stem", LanceDataType::Utf8, false),
            LanceField::new("title", LanceDataType::Utf8, true),
            LanceField::new("path", LanceDataType::Utf8, false),
            LanceField::new("docType", LanceDataType::Utf8, true),
            LanceField::new("tagsJson", LanceDataType::Utf8, false),
            LanceField::new("score", LanceDataType::Float64, false),
            LanceField::new("bestSection", LanceDataType::Utf8, true),
            LanceField::new("matchReason", LanceDataType::Utf8, true),
            LanceField::new("hierarchicalUri", LanceDataType::Utf8, true),
            LanceField::new("hierarchyJson", LanceDataType::Utf8, true),
            LanceField::new("saliencyScore", LanceDataType::Float64, true),
            LanceField::new("auditStatus", LanceDataType::Utf8, true),
            LanceField::new("verificationState", LanceDataType::Utf8, true),
            LanceField::new("implicitBacklinksJson", LanceDataType::Utf8, true),
            LanceField::new("implicitBacklinkItemsJson", LanceDataType::Utf8, true),
            LanceField::new("navigationTargetJson", LanceDataType::Utf8, true),
        ])),
        vec![
            Arc::new(LanceStringArray::from(stems)),
            Arc::new(LanceStringArray::from(titles)),
            Arc::new(LanceStringArray::from(paths)),
            Arc::new(LanceStringArray::from(doc_types)),
            Arc::new(LanceStringArray::from(
                tags_json.iter().map(String::as_str).collect::<Vec<_>>(),
            )),
            Arc::new(LanceFloat64Array::from(
                hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(best_sections)),
            Arc::new(LanceStringArray::from(match_reasons)),
            Arc::new(LanceStringArray::from(hierarchical_uris)),
            Arc::new(LanceStringArray::from(
                hierarchy_json
                    .iter()
                    .map(|value| value.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceFloat64Array::from(saliency_scores)),
            Arc::new(LanceStringArray::from(audit_statuses)),
            Arc::new(LanceStringArray::from(verification_states)),
            Arc::new(LanceStringArray::from(
                implicit_backlinks_json
                    .iter()
                    .map(|value| value.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                implicit_backlink_items_json
                    .iter()
                    .map(|value| value.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                navigation_target_json
                    .iter()
                    .map(|value| value.as_deref())
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| format!("failed to build search-hit Flight batch: {error}"))
}

pub(crate) fn search_response_flight_app_metadata(
    response: &SearchResponse,
) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&serde_json::json!({
        "query": response.query,
        "hitCount": response.hit_count,
        "graphConfidenceScore": response.graph_confidence_score,
        "selectedMode": response.selected_mode,
        "intent": response.intent,
        "intentConfidence": response.intent_confidence,
        "searchMode": response.search_mode,
        "partial": response.partial,
        "indexingState": response.indexing_state,
        "pendingRepos": response.pending_repos,
        "skippedRepos": response.skipped_repos,
    }))
    .map_err(|error| format!("failed to encode search response Flight app_metadata: {error}"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use crate::gateway::studio::repo_index::{
        RepoCodeDocument, RepoIndexEntryStatus, RepoIndexPhase, RepoIndexSnapshot,
    };
    use crate::gateway::studio::search::handlers::tests::{
        publish_repo_content_chunk_index, test_studio_state,
    };
    use xiuxian_vector::LanceStringArray;
    use xiuxian_wendao_runtime::transport::{SEARCH_INTENT_ROUTE, SearchFlightRouteProvider};

    use super::StudioIntentSearchFlightRouteProvider;

    #[tokio::test]
    async fn studio_intent_flight_provider_reads_repo_backed_hits() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let valid_repo = temp.path().join("ValidPkg");
        fs::create_dir_all(valid_repo.join("src"))
            .unwrap_or_else(|error| panic!("create valid src: {error}"));
        fs::write(
            valid_repo.join("Project.toml"),
            "name = \"ValidPkg\"\nuuid = \"00000000-0000-0000-0000-000000000001\"\n",
        )
        .unwrap_or_else(|error| panic!("write project: {error}"));

        let studio = Arc::new(test_studio_state());
        studio.set_ui_config(crate::gateway::studio::types::UiConfig {
            projects: Vec::new(),
            repo_projects: vec![crate::gateway::studio::types::UiRepoProjectConfig {
                id: "valid".to_string(),
                root: Some(valid_repo.display().to_string()),
                url: None,
                git_ref: None,
                refresh: None,
                plugins: vec!["julia".to_string()],
            }],
        });
        publish_repo_content_chunk_index(
            studio.as_ref(),
            "valid",
            vec![RepoCodeDocument {
                path: "src/ValidPkg.jl".to_string(),
                language: Some("julia".to_string()),
                contents: Arc::<str>::from(
                    "module ValidPkg\nusing Reexport\n@reexport using ModelingToolkit\nend\n",
                ),
                size_bytes: 62,
                modified_unix_ms: 0,
            }],
        )
        .await;
        studio
            .repo_index
            .set_snapshot_for_test(&Arc::new(RepoIndexSnapshot {
                repo_id: "valid".to_string(),
                analysis: Arc::new(crate::analyzers::RepositoryAnalysisOutput::default()),
            }));
        studio.repo_index.set_status_for_test(RepoIndexEntryStatus {
            repo_id: "valid".to_string(),
            phase: RepoIndexPhase::Ready,
            queue_position: None,
            last_error: None,
            last_revision: Some("abc123".to_string()),
            updated_at: Some("2026-03-22T00:00:00Z".to_string()),
            attempt_count: 1,
        });

        let provider = StudioIntentSearchFlightRouteProvider::new(Arc::clone(&studio));
        let response = provider
            .search_batch(
                SEARCH_INTENT_ROUTE,
                "lang:julia reexport",
                10,
                Some("code_search"),
                Some("valid"),
            )
            .await
            .unwrap_or_else(|error| panic!("intent-search Flight batch: {error}"));
        let batch = response.batch;

        let paths = batch
            .column_by_name("path")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("path should decode as Utf8");
        let doc_types = batch
            .column_by_name("docType")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("docType should decode as Utf8");

        assert!(batch.num_rows() >= 1);
        assert_eq!(paths.value(0), "src/ValidPkg.jl");
        assert_eq!(doc_types.value(0), "file");
    }

    #[tokio::test]
    async fn studio_intent_flight_provider_rejects_non_intent_routes() {
        let provider = StudioIntentSearchFlightRouteProvider::new(Arc::new(test_studio_state()));

        let error = provider
            .search_batch("/search/symbols", "anything", 5, None, None)
            .await
            .expect_err("non-intent route should fail");

        assert_eq!(
            error,
            "studio intent Flight provider only supports route `/search/intent`, got `/search/symbols`"
        );
    }
}
