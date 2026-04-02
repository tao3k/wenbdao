use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceInt32Array, LanceListArray,
    LanceListBuilder, LanceRecordBatch, LanceSchema, LanceStringArray, LanceStringBuilder,
};
use xiuxian_wendao_runtime::transport::{
    REPO_SEARCH_BEST_SECTION_COLUMN, REPO_SEARCH_DOC_ID_COLUMN, REPO_SEARCH_HIERARCHY_COLUMN,
    REPO_SEARCH_LANGUAGE_COLUMN, REPO_SEARCH_MATCH_REASON_COLUMN,
    REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN, REPO_SEARCH_NAVIGATION_LINE_COLUMN,
    REPO_SEARCH_NAVIGATION_LINE_END_COLUMN, REPO_SEARCH_NAVIGATION_PATH_COLUMN,
    REPO_SEARCH_PATH_COLUMN, REPO_SEARCH_SCORE_COLUMN, REPO_SEARCH_TAGS_COLUMN,
    REPO_SEARCH_TITLE_COLUMN, RepoSearchFlightRouteProvider, RerankScoreWeights,
    WendaoFlightService,
};

use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::gateway::studio::router::{GatewayState, StudioState};
use crate::gateway::studio::search::handlers::build_studio_search_flight_service_with_repo_provider;
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::SearchPlaneService;

/// Runtime-backed repo-search Flight provider that reads from the Wendao search plane.
#[derive(Clone)]
pub struct SearchPlaneRepoSearchFlightRouteProvider {
    search_plane: Arc<SearchPlaneService>,
    repo_id: String,
}

impl std::fmt::Debug for SearchPlaneRepoSearchFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SearchPlaneRepoSearchFlightRouteProvider")
            .field("repo_id", &self.repo_id)
            .finish_non_exhaustive()
    }
}

impl SearchPlaneRepoSearchFlightRouteProvider {
    /// Create one search-plane-backed repo-search Flight provider.
    ///
    /// # Errors
    ///
    /// Returns an error when the repo identifier is blank.
    pub fn new(
        search_plane: Arc<SearchPlaneService>,
        repo_id: impl Into<String>,
    ) -> Result<Self, String> {
        let repo_id = repo_id.into();
        if repo_id.trim().is_empty() {
            return Err(
                "search-plane repo-search Flight provider repo_id must not be blank".to_string(),
            );
        }
        Ok(Self {
            search_plane,
            repo_id,
        })
    }
}

#[async_trait]
impl RepoSearchFlightRouteProvider for SearchPlaneRepoSearchFlightRouteProvider {
    async fn repo_search_batch(
        &self,
        query_text: &str,
        limit: usize,
        language_filters: &HashSet<String>,
        path_prefixes: &HashSet<String>,
        title_filters: &HashSet<String>,
        tag_filters: &HashSet<String>,
        filename_filters: &HashSet<String>,
    ) -> Result<LanceRecordBatch, String> {
        let mut hits = self
            .search_plane
            .search_repo_content_chunks(&self.repo_id, query_text, language_filters, limit)
            .await
            .map_err(|error| {
                format!(
                    "search-plane repo-search Flight provider failed for repo `{}`: {error}",
                    self.repo_id
                )
            })?;
        if !path_prefixes.is_empty() {
            hits.retain(|hit| {
                path_prefixes
                    .iter()
                    .any(|prefix| hit.path.starts_with(prefix))
            });
        }
        if !title_filters.is_empty() {
            hits.retain(|hit| {
                let title = hit
                    .title
                    .clone()
                    .unwrap_or_else(|| hit.path.clone())
                    .to_ascii_lowercase();
                title_filters
                    .iter()
                    .any(|filter| title.contains(filter.to_ascii_lowercase().as_str()))
            });
        }
        if !tag_filters.is_empty() {
            hits.retain(|hit| {
                let normalized_tags = hit
                    .tags
                    .iter()
                    .map(|tag| tag.to_ascii_lowercase())
                    .collect::<HashSet<_>>();
                tag_filters
                    .iter()
                    .any(|filter| normalized_tags.contains(&filter.to_ascii_lowercase()))
            });
        }
        if !filename_filters.is_empty() {
            hits.retain(|hit| {
                let normalized_stem = hit.stem.to_ascii_lowercase();
                filename_filters
                    .iter()
                    .any(|filter| normalized_stem == filter.to_ascii_lowercase())
            });
        }
        repo_search_batch_from_hits(&hits)
    }
}

/// Build one runtime-owned Flight service from the Wendao search plane.
///
/// # Errors
///
/// Returns an error when the repo identifier is blank or when the runtime
/// Flight service cannot be constructed for the requested schema version and
/// rerank dimension.
pub fn build_search_plane_flight_service(
    search_plane: Arc<SearchPlaneService>,
    repo_id: impl Into<String>,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
) -> Result<WendaoFlightService, String> {
    build_search_plane_flight_service_with_weights(
        search_plane,
        repo_id,
        expected_schema_version,
        rerank_dimension,
        RerankScoreWeights::default(),
    )
}

