use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
#[cfg(test)]
use axum::Json;
#[cfg(test)]
use axum::extract::{Query, State};
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema, LanceStringArray,
    LanceUInt64Array,
};
use xiuxian_wendao_runtime::transport::AstSearchFlightRouteProvider;

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::search::project_scope::project_metadata_for_path;
use crate::gateway::studio::types::{AstSearchHit, AstSearchResponse, UiProjectConfig};

use super::queries::AstSearchQuery;

#[cfg(test)]
pub async fn search_ast(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AstSearchQuery>,
) -> Result<Json<AstSearchResponse>, StudioApiError> {
    let response = load_ast_search_response(state.as_ref(), query).await?;
    Ok(Json(response))
}

pub(crate) async fn load_ast_search_response(
    state: &GatewayState,
    query: AstSearchQuery,
) -> Result<AstSearchResponse, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "AST search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(20).max(1);
    state.studio.ensure_local_symbol_index_ready().await?;
    let ast_hits = state
        .studio
        .search_local_symbol_hits(query_text, limit)
        .await?;
    let projects = state.studio.configured_projects();
    let mut hits = ast_hits
        .iter()
        .map(|hit| {
            enrich_ast_hit(
                hit,
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                projects.as_slice(),
            )
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    hits.truncate(limit);

    Ok(AstSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "definitions".to_string(),
    })
}

pub(crate) struct StudioAstSearchFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioAstSearchFlightRouteProvider {
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioAstSearchFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioAstSearchFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl AstSearchFlightRouteProvider for StudioAstSearchFlightRouteProvider {
    async fn ast_search_batch(
        &self,
        query_text: &str,
        limit: usize,
    ) -> Result<LanceRecordBatch, String> {
        let response = load_ast_search_response(
            self.state.as_ref(),
            AstSearchQuery {
                q: Some(query_text.to_string()),
                limit: Some(limit),
            },
        )
        .await
        .map_err(|error| {
            error
                .error
                .details
                .clone()
                .unwrap_or_else(|| format!("{}: {}", error.code(), error.error.message))
        })?;
        build_ast_hits_flight_batch(response.hits.as_slice())
    }
}

fn build_ast_hits_flight_batch(hits: &[AstSearchHit]) -> Result<LanceRecordBatch, String> {
    let names = hits.iter().map(|hit| hit.name.as_str()).collect::<Vec<_>>();
    let signatures = hits
        .iter()
        .map(|hit| hit.signature.as_str())
        .collect::<Vec<_>>();
    let paths = hits.iter().map(|hit| hit.path.as_str()).collect::<Vec<_>>();
    let languages = hits
        .iter()
        .map(|hit| hit.language.as_str())
        .collect::<Vec<_>>();
    let crate_names = hits
        .iter()
        .map(|hit| hit.crate_name.as_str())
        .collect::<Vec<_>>();
    let project_names = hits
        .iter()
        .map(|hit| hit.project_name.as_deref())
        .collect::<Vec<_>>();
    let root_labels = hits
        .iter()
        .map(|hit| hit.root_label.as_deref())
        .collect::<Vec<_>>();
    let node_kinds = hits
        .iter()
        .map(|hit| hit.node_kind.as_deref())
        .collect::<Vec<_>>();
    let owner_titles = hits
        .iter()
        .map(|hit| hit.owner_title.as_deref())
        .collect::<Vec<_>>();
    let navigation_targets_json = hits
        .iter()
        .map(|hit| serde_json::to_string(&hit.navigation_target).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

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
            LanceField::new("lineStart", LanceDataType::UInt64, false),
            LanceField::new("lineEnd", LanceDataType::UInt64, false),
            LanceField::new("score", LanceDataType::Float64, false),
        ])),
        vec![
            Arc::new(LanceStringArray::from(names)),
            Arc::new(LanceStringArray::from(signatures)),
            Arc::new(LanceStringArray::from(paths)),
            Arc::new(LanceStringArray::from(languages)),
            Arc::new(LanceStringArray::from(crate_names)),
            Arc::new(LanceStringArray::from(project_names)),
            Arc::new(LanceStringArray::from(root_labels)),
            Arc::new(LanceStringArray::from(node_kinds)),
            Arc::new(LanceStringArray::from(owner_titles)),
            Arc::new(LanceStringArray::from(
                navigation_targets_json
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceUInt64Array::from(
                hits.iter()
                    .map(|hit| hit.line_start as u64)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceUInt64Array::from(
                hits.iter()
                    .map(|hit| hit.line_end as u64)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceFloat64Array::from(
                hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| format!("failed to build AST-search Flight batch: {error}"))
}

fn enrich_ast_hit(
    hit: &AstSearchHit,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> AstSearchHit {
    let metadata =
        project_metadata_for_path(project_root, config_root, projects, hit.path.as_str());
    let mut navigation_target = hit.navigation_target.clone();
    navigation_target
        .project_name
        .clone_from(&metadata.project_name);
    navigation_target
        .root_label
        .clone_from(&metadata.root_label);

    let mut enriched = hit.clone();
    enriched.project_name = metadata.project_name;
    enriched.root_label = metadata.root_label;
    enriched.navigation_target = navigation_target;
    if enriched.score <= 0.0 {
        enriched.score = ast_hit_score(&enriched);
    }
    enriched
}

fn ast_hit_score(hit: &AstSearchHit) -> f64 {
    if hit.language != "markdown" {
        return 0.95;
    }

    match hit.node_kind.as_deref() {
        Some("task") => 0.88,
        Some("property" | "observation") => 0.8,
        _ => 0.95,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use xiuxian_wendao_runtime::transport::AstSearchFlightRouteProvider;

    use crate::gateway::studio::search::build_symbol_index;
    use crate::gateway::studio::search::handlers::tests::test_studio_state;
    use crate::gateway::studio::types::{UiConfig, UiProjectConfig};

    #[tokio::test]
    async fn studio_ast_flight_provider_materializes_ast_batches() {
        let temp_dir = tempdir().expect("AST provider tempdir should build");
        let source_dir = temp_dir.path().join("packages/rust/crates/demo/src");
        fs::create_dir_all(&source_dir).expect("AST provider source dir should build");
        fs::write(
            source_dir.join("lib.rs"),
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
        )
        .expect("AST provider source fixture should write");

        let mut studio = test_studio_state();
        studio.project_root = temp_dir.path().to_path_buf();
        studio.config_root = temp_dir.path().to_path_buf();
        studio.set_ui_config(UiConfig {
            projects: vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["packages".to_string()],
            }],
            repo_projects: Vec::new(),
        });
        let warmed_index = build_symbol_index(
            studio.project_root.as_path(),
            studio.config_root.as_path(),
            studio.configured_projects().as_slice(),
        );
        studio.symbol_index_coordinator.set_ready_index_for_test(
            studio.configured_projects().as_slice(),
            Arc::clone(&studio.symbol_index),
            warmed_index,
        );

        let provider = StudioAstSearchFlightRouteProvider::new(Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio),
        }));

        let batch = provider
            .ast_search_batch("alpha", 5)
            .await
            .expect("dedicated AST provider should accept AST requests");

        assert!(batch.num_rows() >= 1);
    }
}