/// Build one runtime-owned Flight service from the Wendao search plane with
/// explicit rerank score weights.
///
/// # Errors
///
/// Returns an error when the repo identifier is blank or when the runtime
/// Flight service cannot be constructed for the requested schema version,
/// rerank dimension, and rerank score weights.
pub fn build_search_plane_flight_service_with_weights(
    search_plane: Arc<SearchPlaneService>,
    repo_id: impl Into<String>,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
    rerank_weights: RerankScoreWeights,
) -> Result<WendaoFlightService, String> {
    let provider = Arc::new(SearchPlaneRepoSearchFlightRouteProvider::new(
        search_plane,
        repo_id,
    )?);
    WendaoFlightService::new_with_provider(
        expected_schema_version,
        provider,
        rerank_dimension,
        rerank_weights,
    )
}

/// Build one runtime-owned Flight service from the Wendao search plane and the
/// current Studio-owned semantic search providers.
///
/// # Errors
///
/// Returns an error when the repo identifier is blank or when the runtime
/// Flight service cannot be constructed for the requested schema version and
/// rerank dimension.
pub fn build_search_plane_studio_flight_service(
    search_plane: Arc<SearchPlaneService>,
    repo_id: impl Into<String>,
    gateway_state: Arc<GatewayState>,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
) -> Result<WendaoFlightService, String> {
    build_search_plane_studio_flight_service_with_weights(
        search_plane,
        repo_id,
        gateway_state,
        expected_schema_version,
        rerank_dimension,
        RerankScoreWeights::default(),
    )
}

/// Build one runtime-owned Flight service from the Wendao search plane and the
/// current Studio-owned semantic search providers with explicit rerank weights.
///
/// # Errors
///
/// Returns an error when the repo identifier is blank or when the runtime
/// Flight service cannot be constructed for the requested schema version,
/// rerank dimension, and rerank score weights.
pub fn build_search_plane_studio_flight_service_with_weights(
    search_plane: Arc<SearchPlaneService>,
    repo_id: impl Into<String>,
    gateway_state: Arc<GatewayState>,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
    rerank_weights: RerankScoreWeights,
) -> Result<WendaoFlightService, String> {
    let provider = Arc::new(SearchPlaneRepoSearchFlightRouteProvider::new(
        search_plane,
        repo_id,
    )?);
    build_studio_search_flight_service_with_repo_provider(
        expected_schema_version,
        provider,
        gateway_state,
        rerank_dimension,
        rerank_weights,
    )
}

/// Build one runtime-owned Flight service from the Wendao search plane and one
/// Studio bootstrap state resolved from explicit project/config roots.
///
/// # Errors
///
/// Returns an error when the plugin registry cannot be bootstrapped, when the
/// repo identifier is blank, or when the runtime Flight service cannot be
/// constructed for the requested schema version and rerank dimension.
pub fn build_search_plane_studio_flight_service_for_roots(
    search_plane: Arc<SearchPlaneService>,
    repo_id: impl Into<String>,
    project_root: std::path::PathBuf,
    config_root: std::path::PathBuf,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
) -> Result<WendaoFlightService, String> {
    build_search_plane_studio_flight_service_for_roots_with_weights(
        search_plane,
        repo_id,
        project_root,
        config_root,
        expected_schema_version,
        rerank_dimension,
        RerankScoreWeights::default(),
    )
}

/// Build one runtime-owned Flight service from the Wendao search plane and one
/// Studio bootstrap state resolved from explicit project/config roots with
/// explicit rerank score weights.
///
/// # Errors
///
/// Returns an error when the plugin registry cannot be bootstrapped, when the
/// repo identifier is blank, or when the runtime Flight service cannot be
/// constructed for the requested schema version, rerank dimension, and rerank
/// score weights.
pub fn build_search_plane_studio_flight_service_for_roots_with_weights(
    search_plane: Arc<SearchPlaneService>,
    repo_id: impl Into<String>,
    project_root: std::path::PathBuf,
    config_root: std::path::PathBuf,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
    rerank_weights: RerankScoreWeights,
) -> Result<WendaoFlightService, String> {
    let plugin_registry = Arc::new(
        crate::analyzers::bootstrap_builtin_registry()
            .map_err(|error| format!("bootstrap registry: {error}"))?,
    );
    let studio = StudioState::new_with_bootstrap_ui_config_for_roots_and_search_plane(
        plugin_registry,
        project_root,
        config_root,
        search_plane.as_ref().clone(),
    );
    let gateway_state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(studio),
    });
    build_search_plane_studio_flight_service_with_weights(
        search_plane,
        repo_id,
        gateway_state,
        expected_schema_version,
        rerank_dimension,
        rerank_weights,
    )
}

/// Seed one minimal repo-content sample into the search plane for Flight smoke
/// tests and local bring-up.
///
/// # Errors
///
/// Returns an error when the repo identifier is blank or when the search-plane
/// publication fails.
pub async fn bootstrap_sample_repo_search_content(
    search_plane: &SearchPlaneService,
    repo_id: impl AsRef<str>,
) -> Result<(), String> {
    let repo_id = repo_id.as_ref().trim();
    if repo_id.is_empty() {
        return Err("sample repo-search bootstrap repo_id must not be blank".to_string());
    }

    let documents = vec![
        RepoCodeDocument {
            path: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("pub fn alpha_beta() {}\n"),
            size_bytes: 23,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/flight.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("pub fn flight_router() -> &'static str { \"flight\" }\n"),
            size_bytes: 52,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/search.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(
                "pub fn repo_search_kernel() -> &'static str { \"searchonlytoken semantic search kernel\" }\n",
            ),
            size_bytes: 88,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/flight_search.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(
                "pub fn flight_search_bridge() -> &'static str { \"flightbridgetoken flight search bridge\" }\n",
            ),
            size_bytes: 92,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/camelbridge.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(
                "pub fn camel_bridge_lower() -> &'static str { \"camelbridgetoken\" }\n",
            ),
            size_bytes: 70,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/a_rank.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(
                "pub fn alpha_rank() -> &'static str { \"ranktieexacttoken\" }\n",
            ),
            size_bytes: 62,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/z_rank.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(
                "pub fn zeta_rank() -> &'static str { \"ranktieexacttoken\" }\n",
            ),
            size_bytes: 61,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "README.md".to_string(),
            language: Some("markdown".to_string()),
            contents: Arc::<str>::from(
                "# alpha repo\nThis repo mentions alpha beta flight search.\n",
            ),
            size_bytes: 56,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "docs/CamelBridge.md".to_string(),
            language: Some("markdown".to_string()),
            contents: Arc::<str>::from(
                "# CamelBridgeToken\nExact-case bridge token for flight ranking.\n",
            ),
            size_bytes: 64,
            modified_unix_ms: 10,
        },
    ];

    search_plane
        .publish_repo_content_chunks_with_revision(repo_id, &documents, Some("flight-smoke-rev"))
        .await
        .map_err(|error| {
            format!("failed to bootstrap sample repo-search content for `{repo_id}`: {error}")
        })
}

fn repo_search_batch_from_hits(hits: &[SearchHit]) -> Result<LanceRecordBatch, String> {
    let doc_ids = hits
        .iter()
        .map(repo_search_doc_id_from_hit)
        .collect::<Vec<_>>();
    let paths = hits.iter().map(|hit| hit.path.clone()).collect::<Vec<_>>();
    let titles = hits
        .iter()
        .map(|hit| hit.title.clone().unwrap_or_else(|| hit.path.clone()))
        .collect::<Vec<_>>();
    let best_sections = hits
        .iter()
        .map(|hit| hit.best_section.clone().unwrap_or_default())
        .collect::<Vec<_>>();
    let match_reasons = hits
        .iter()
        .map(|hit| hit.match_reason.clone().unwrap_or_default())
        .collect::<Vec<_>>();
    let navigation_paths = hits
        .iter()
        .map(|hit| {
            hit.navigation_target
                .as_ref()
                .map(|target| target.path.clone())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let navigation_categories = hits
        .iter()
        .map(|hit| {
            hit.navigation_target
                .as_ref()
                .map(|target| target.category.clone())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let navigation_lines = hits
        .iter()
        .map(|hit| {
            hit.navigation_target
                .as_ref()
                .and_then(|target| target.line)
                .map_or(0_i32, |value| value as i32)
        })
        .collect::<Vec<_>>();
    let navigation_line_ends = hits
        .iter()
        .map(|hit| {
            hit.navigation_target
                .as_ref()
                .and_then(|target| target.line_end)
                .map_or(0_i32, |value| value as i32)
        })
        .collect::<Vec<_>>();
    let hierarchy_rows = hits
        .iter()
        .map(|hit| {
            hit.hierarchy
                .as_ref()
                .map_or_else(|| &[][..], Vec::as_slice)
        })
        .collect::<Vec<_>>();
    let tag_rows = hits
        .iter()
        .map(|hit| hit.tags.as_slice())
        .collect::<Vec<_>>();
    let scores = hits.iter().map(|hit| hit.score).collect::<Vec<_>>();
    let languages = hits
        .iter()
        .map(repo_search_language_from_hit)
        .collect::<Vec<_>>();

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new(REPO_SEARCH_DOC_ID_COLUMN, LanceDataType::Utf8, false),
            LanceField::new(REPO_SEARCH_PATH_COLUMN, LanceDataType::Utf8, false),
            LanceField::new(REPO_SEARCH_TITLE_COLUMN, LanceDataType::Utf8, false),
            LanceField::new(REPO_SEARCH_BEST_SECTION_COLUMN, LanceDataType::Utf8, false),
            LanceField::new(REPO_SEARCH_MATCH_REASON_COLUMN, LanceDataType::Utf8, false),
            LanceField::new(
                REPO_SEARCH_NAVIGATION_PATH_COLUMN,
                LanceDataType::Utf8,
                false,
            ),
            LanceField::new(
                REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN,
                LanceDataType::Utf8,
                false,
            ),
            LanceField::new(
                REPO_SEARCH_NAVIGATION_LINE_COLUMN,
                LanceDataType::Int32,
                false,
            ),
            LanceField::new(
                REPO_SEARCH_NAVIGATION_LINE_END_COLUMN,
                LanceDataType::Int32,
                false,
            ),
            LanceField::new(
                REPO_SEARCH_HIERARCHY_COLUMN,
                LanceDataType::List(Arc::new(LanceField::new("item", LanceDataType::Utf8, true))),
                false,
            ),
            LanceField::new(
                REPO_SEARCH_TAGS_COLUMN,
                LanceDataType::List(Arc::new(LanceField::new("item", LanceDataType::Utf8, true))),
                false,
            ),
            LanceField::new(REPO_SEARCH_SCORE_COLUMN, LanceDataType::Float64, false),
            LanceField::new(REPO_SEARCH_LANGUAGE_COLUMN, LanceDataType::Utf8, false),
        ])),
        vec![
            Arc::new(LanceStringArray::from(doc_ids)),
            Arc::new(LanceStringArray::from(paths)),
            Arc::new(LanceStringArray::from(titles)),
            Arc::new(LanceStringArray::from(best_sections)),
            Arc::new(LanceStringArray::from(match_reasons)),
            Arc::new(LanceStringArray::from(navigation_paths)),
            Arc::new(LanceStringArray::from(navigation_categories)),
            Arc::new(LanceInt32Array::from(navigation_lines)),
            Arc::new(LanceInt32Array::from(navigation_line_ends)),
            Arc::new(build_utf8_list_array(&hierarchy_rows)),
            Arc::new(build_utf8_list_array(&tag_rows)),
            Arc::new(LanceFloat64Array::from(scores)),
            Arc::new(LanceStringArray::from(languages)),
        ],
    )
    .map_err(|error| format!("failed to build repo-search Flight batch: {error}"))
}

fn build_utf8_list_array(rows: &[&[String]]) -> LanceListArray {
    let mut builder = LanceListBuilder::new(LanceStringBuilder::new());
    for row in rows {
        for value in *row {
            builder.values().append_value(value);
        }
        builder.append(true);
    }
    builder.finish()
}

fn repo_search_doc_id_from_hit(hit: &SearchHit) -> String {
    let stem = hit.stem.trim();
    if stem.is_empty() {
        hit.path.clone()
    } else {
        stem.to_string()
    }
}

fn repo_search_language_from_hit(hit: &SearchHit) -> String {
    hit.tags
        .iter()
        .find_map(|tag| tag.strip_prefix("lang:").map(ToString::to_string))
        .or_else(|| infer_code_language(hit.path.as_str()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn infer_code_language(path: &str) -> Option<String> {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("jl") || ext.eq_ignore_ascii_case("julia") => {
            Some("julia".to_string())
        }
        Some(ext) if ext.eq_ignore_ascii_case("mo") || ext.eq_ignore_ascii_case("modelica") => {
            Some("modelica".to_string())
        }
        Some(ext) if ext.eq_ignore_ascii_case("rs") => Some("rust".to_string()),
        Some(ext) if ext.eq_ignore_ascii_case("py") => Some("python".to_string()),
        Some(ext) if ext.eq_ignore_ascii_case("ts") || ext.eq_ignore_ascii_case("tsx") => {
            Some("typescript".to_string())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::Arc;

    use arrow_flight::FlightDescriptor;
    use arrow_flight::flight_service_server::FlightService;
    use tempfile::tempdir;
    use tonic::Request;
    use xiuxian_vector::{LanceFloat64Array, LanceStringArray};

    use super::{
        SearchPlaneRepoSearchFlightRouteProvider, bootstrap_sample_repo_search_content,
        build_search_plane_flight_service, build_search_plane_studio_flight_service,
        build_search_plane_studio_flight_service_for_roots,
    };
    use crate::analyzers::bootstrap_builtin_registry;
    use crate::gateway::studio::repo_index::RepoCodeDocument;
    use crate::gateway::studio::router::{GatewayState, StudioState};
    use crate::gateway::studio::search::build_symbol_index;
    use crate::gateway::studio::types::{UiConfig, UiProjectConfig};
    use crate::search_plane::{
        SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneService,
    };
    use xiuxian_wendao_runtime::transport::{
        ANALYSIS_CODE_AST_ROUTE, ANALYSIS_MARKDOWN_ROUTE, RepoSearchFlightRouteProvider,
        SEARCH_SYMBOLS_ROUTE, WENDAO_ANALYSIS_LINE_HEADER, WENDAO_ANALYSIS_PATH_HEADER,
        WENDAO_ANALYSIS_REPO_HEADER, WENDAO_SCHEMA_VERSION_HEADER, WENDAO_SEARCH_LIMIT_HEADER,
        WENDAO_SEARCH_QUERY_HEADER, flight_descriptor_path,
    };

    fn repo_document(path: &str, language: &str, contents: &str) -> RepoCodeDocument {
        RepoCodeDocument {
            path: path.to_string(),
            language: Some(language.to_string()),
            contents: Arc::<str>::from(contents),
            size_bytes: u64::try_from(contents.len()).expect("document length should fit"),
            modified_unix_ms: 10,
        }
    }

    fn populate_search_headers(
        metadata: &mut tonic::metadata::MetadataMap,
        query: &str,
        limit: usize,
    ) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            "v2".parse()
                .unwrap_or_else(|error| panic!("schema metadata: {error}")),
        );
        metadata.insert(
            WENDAO_SEARCH_QUERY_HEADER,
            query
                .parse()
                .unwrap_or_else(|error| panic!("query metadata: {error}")),
        );
        metadata.insert(
            WENDAO_SEARCH_LIMIT_HEADER,
            limit
                .to_string()
                .parse()
                .unwrap_or_else(|error| panic!("limit metadata: {error}")),
        );
    }

    fn populate_markdown_analysis_headers(metadata: &mut tonic::metadata::MetadataMap, path: &str) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            "v2".parse()
                .unwrap_or_else(|error| panic!("schema metadata: {error}")),
        );
        metadata.insert(
            WENDAO_ANALYSIS_PATH_HEADER,
            path.parse()
                .unwrap_or_else(|error| panic!("analysis path metadata: {error}")),
        );
    }

    fn populate_code_ast_analysis_headers(
        metadata: &mut tonic::metadata::MetadataMap,
        path: &str,
        repo_id: &str,
        line_hint: Option<usize>,
    ) {
        populate_markdown_analysis_headers(metadata, path);
        metadata.insert(
            WENDAO_ANALYSIS_REPO_HEADER,
            repo_id
                .parse()
                .unwrap_or_else(|error| panic!("analysis repo metadata: {error}")),
        );
        if let Some(line_hint) = line_hint {
            metadata.insert(
                WENDAO_ANALYSIS_LINE_HEADER,
                line_hint
                    .to_string()
                    .parse()
                    .unwrap_or_else(|error| panic!("analysis line metadata: {error}")),
            );
        }
    }

    fn test_studio_state(search_plane_root: PathBuf) -> StudioState {
        StudioState::new_with_bootstrap_ui_config_and_search_plane_root(
            Arc::new(
                bootstrap_builtin_registry()
                    .unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
            ),
            search_plane_root,
        )
    }

    #[tokio::test]
    async fn search_plane_repo_search_provider_reads_repo_content_hits() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-provider"),
            SearchMaintenancePolicy::default(),
        ));
        let repo_id = "alpha/repo";
        let documents = vec![
            repo_document("src/lib.rs", "rust", "pub fn alpha_beta() {}\n"),
            repo_document("src/other.rs", "rust", "pub fn unrelated() {}\n"),
        ];
        service
            .publish_repo_content_chunks_with_revision(repo_id, &documents, Some("rev-1"))
            .await
            .expect("repo content publication should succeed");

        let provider = SearchPlaneRepoSearchFlightRouteProvider::new(Arc::clone(&service), repo_id)
            .expect("provider should build");
        let batch = provider
            .repo_search_batch(
                "alpha",
                5,
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
            )
            .await
            .expect("provider should materialize one search batch");

        let doc_ids = batch
            .column_by_name("doc_id")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("doc_id should decode as Utf8");
        let paths = batch
            .column_by_name("path")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("path should decode as Utf8");
        let languages = batch
            .column_by_name("language")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("language should decode as Utf8");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(doc_ids.value(0), "lib.rs");
        assert_eq!(paths.value(0), "src/lib.rs");
        assert_eq!(languages.value(0), "rust");
    }

    #[tokio::test]
    async fn build_search_plane_studio_flight_service_accepts_runtime_studio_providers() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(project_root.join("packages/rust/crates/demo/src"))
            .expect("project fixture dirs should build");
        std::fs::write(
            project_root.join("packages/rust/crates/demo/src/lib.rs"),
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
        )
        .expect("project fixture file should write");

        let mut studio = test_studio_state(project_root.join("studio-search-plane"));
        studio.project_root = project_root.clone();
        studio.config_root = project_root.clone();
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
        let state = Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio),
        });

        let search_plane = Arc::new(SearchPlaneService::with_paths(
            project_root,
            storage_root,
            SearchManifestKeyspace::new("xiuxian:test:search-plane-studio-flight-service"),
            SearchMaintenancePolicy::default(),
        ));
        let flight_service =
            build_search_plane_studio_flight_service(search_plane, "alpha/repo", state, "v2", 3)
                .expect("studio flight service should build");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_SYMBOLS_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_search_headers(request.metadata_mut(), "alpha", 5);

        let response = flight_service
            .get_flight_info(request)
            .await
            .expect("studio flight service should resolve symbols route");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("symbols route should emit one ticket");

        assert_eq!(ticket, SEARCH_SYMBOLS_ROUTE);
    }

    #[tokio::test]
    async fn build_search_plane_studio_flight_service_for_roots_accepts_runtime_studio_providers() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(project_root.join("packages/rust/crates/demo/src"))
            .expect("project fixture dirs should build");
        std::fs::write(
            project_root.join("packages/rust/crates/demo/src/lib.rs"),
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
        )
        .expect("project fixture file should write");
        std::fs::write(
            project_root.join("wendao.toml"),
            r#"
[link_graph.projects.kernel]
root = "."
dirs = ["packages"]
"#,
        )
        .expect("wendao.toml should write");

        let search_plane = Arc::new(SearchPlaneService::with_paths(
            project_root.clone(),
            storage_root,
            SearchManifestKeyspace::new("xiuxian:test:search-plane-studio-flight-service-roots"),
            SearchMaintenancePolicy::default(),
        ));
        let flight_service = build_search_plane_studio_flight_service_for_roots(
            search_plane,
            "alpha/repo",
            project_root.clone(),
            project_root.clone(),
            "v2",
            3,
        )
        .expect("studio flight service should build from roots");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_SYMBOLS_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_search_headers(request.metadata_mut(), "alpha", 5);

        let response = flight_service
            .get_flight_info(request)
            .await
            .expect("studio flight service should resolve symbols route");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("symbols route should emit one ticket");

        assert_eq!(ticket, SEARCH_SYMBOLS_ROUTE);
    }

    #[tokio::test]
    async fn build_search_plane_studio_flight_service_for_roots_accepts_markdown_analysis_routes() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(project_root.join("docs")).expect("project docs dir should build");
        std::fs::write(
            project_root.join("docs/analysis.md"),
            "# Analysis Kernel\n\n## Inputs\n- [ ] Parse markdown\n",
        )
        .expect("project markdown fixture should write");
        std::fs::write(
            project_root.join("wendao.toml"),
            r#"
[link_graph.projects.kernel]
root = "."
dirs = ["docs"]
"#,
        )
        .expect("wendao.toml should write");

        let search_plane = Arc::new(SearchPlaneService::with_paths(
            project_root.clone(),
            storage_root,
            SearchManifestKeyspace::new(
                "xiuxian:test:flight-search-plane-studio-flight-service-roots-markdown",
            ),
            SearchMaintenancePolicy::default(),
        ));
        let flight_service = build_search_plane_studio_flight_service_for_roots(
            search_plane,
            "alpha/repo",
            project_root.clone(),
            project_root.clone(),
            "v2",
            3,
        )
        .expect("studio flight service should build from roots");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(ANALYSIS_MARKDOWN_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_markdown_analysis_headers(request.metadata_mut(), "docs/analysis.md");

        let response = flight_service
            .get_flight_info(request)
            .await
            .expect("studio flight service should resolve markdown analysis route");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("markdown analysis route should emit one ticket");

        assert_eq!(ticket, ANALYSIS_MARKDOWN_ROUTE);
    }

    #[tokio::test]
    async fn build_search_plane_studio_flight_service_for_roots_accepts_code_ast_analysis_routes() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(project_root.join("repo/src"))
            .expect("project repo dir should build");
        git2::Repository::init(project_root.join("repo"))
            .expect("analysis repo fixture should initialize");
        std::fs::write(
            project_root.join("repo/Project.toml"),
            "name = \"Demo\"\nuuid = \"00000000-0000-0000-0000-000000000001\"\n",
        )
        .expect("Project.toml should write");
        std::fs::write(
            project_root.join("repo/src/lib.jl"),
            "module Demo\nexport solve\nsolve(x) = x + 1\nend\n",
        )
        .expect("source fixture should write");
        std::fs::write(
            project_root.join("wendao.toml"),
            r#"
[link_graph.projects.kernel]
root = "."
dirs = ["docs"]

[link_graph.projects.demo]
root = "repo"
plugins = ["julia"]
"#,
        )
        .expect("wendao.toml should write");

        let search_plane = Arc::new(SearchPlaneService::with_paths(
            project_root.clone(),
            storage_root,
            SearchManifestKeyspace::new(
                "xiuxian:test:flight-search-plane-studio-flight-service-roots-code-ast",
            ),
            SearchMaintenancePolicy::default(),
        ));
        let flight_service = build_search_plane_studio_flight_service_for_roots(
            search_plane,
            "alpha/repo",
            project_root.clone(),
            project_root.clone(),
            "v2",
            3,
        )
        .expect("studio flight service should build from roots");
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(ANALYSIS_CODE_AST_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_code_ast_analysis_headers(request.metadata_mut(), "src/lib.jl", "demo", Some(3));

        let response = flight_service
            .get_flight_info(request)
            .await
            .expect("studio flight service should resolve code AST analysis route");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("code AST analysis route should emit one ticket");

        assert_eq!(ticket, ANALYSIS_CODE_AST_ROUTE);
    }

    #[tokio::test]
    async fn search_plane_repo_search_provider_applies_language_filters() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-provider-filters"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");

        let provider =
            SearchPlaneRepoSearchFlightRouteProvider::new(Arc::clone(&service), "alpha/repo")
                .expect("provider should build");
        let batch = provider
            .repo_search_batch(
                "alpha",
                10,
                &HashSet::from(["markdown".to_string()]),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
            )
            .await
            .expect("provider should materialize one markdown-filtered search batch");

        let paths = batch
            .column_by_name("path")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("path should decode as Utf8");
        let languages = batch
            .column_by_name("language")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("language should decode as Utf8");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(paths.value(0), "README.md");
        assert_eq!(languages.value(0), "markdown");
    }

    #[tokio::test]
    async fn search_plane_repo_search_provider_applies_path_prefix_filters() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-provider-prefixes"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");

        let provider =
            SearchPlaneRepoSearchFlightRouteProvider::new(Arc::clone(&service), "alpha/repo")
                .expect("provider should build");
        let batch = provider
            .repo_search_batch(
                "flightbridgetoken",
                10,
                &HashSet::new(),
                &HashSet::from(["src/flight".to_string()]),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
            )
            .await
            .expect("provider should materialize one path-filtered search batch");

        let paths = batch
            .column_by_name("path")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("path should decode as Utf8");

        assert_eq!(batch.num_rows(), 1);
        assert!(paths.value(0).starts_with("src/flight"));
    }

    #[tokio::test]
    async fn search_plane_repo_search_provider_applies_title_filters() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-provider-titles"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");

        let provider =
            SearchPlaneRepoSearchFlightRouteProvider::new(Arc::clone(&service), "alpha/repo")
                .expect("provider should build");
        let batch = provider
            .repo_search_batch(
                "alpha",
                10,
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::from(["readme".to_string()]),
                &HashSet::new(),
                &HashSet::new(),
            )
            .await
            .expect("provider should materialize one title-filtered search batch");

        let paths = batch
            .column_by_name("path")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("path should decode as Utf8");
        let titles = batch
            .column_by_name("title")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("title should decode as Utf8");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(paths.value(0), "README.md");
        assert_eq!(titles.value(0), "README.md");
    }

    #[tokio::test]
    async fn search_plane_repo_search_provider_applies_tag_filters() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-provider-tags"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");

        let provider =
            SearchPlaneRepoSearchFlightRouteProvider::new(Arc::clone(&service), "alpha/repo")
                .expect("provider should build");
        let batch = provider
            .repo_search_batch(
                "alpha",
                10,
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::from(["lang:markdown".to_string()]),
                &HashSet::new(),
            )
            .await
            .expect("provider should materialize one tag-filtered search batch");

        let paths = batch
            .column_by_name("path")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("path should decode as Utf8");
        let languages = batch
            .column_by_name("language")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("language should decode as Utf8");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(paths.value(0), "README.md");
        assert_eq!(languages.value(0), "markdown");
    }

    #[tokio::test]
    async fn search_plane_repo_search_provider_exposes_exact_match_tag() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-provider-exact-tag"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");

        let provider =
            SearchPlaneRepoSearchFlightRouteProvider::new(Arc::clone(&service), "alpha/repo")
                .expect("provider should build");
        let batch = provider
            .repo_search_batch(
                "searchonlytoken",
                10,
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::from(["match:exact".to_string()]),
                &HashSet::new(),
            )
            .await
            .expect("provider should materialize one exact-match-tagged search batch");

        let paths = batch
            .column_by_name("path")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("path should decode as Utf8");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(paths.value(0), "src/search.rs");
    }

    #[tokio::test]
    async fn search_plane_repo_search_provider_prefers_exact_case_match_over_folded_match() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-provider-exact-rank"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");

        let provider =
            SearchPlaneRepoSearchFlightRouteProvider::new(Arc::clone(&service), "alpha/repo")
                .expect("provider should build");
        let batch = provider
            .repo_search_batch(
                "CamelBridgeToken",
                2,
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
            )
            .await
            .expect("provider should materialize one exact-ranked search batch");

        let paths = batch
            .column_by_name("path")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("path should decode as Utf8");
        let scores = batch
            .column_by_name("score")
            .and_then(|column| column.as_any().downcast_ref::<LanceFloat64Array>())
            .expect("score should decode as Float64");

        assert_eq!(batch.num_rows(), 2);
        assert_eq!(paths.value(0), "docs/CamelBridge.md");
        assert_eq!(paths.value(1), "src/camelbridge.rs");
        assert!(scores.value(0) > scores.value(1));
    }

    #[tokio::test]
    async fn search_plane_repo_search_provider_applies_filename_filters() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-provider-filenames"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");

        let provider =
            SearchPlaneRepoSearchFlightRouteProvider::new(Arc::clone(&service), "alpha/repo")
                .expect("provider should build");
        let batch = provider
            .repo_search_batch(
                "alpha",
                10,
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::from(["readme.md".to_string()]),
            )
            .await
            .expect("provider should materialize one filename-filtered search batch");

        let paths = batch
            .column_by_name("path")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("path should decode as Utf8");
        let doc_ids = batch
            .column_by_name("doc_id")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .expect("doc_id should decode as Utf8");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(paths.value(0), "README.md");
        assert_eq!(doc_ids.value(0), "README.md");
    }

    #[test]
    fn search_plane_repo_search_provider_rejects_blank_repo_id() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-provider-blank"),
            SearchMaintenancePolicy::default(),
        ));
        let error = SearchPlaneRepoSearchFlightRouteProvider::new(service, "   ")
            .expect_err("blank repo id should fail");
        assert_eq!(
            error,
            "search-plane repo-search Flight provider repo_id must not be blank"
        );
    }

    #[test]
    fn build_search_plane_flight_service_accepts_runtime_search_plane_provider() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-service"),
            SearchMaintenancePolicy::default(),
        ));
        let flight_service = build_search_plane_flight_service(service, "alpha/repo", "v2", 3)
            .expect("flight service should build");

        let _ = flight_service;
    }

    #[tokio::test]
    async fn bootstrap_sample_repo_search_content_publishes_queryable_rows() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-bootstrap"),
            SearchMaintenancePolicy::default(),
        );
        bootstrap_sample_repo_search_content(&service, "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");

        let hits = service
            .search_repo_content_chunks("alpha/repo", "flight", &HashSet::new(), 5)
            .await
            .expect("bootstrapped repo should be searchable");

        assert!(!hits.is_empty());
        assert!(hits.iter().any(|hit| hit.path == "src/flight.rs"));
        assert!(hits.iter().any(|hit| hit.path == "src/flight_search.rs"));
    }

    #[tokio::test]
    async fn bootstrap_sample_repo_search_content_respects_query_and_limit() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-bootstrap-query-limit"),
            SearchMaintenancePolicy::default(),
        );
        bootstrap_sample_repo_search_content(&service, "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");

        let search_hits = service
            .search_repo_content_chunks("alpha/repo", "searchonlytoken", &HashSet::new(), 1)
            .await
            .expect("bootstrapped repo should be searchable by search keyword");
        let flight_hits = service
            .search_repo_content_chunks("alpha/repo", "flightbridgetoken", &HashSet::new(), 5)
            .await
            .expect("bootstrapped repo should be searchable by combined keywords");

        assert_eq!(search_hits.len(), 1);
        assert_eq!(search_hits[0].path, "src/search.rs");
        assert!(
            flight_hits
                .iter()
                .any(|hit| hit.path == "src/flight_search.rs")
        );
    }

    #[tokio::test]
    async fn bootstrap_sample_repo_search_content_uses_path_order_for_exact_match_ties() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let service = SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-bootstrap-rank-tie"),
            SearchMaintenancePolicy::default(),
        );
        bootstrap_sample_repo_search_content(&service, "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");

        let hits = service
            .search_repo_content_chunks("alpha/repo", "ranktieexacttoken", &HashSet::new(), 1)
            .await
            .expect("bootstrapped repo should expose deterministic exact-match tie ordering");

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "src/a_rank.rs");
    }

    #[tokio::test]
    async fn bootstrap_sample_repo_search_content_persists_across_service_restart() {
        let temp_dir = tempdir().expect("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        std::fs::create_dir_all(&project_root).expect("project root should build");

        let writer = SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-bootstrap-persist"),
            SearchMaintenancePolicy::default(),
        );
        bootstrap_sample_repo_search_content(&writer, "alpha/repo")
            .await
            .expect("sample bootstrap should publish repo content");
        drop(writer);

        let reader = SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-search-plane-bootstrap-persist"),
            SearchMaintenancePolicy::default(),
        );
        let hits = reader
            .search_repo_content_chunks("alpha/repo", "alpha", &HashSet::new(), 5)
            .await
            .expect("bootstrapped repo should remain searchable after restart");

        assert!(!hits.is_empty());
        assert!(hits.iter().any(|hit| hit.path == "src/lib.rs"));
    }
}
