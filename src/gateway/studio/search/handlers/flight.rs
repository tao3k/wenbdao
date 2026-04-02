use std::sync::Arc;

use async_trait::async_trait;
#[cfg(feature = "julia")]
use xiuxian_wendao_runtime::transport::{
    RepoSearchFlightRouteProvider, RerankScoreWeights, SEARCH_INTENT_ROUTE, SEARCH_KNOWLEDGE_ROUTE,
    SEARCH_REFERENCES_ROUTE, SEARCH_SYMBOLS_ROUTE, SearchFlightRouteProvider,
    SearchFlightRouteResponse, WendaoFlightService,
};
#[cfg(not(feature = "julia"))]
use xiuxian_wendao_runtime::transport::{
    SEARCH_AST_ROUTE, SEARCH_INTENT_ROUTE, SEARCH_KNOWLEDGE_ROUTE, SEARCH_REFERENCES_ROUTE,
    SEARCH_SYMBOLS_ROUTE, SearchFlightRouteProvider, SearchFlightRouteResponse,
};

use crate::gateway::studio::router::GatewayState;
use crate::gateway::studio::router::handlers::analysis::{
    StudioCodeAstAnalysisFlightRouteProvider, StudioMarkdownAnalysisFlightRouteProvider,
};
use crate::gateway::studio::router::handlers::graph::StudioGraphNeighborsFlightRouteProvider;
#[cfg(feature = "julia")]
use crate::gateway::studio::vfs::StudioVfsResolveFlightRouteProvider;

use super::ast::StudioAstSearchFlightRouteProvider;
use super::attachments::StudioAttachmentSearchFlightRouteProvider;
use super::autocomplete::StudioAutocompleteFlightRouteProvider;
use super::definition::StudioDefinitionFlightRouteProvider;
use super::knowledge::intent::flight::load_intent_search_flight_response;
use super::knowledge::load_knowledge_search_flight_response;
use super::queries::{ReferenceSearchQuery, SymbolSearchQuery};
use super::references::load_reference_search_flight_response;
use super::symbols::load_symbol_search_flight_response;

/// Studio-backed aggregate Flight provider for the currently-aligned semantic
/// search families.
#[derive(Clone)]
pub(crate) struct StudioSearchFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioSearchFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioSearchFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioSearchFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl SearchFlightRouteProvider for StudioSearchFlightRouteProvider {
    async fn search_batch(
        &self,
        route: &str,
        query_text: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Result<SearchFlightRouteResponse, String> {
        match route {
            SEARCH_INTENT_ROUTE => load_intent_search_flight_response(
                Arc::clone(&self.state.studio),
                query_text,
                query_text,
                repo_hint,
                limit,
                intent.map(ToString::to_string),
            )
            .await
            .map_err(|error| {
                format!(
                    "studio aggregate Flight provider failed to build intent response for `{query_text}`: {error:?}"
                )
            }),
            SEARCH_KNOWLEDGE_ROUTE => load_knowledge_search_flight_response(
                Arc::clone(&self.state.studio),
                query_text,
                limit,
            )
            .await
            .map_err(|error| {
                format!(
                    "studio aggregate Flight provider failed to build knowledge response for `{query_text}`: {error:?}"
                )
            }),
            SEARCH_REFERENCES_ROUTE => load_reference_search_flight_response(
                Arc::clone(&self.state),
                ReferenceSearchQuery {
                    q: Some(query_text.to_string()),
                    limit: Some(limit),
                },
            )
            .await
            .map_err(|error| {
                format!(
                    "studio aggregate Flight provider failed to build reference response for `{query_text}`: {error:?}"
                )
            }),
            SEARCH_SYMBOLS_ROUTE => load_symbol_search_flight_response(
                Arc::clone(&self.state),
                SymbolSearchQuery {
                    q: Some(query_text.to_string()),
                    limit: Some(limit),
                },
            )
            .await
            .map_err(|error| {
                format!(
                    "studio aggregate Flight provider failed to build symbol response for `{query_text}`: {error:?}"
                )
            }),
            _ => Err(format!(
                "studio aggregate Flight provider does not support route `{route}`"
            )),
        }
    }
}

#[cfg(feature = "julia")]
pub(crate) fn build_studio_search_flight_service_with_repo_provider(
    expected_schema_version: impl Into<String>,
    repo_search_provider: Arc<dyn RepoSearchFlightRouteProvider>,
    state: Arc<GatewayState>,
    rerank_dimension: usize,
    rerank_weights: RerankScoreWeights,
) -> Result<WendaoFlightService, String> {
    WendaoFlightService::new_with_route_providers(
        expected_schema_version,
        repo_search_provider,
        Some(Arc::new(StudioSearchFlightRouteProvider::new(Arc::clone(
            &state,
        )))),
        Some(Arc::new(StudioAttachmentSearchFlightRouteProvider::new(
            Arc::clone(&state.studio),
        ))),
        Some(Arc::new(StudioAstSearchFlightRouteProvider::new(
            Arc::clone(&state),
        ))),
        Some(Arc::new(StudioDefinitionFlightRouteProvider::new(
            Arc::clone(&state.studio),
        ))),
        Some(Arc::new(StudioAutocompleteFlightRouteProvider::new(
            Arc::clone(&state.studio),
        ))),
        Some(Arc::new(StudioMarkdownAnalysisFlightRouteProvider::new(
            Arc::clone(&state),
        ))),
        Some(Arc::new(StudioCodeAstAnalysisFlightRouteProvider::new(
            Arc::clone(&state),
        ))),
        Some(Arc::new(StudioVfsResolveFlightRouteProvider::new(
            Arc::clone(&state.studio),
        ))),
        Some(Arc::new(StudioGraphNeighborsFlightRouteProvider::new(
            Arc::clone(&state),
        ))),
        rerank_dimension,
        rerank_weights,
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use arrow_flight::FlightDescriptor;
    use arrow_flight::flight_service_server::FlightService;
    use async_trait::async_trait;
    use git2::Repository;
    use serde::Serialize;
    use serde_json::json;
    use tempfile::{TempDir, tempdir};
    use tonic::Request;
    use tonic::metadata::MetadataMap;
    use xiuxian_vector::LanceStringArray;
    use xiuxian_vector::{
        LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema,
    };
    use xiuxian_wendao_runtime::transport::{
        ANALYSIS_CODE_AST_ROUTE, ANALYSIS_MARKDOWN_ROUTE, GRAPH_NEIGHBORS_ROUTE,
        RepoSearchFlightRouteProvider, RerankScoreWeights, SEARCH_AST_ROUTE,
        SEARCH_ATTACHMENTS_ROUTE, SEARCH_AUTOCOMPLETE_ROUTE, SEARCH_DEFINITION_ROUTE,
        SEARCH_INTENT_ROUTE, SEARCH_KNOWLEDGE_ROUTE, SEARCH_REFERENCES_ROUTE, SEARCH_SYMBOLS_ROUTE,
        SearchFlightRouteProvider, VFS_RESOLVE_ROUTE, WENDAO_ANALYSIS_LINE_HEADER,
        WENDAO_ANALYSIS_PATH_HEADER, WENDAO_ANALYSIS_REPO_HEADER,
        WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER,
        WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER, WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER,
        WENDAO_AUTOCOMPLETE_LIMIT_HEADER, WENDAO_AUTOCOMPLETE_PREFIX_HEADER,
        WENDAO_DEFINITION_LINE_HEADER, WENDAO_DEFINITION_PATH_HEADER,
        WENDAO_DEFINITION_QUERY_HEADER, WENDAO_GRAPH_DIRECTION_HEADER, WENDAO_GRAPH_HOPS_HEADER,
        WENDAO_GRAPH_LIMIT_HEADER, WENDAO_GRAPH_NODE_ID_HEADER, WENDAO_SCHEMA_VERSION_HEADER,
        WENDAO_SEARCH_LIMIT_HEADER, WENDAO_SEARCH_QUERY_HEADER, WENDAO_VFS_PATH_HEADER,
        WendaoFlightService, flight_descriptor_path,
    };

    use super::{
        StudioSearchFlightRouteProvider, build_studio_search_flight_service_with_repo_provider,
    };
    use crate::gateway::studio::router::{GatewayState, StudioState};
    use crate::gateway::studio::search::build_symbol_index;
    use crate::gateway::studio::search::handlers::tests::test_studio_state;
    use crate::gateway::studio::types::{UiConfig, UiProjectConfig, UiRepoProjectConfig};

    struct GatewayStateFixture {
        _temp_dir: TempDir,
        state: Arc<GatewayState>,
    }

    fn assert_studio_flight_snapshot(name: &str, value: impl Serialize) {
        insta::with_settings!({
            snapshot_path => concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots/gateway/studio"),
            prepend_module_to_snapshot => false,
            sort_maps => true,
        }, {
            insta::assert_json_snapshot!(name, value);
        });
    }

    fn make_gateway_state_with_docs(docs: &[(&str, &str)]) -> GatewayStateFixture {
        let temp_dir = tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        for (path, contents) in docs {
            let full_path = temp_dir.path().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)
                    .unwrap_or_else(|error| panic!("create fixture dirs for {path}: {error}"));
            }
            fs::write(&full_path, contents)
                .unwrap_or_else(|error| panic!("write fixture doc {path}: {error}"));
        }

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

        GatewayStateFixture {
            _temp_dir: temp_dir,
            state: Arc::new(GatewayState {
                index: None,
                signal_tx: None,
                studio: Arc::new(studio),
            }),
        }
    }

    async fn make_gateway_state_with_search_routes() -> GatewayStateFixture {
        let temp_dir = tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let docs = [
            (
                "docs/alpha.md",
                "# Alpha\n\nIntent keyword: alpha.\n\n![Topology](assets/topology.png)\n",
            ),
            (
                "packages/rust/crates/demo/src/lib.rs",
                "pub struct AlphaService;\npub fn alpha_handler() {}\n",
            ),
        ];
        for (path, contents) in docs {
            let full_path = temp_dir.path().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)
                    .unwrap_or_else(|error| panic!("create fixture dirs for {path}: {error}"));
            }
            fs::write(&full_path, contents)
                .unwrap_or_else(|error| panic!("write fixture doc {path}: {error}"));
        }

        let mut studio = test_studio_state();
        studio.project_root = temp_dir.path().to_path_buf();
        studio.config_root = temp_dir.path().to_path_buf();
        studio.set_ui_config(UiConfig {
            projects: vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string(), "packages".to_string()],
            }],
            repo_projects: Vec::new(),
        });

        let configured_projects = studio.configured_projects();
        let warmed_index = build_symbol_index(
            studio.project_root.as_path(),
            studio.config_root.as_path(),
            configured_projects.as_slice(),
        );
        studio.symbol_index_coordinator.set_ready_index_for_test(
            configured_projects.as_slice(),
            Arc::clone(&studio.symbol_index),
            warmed_index,
        );

        let knowledge_fingerprint = format!(
            "test:knowledge:{}",
            blake3::hash(
                format!(
                    "{}:{}:{}",
                    studio.project_root.display(),
                    studio.config_root.display(),
                    configured_projects.len()
                )
                .as_bytes()
            )
            .to_hex()
        );
        studio
            .search_plane
            .publish_knowledge_sections_from_projects(
                studio.project_root.as_path(),
                studio.config_root.as_path(),
                &configured_projects,
                knowledge_fingerprint.as_str(),
            )
            .await
            .unwrap_or_else(|error| panic!("publish knowledge sections: {error}"));

        let reference_fingerprint = format!(
            "test:reference:{}",
            blake3::hash(
                format!(
                    "{}:{}:{}",
                    studio.project_root.display(),
                    studio.config_root.display(),
                    configured_projects.len()
                )
                .as_bytes()
            )
            .to_hex()
        );
        studio
            .search_plane
            .publish_reference_occurrences_from_projects(
                studio.project_root.as_path(),
                studio.config_root.as_path(),
                &configured_projects,
                reference_fingerprint.as_str(),
            )
            .await
            .unwrap_or_else(|error| panic!("publish reference occurrences: {error}"));

        let attachment_fingerprint = format!(
            "test:attachment:{}",
            blake3::hash(
                format!(
                    "{}:{}:{}",
                    studio.project_root.display(),
                    studio.config_root.display(),
                    configured_projects.len()
                )
                .as_bytes()
            )
            .to_hex()
        );
        studio
            .search_plane
            .publish_attachments_from_projects(
                studio.project_root.as_path(),
                studio.config_root.as_path(),
                &configured_projects,
                attachment_fingerprint.as_str(),
            )
            .await
            .unwrap_or_else(|error| panic!("publish attachments: {error}"));

        GatewayStateFixture {
            _temp_dir: temp_dir,
            state: Arc::new(GatewayState {
                index: None,
                signal_tx: None,
                studio: Arc::new(studio),
            }),
        }
    }

    fn make_gateway_state_with_repo(repo_files: &[(&str, &str)]) -> GatewayStateFixture {
        let temp_dir = tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        Repository::init(temp_dir.path().join("repo"))
            .unwrap_or_else(|error| panic!("init repo fixture: {error}"));
        for (path, contents) in repo_files {
            let full_path = temp_dir.path().join("repo").join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)
                    .unwrap_or_else(|error| panic!("create repo fixture dirs for {path}: {error}"));
            }
            fs::write(&full_path, contents)
                .unwrap_or_else(|error| panic!("write repo fixture {path}: {error}"));
        }

        let mut studio = test_studio_state();
        studio.project_root = temp_dir.path().to_path_buf();
        studio.config_root = temp_dir.path().to_path_buf();
        studio.set_ui_config(UiConfig {
            projects: vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            }],
            repo_projects: vec![UiRepoProjectConfig {
                id: "demo".to_string(),
                root: Some("repo".to_string()),
                url: None,
                git_ref: None,
                refresh: None,
                plugins: vec!["julia".to_string()],
            }],
        });

        GatewayStateFixture {
            _temp_dir: temp_dir,
            state: Arc::new(GatewayState {
                index: None,
                signal_tx: None,
                studio: Arc::new(studio),
            }),
        }
    }

    async fn make_gateway_state_with_attachments() -> GatewayStateFixture {
        let temp_dir = tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        std::fs::create_dir_all(temp_dir.path().join("docs/assets"))
            .unwrap_or_else(|error| panic!("create docs/assets: {error}"));
        std::fs::write(
            temp_dir.path().join("docs/alpha.md"),
            "# Alpha\n\n![Topology](assets/topology.png)\n",
        )
        .unwrap_or_else(|error| panic!("write alpha.md: {error}"));

        let mut studio = test_studio_state();
        studio.project_root = temp_dir.path().to_path_buf();
        studio.config_root = temp_dir.path().to_path_buf();
        studio.set_ui_config(UiConfig {
            projects: vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            }],
            repo_projects: Vec::new(),
        });

        let fingerprint = format!(
            "test:attachment:{}",
            blake3::hash(
                format!(
                    "{}:{}:{}",
                    studio.project_root.display(),
                    studio.config_root.display(),
                    studio.configured_projects().len()
                )
                .as_bytes()
            )
            .to_hex()
        );
        studio
            .search_plane
            .publish_attachments_from_projects(
                studio.project_root.as_path(),
                studio.config_root.as_path(),
                &studio.configured_projects(),
                fingerprint.as_str(),
            )
            .await
            .unwrap_or_else(|error| panic!("publish attachments: {error}"));

        GatewayStateFixture {
            _temp_dir: temp_dir,
            state: Arc::new(GatewayState {
                index: None,
                signal_tx: None,
                studio: Arc::new(studio),
            }),
        }
    }

    fn populate_search_headers(metadata: &mut MetadataMap, query_text: &str, limit: usize) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            "v2".parse()
                .unwrap_or_else(|error| panic!("schema metadata: {error}")),
        );
        metadata.insert(
            WENDAO_SEARCH_QUERY_HEADER,
            query_text
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

    fn populate_attachment_headers(metadata: &mut MetadataMap, query_text: &str, limit: usize) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            "v2".parse()
                .unwrap_or_else(|error| panic!("schema metadata: {error}")),
        );
        metadata.insert(
            WENDAO_SEARCH_QUERY_HEADER,
            query_text
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
        metadata.insert(
            WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER,
            "png"
                .parse()
                .unwrap_or_else(|error| panic!("ext metadata: {error}")),
        );
        metadata.insert(
            WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER,
            "image"
                .parse()
                .unwrap_or_else(|error| panic!("kind metadata: {error}")),
        );
        metadata.insert(
            WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER,
            "false"
                .parse()
                .unwrap_or_else(|error| panic!("case metadata: {error}")),
        );
    }

    fn populate_definition_headers(
        metadata: &mut MetadataMap,
        query_text: &str,
        source_path: &str,
        source_line: usize,
    ) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            "v2".parse()
                .unwrap_or_else(|error| panic!("schema metadata: {error}")),
        );
        metadata.insert(
            WENDAO_DEFINITION_QUERY_HEADER,
            query_text
                .parse()
                .unwrap_or_else(|error| panic!("definition query metadata: {error}")),
        );
        metadata.insert(
            WENDAO_DEFINITION_PATH_HEADER,
            source_path
                .parse()
                .unwrap_or_else(|error| panic!("definition path metadata: {error}")),
        );
        metadata.insert(
            WENDAO_DEFINITION_LINE_HEADER,
            source_line
                .to_string()
                .parse()
                .unwrap_or_else(|error| panic!("definition line metadata: {error}")),
        );
    }

    fn populate_autocomplete_headers(metadata: &mut MetadataMap, prefix: &str, limit: usize) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            "v2".parse()
                .unwrap_or_else(|error| panic!("schema metadata: {error}")),
        );
        metadata.insert(
            WENDAO_AUTOCOMPLETE_PREFIX_HEADER,
            prefix
                .parse()
                .unwrap_or_else(|error| panic!("autocomplete prefix metadata: {error}")),
        );
        metadata.insert(
            WENDAO_AUTOCOMPLETE_LIMIT_HEADER,
            limit
                .to_string()
                .parse()
                .unwrap_or_else(|error| panic!("autocomplete limit metadata: {error}")),
        );
    }

    fn populate_vfs_resolve_headers(metadata: &mut MetadataMap, path: &str) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            "v2".parse()
                .unwrap_or_else(|error| panic!("schema metadata: {error}")),
        );
        metadata.insert(
            WENDAO_VFS_PATH_HEADER,
            path.parse()
                .unwrap_or_else(|error| panic!("VFS path metadata: {error}")),
        );
    }

    fn populate_graph_neighbors_headers(
        metadata: &mut MetadataMap,
        node_id: &str,
        direction: &str,
        hops: usize,
        limit: usize,
    ) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            "v2".parse()
                .unwrap_or_else(|error| panic!("schema metadata: {error}")),
        );
        metadata.insert(
            WENDAO_GRAPH_NODE_ID_HEADER,
            node_id
                .parse()
                .unwrap_or_else(|error| panic!("graph node id metadata: {error}")),
        );
        metadata.insert(
            WENDAO_GRAPH_DIRECTION_HEADER,
            direction
                .parse()
                .unwrap_or_else(|error| panic!("graph direction metadata: {error}")),
        );
        metadata.insert(
            WENDAO_GRAPH_HOPS_HEADER,
            hops.to_string()
                .parse()
                .unwrap_or_else(|error| panic!("graph hops metadata: {error}")),
        );
        metadata.insert(
            WENDAO_GRAPH_LIMIT_HEADER,
            limit
                .to_string()
                .parse()
                .unwrap_or_else(|error| panic!("graph limit metadata: {error}")),
        );
    }

    fn populate_markdown_analysis_headers(metadata: &mut MetadataMap, path: &str) {
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
        metadata: &mut MetadataMap,
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

    #[derive(Debug)]
    struct RecordingRepoSearchProvider;

    #[async_trait]
    impl RepoSearchFlightRouteProvider for RecordingRepoSearchProvider {
        async fn repo_search_batch(
            &self,
            query_text: &str,
            limit: usize,
            _language_filters: &std::collections::HashSet<String>,
            _path_prefixes: &std::collections::HashSet<String>,
            _title_filters: &std::collections::HashSet<String>,
            _tag_filters: &std::collections::HashSet<String>,
            _filename_filters: &std::collections::HashSet<String>,
        ) -> Result<LanceRecordBatch, String> {
            LanceRecordBatch::try_new(
                Arc::new(LanceSchema::new(vec![
                    LanceField::new("doc_id", LanceDataType::Utf8, false),
                    LanceField::new("score", LanceDataType::Float64, false),
                ])),
                vec![
                    Arc::new(LanceStringArray::from(vec![format!(
                        "repo:{query_text}:{limit}"
                    )])) as _,
                    Arc::new(LanceFloat64Array::from(vec![0.99_f64])) as _,
                ],
            )
            .map_err(|error| error.to_string())
        }
    }

    fn first_string(batch: &xiuxian_vector::LanceRecordBatch, column: &str) -> String {
        batch
            .column_by_name(column)
            .unwrap_or_else(|| panic!("missing column `{column}`"))
            .as_any()
            .downcast_ref::<LanceStringArray>()
            .unwrap_or_else(|| panic!("column `{column}` should be utf8"))
            .value(0)
            .to_string()
    }

    async fn snapshot_search_route_contract(
        service: &WendaoFlightService,
        route: &str,
        query_text: &str,
        limit: usize,
    ) -> serde_json::Value {
        let descriptor_path = flight_descriptor_path(route)
            .unwrap_or_else(|error| panic!("descriptor path: {error}"));
        let mut request = Request::new(FlightDescriptor::new_path(descriptor_path.clone()));
        populate_search_headers(request.metadata_mut(), query_text, limit);

        let response = service
            .get_flight_info(request)
            .await
            .unwrap_or_else(|error| panic!("search route `{route}` should resolve: {error}"));
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .unwrap_or_else(|| panic!("search route `{route}` should emit one ticket"));

        json!({
            "route": route,
            "descriptorPath": descriptor_path,
            "query": query_text,
            "limit": limit,
            "ticket": ticket,
            "endpointCount": flight_info.endpoint.len(),
            "schemaLength": flight_info.schema.len(),
        })
    }

    async fn snapshot_attachment_route_contract(
        service: &WendaoFlightService,
        query_text: &str,
        limit: usize,
    ) -> serde_json::Value {
        let descriptor_path = flight_descriptor_path(SEARCH_ATTACHMENTS_ROUTE)
            .unwrap_or_else(|error| panic!("descriptor path: {error}"));
        let mut request = Request::new(FlightDescriptor::new_path(descriptor_path.clone()));
        populate_attachment_headers(request.metadata_mut(), query_text, limit);

        let response = service
            .get_flight_info(request)
            .await
            .unwrap_or_else(|error| panic!("attachment route should resolve: {error}"));
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .unwrap_or_else(|| panic!("attachment route should emit one ticket"));

        json!({
            "route": SEARCH_ATTACHMENTS_ROUTE,
            "descriptorPath": descriptor_path,
            "query": query_text,
            "limit": limit,
            "extFilters": ["png"],
            "kindFilters": ["image"],
            "ticket": ticket,
            "endpointCount": flight_info.endpoint.len(),
            "schemaLength": flight_info.schema.len(),
        })
    }

    async fn snapshot_definition_route_contract(
        service: &WendaoFlightService,
        query_text: &str,
        source_path: &str,
        source_line: usize,
    ) -> serde_json::Value {
        let descriptor_path = flight_descriptor_path(SEARCH_DEFINITION_ROUTE)
            .unwrap_or_else(|error| panic!("descriptor path: {error}"));
        let mut request = Request::new(FlightDescriptor::new_path(descriptor_path.clone()));
        populate_definition_headers(request.metadata_mut(), query_text, source_path, source_line);

        let response = service
            .get_flight_info(request)
            .await
            .unwrap_or_else(|error| panic!("definition route should resolve: {error}"));
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .unwrap_or_else(|| panic!("definition route should emit one ticket"));

        json!({
            "route": SEARCH_DEFINITION_ROUTE,
            "descriptorPath": descriptor_path,
            "query": query_text,
            "sourcePath": source_path,
            "sourceLine": source_line,
            "ticket": ticket,
            "endpointCount": flight_info.endpoint.len(),
            "schemaLength": flight_info.schema.len(),
        })
    }

    async fn snapshot_autocomplete_route_contract(
        service: &WendaoFlightService,
        prefix: &str,
        limit: usize,
    ) -> serde_json::Value {
        let descriptor_path = flight_descriptor_path(SEARCH_AUTOCOMPLETE_ROUTE)
            .unwrap_or_else(|error| panic!("descriptor path: {error}"));
        let mut request = Request::new(FlightDescriptor::new_path(descriptor_path.clone()));
        populate_autocomplete_headers(request.metadata_mut(), prefix, limit);

        let response = service
            .get_flight_info(request)
            .await
            .unwrap_or_else(|error| panic!("autocomplete route should resolve: {error}"));
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .unwrap_or_else(|| panic!("autocomplete route should emit one ticket"));

        json!({
            "route": SEARCH_AUTOCOMPLETE_ROUTE,
            "descriptorPath": descriptor_path,
            "prefix": prefix,
            "limit": limit,
            "ticket": ticket,
            "endpointCount": flight_info.endpoint.len(),
            "schemaLength": flight_info.schema.len(),
        })
    }

    async fn snapshot_vfs_resolve_route_contract(
        service: &WendaoFlightService,
        path: &str,
    ) -> serde_json::Value {
        let descriptor_path = flight_descriptor_path(VFS_RESOLVE_ROUTE)
            .unwrap_or_else(|error| panic!("descriptor path: {error}"));
        let mut request = Request::new(FlightDescriptor::new_path(descriptor_path.clone()));
        populate_vfs_resolve_headers(request.metadata_mut(), path);

        let response = service
            .get_flight_info(request)
            .await
            .unwrap_or_else(|error| panic!("VFS resolve route should resolve: {error}"));
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .unwrap_or_else(|| panic!("VFS resolve route should emit one ticket"));

        json!({
            "route": VFS_RESOLVE_ROUTE,
            "descriptorPath": descriptor_path,
            "path": path,
            "ticket": ticket,
            "endpointCount": flight_info.endpoint.len(),
            "schemaLength": flight_info.schema.len(),
        })
    }

    async fn snapshot_graph_neighbors_route_contract(
        service: &WendaoFlightService,
        node_id: &str,
        direction: &str,
        hops: usize,
        limit: usize,
    ) -> serde_json::Value {
        let descriptor_path = flight_descriptor_path(GRAPH_NEIGHBORS_ROUTE)
            .unwrap_or_else(|error| panic!("descriptor path: {error}"));
        let mut request = Request::new(FlightDescriptor::new_path(descriptor_path.clone()));
        populate_graph_neighbors_headers(request.metadata_mut(), node_id, direction, hops, limit);

        let response = service
            .get_flight_info(request)
            .await
            .unwrap_or_else(|error| panic!("graph-neighbors route should resolve: {error}"));
        let flight_info = response.into_inner();
        let ticket = flight_info
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .unwrap_or_else(|| panic!("graph-neighbors route should emit one ticket"));

        json!({
            "route": GRAPH_NEIGHBORS_ROUTE,
            "descriptorPath": descriptor_path,
            "nodeId": node_id,
            "direction": direction,
            "hops": hops,
            "limit": limit,
            "ticket": ticket,
            "endpointCount": flight_info.endpoint.len(),
            "schemaLength": flight_info.schema.len(),
        })
    }

    #[tokio::test]
    async fn studio_search_flight_provider_dispatches_symbol_route() {
        let fixture = make_gateway_state_with_docs(&[(
            "packages/rust/crates/demo/src/lib.rs",
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
        )]);
        let provider = StudioSearchFlightRouteProvider::new(Arc::clone(&fixture.state));

        let batch = provider
            .search_batch(SEARCH_SYMBOLS_ROUTE, "alpha", 5, None, None)
            .await
            .expect("symbol route should succeed");

        assert!(batch.batch.num_rows() >= 2);
        assert_eq!(first_string(&batch.batch, "name"), "AlphaService");
    }

    #[tokio::test]
    async fn studio_search_flight_provider_dispatches_knowledge_route() {
        let fixture = make_gateway_state_with_search_routes().await;
        let provider = StudioSearchFlightRouteProvider::new(Arc::clone(&fixture.state));

        let batch = provider
            .search_batch(SEARCH_KNOWLEDGE_ROUTE, "alpha", 5, None, None)
            .await
            .expect("knowledge route should succeed");

        assert!(batch.batch.num_rows() >= 1);
        assert_eq!(first_string(&batch.batch, "stem"), "alpha");
        let app_metadata: serde_json::Value = serde_json::from_slice(&batch.app_metadata)
            .expect("knowledge app_metadata should decode");
        assert_eq!(app_metadata["query"], "alpha");
        assert_eq!(app_metadata["hitCount"], 1);
    }

    #[tokio::test]
    async fn studio_search_flight_provider_rejects_unknown_routes() {
        let provider = StudioSearchFlightRouteProvider::new(Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(StudioState::new_with_bootstrap_ui_config(Arc::new(
                crate::analyzers::bootstrap_builtin_registry()
                    .unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
            ))),
        }));

        let error = provider
            .search_batch("/search/unknown", "alpha", 5, None, None)
            .await
            .expect_err("unknown route should be rejected");

        assert!(
            error.contains("/search/unknown"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn build_studio_search_flight_service_wires_search_routes() {
        let fixture = make_gateway_state_with_docs(&[(
            "packages/rust/crates/demo/src/lib.rs",
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
        )]);
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_SYMBOLS_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_search_headers(request.metadata_mut(), "alpha", 5);

        let response = service
            .get_flight_info(request)
            .await
            .expect("search route should resolve through studio builder");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("search route should emit one ticket");

        assert_eq!(ticket, SEARCH_SYMBOLS_ROUTE);
    }

    #[tokio::test]
    async fn build_studio_search_flight_service_wires_attachment_routes() {
        let fixture = make_gateway_state_with_attachments().await;
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_ATTACHMENTS_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_attachment_headers(request.metadata_mut(), "topology", 5);

        let response = service
            .get_flight_info(request)
            .await
            .expect("attachment route should resolve through studio builder");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("attachment route should emit one ticket");

        assert_eq!(ticket, SEARCH_ATTACHMENTS_ROUTE);
    }

    #[tokio::test]
    async fn build_studio_search_flight_service_wires_ast_routes() {
        let fixture = make_gateway_state_with_docs(&[(
            "packages/rust/crates/demo/src/lib.rs",
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
        )]);
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_AST_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_search_headers(request.metadata_mut(), "alpha", 5);

        let response = service
            .get_flight_info(request)
            .await
            .expect("AST route should resolve through studio builder");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("AST route should emit one ticket");

        assert_eq!(ticket, SEARCH_AST_ROUTE);
    }

    #[tokio::test]
    async fn build_studio_search_flight_service_wires_definition_routes() {
        let fixture = make_gateway_state_with_docs(&[
            (
                "packages/rust/crates/demo/src/lib.rs",
                "pub fn build_service() {\n    let _service = AlphaService::new();\n}\n",
            ),
            (
                "packages/rust/crates/demo/src/service.rs",
                "pub struct AlphaService {\n    ready: bool,\n}\n",
            ),
        ]);
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_DEFINITION_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_definition_headers(
            request.metadata_mut(),
            "AlphaService",
            "packages/rust/crates/demo/src/lib.rs",
            2,
        );

        let response = service
            .get_flight_info(request)
            .await
            .expect("definition route should resolve through studio builder");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("definition route should emit one ticket");

        assert_eq!(ticket, SEARCH_DEFINITION_ROUTE);
    }

    #[tokio::test]
    async fn build_studio_search_flight_service_wires_autocomplete_routes() {
        let fixture = make_gateway_state_with_docs(&[(
            "packages/rust/crates/demo/src/lib.rs",
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
        )]);
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_AUTOCOMPLETE_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_autocomplete_headers(request.metadata_mut(), "Alpha", 5);

        let response = service
            .get_flight_info(request)
            .await
            .expect("autocomplete route should resolve through studio builder");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("autocomplete route should emit one ticket");

        assert_eq!(ticket, SEARCH_AUTOCOMPLETE_ROUTE);
    }

    #[tokio::test]
    async fn build_studio_search_flight_service_wires_vfs_resolve_routes() {
        let fixture = make_gateway_state_with_docs(&[(
            "docs/index.md",
            "# Index\n\n- [Overview](overview.md)\n",
        )]);
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(VFS_RESOLVE_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_vfs_resolve_headers(request.metadata_mut(), "docs/index.md");

        let response = service
            .get_flight_info(request)
            .await
            .expect("VFS resolve route should resolve through studio builder");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("VFS resolve route should emit one ticket");

        assert_eq!(ticket, VFS_RESOLVE_ROUTE);
    }

    #[tokio::test]
    async fn build_studio_search_flight_service_wires_markdown_analysis_routes() {
        let fixture = make_gateway_state_with_docs(&[(
            "docs/analysis.md",
            "# Analysis Kernel\n\n## Inputs\n- [ ] Parse markdown\n",
        )]);
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(ANALYSIS_MARKDOWN_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_markdown_analysis_headers(request.metadata_mut(), "docs/analysis.md");

        let response = service
            .get_flight_info(request)
            .await
            .expect("markdown analysis route should resolve through studio builder");
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
    async fn build_studio_search_flight_service_wires_code_ast_analysis_routes() {
        let fixture = make_gateway_state_with_repo(&[
            (
                "Project.toml",
                "name = \"Demo\"\nuuid = \"00000000-0000-0000-0000-000000000001\"\n",
            ),
            (
                "src/lib.jl",
                "module Demo\nexport solve\nsolve(x) = x + 1\nend\n",
            ),
        ]);
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(ANALYSIS_CODE_AST_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_code_ast_analysis_headers(request.metadata_mut(), "src/lib.jl", "demo", Some(3));

        let response = service
            .get_flight_info(request)
            .await
            .expect("code-AST analysis route should resolve through studio builder");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("code-AST analysis route should emit one ticket");

        assert_eq!(ticket, ANALYSIS_CODE_AST_ROUTE);
    }

    #[tokio::test]
    async fn build_studio_search_flight_service_wires_graph_neighbors_routes() {
        let fixture = make_gateway_state_with_docs(&[
            ("docs/alpha.md", "# Alpha\n\nSee [[beta]].\n"),
            ("docs/beta.md", "# Beta\n\nBody.\n"),
        ]);
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(GRAPH_NEIGHBORS_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_graph_neighbors_headers(
            request.metadata_mut(),
            "kernel/docs/alpha.md",
            "both",
            1,
            20,
        );

        let response = service
            .get_flight_info(request)
            .await
            .expect("graph-neighbors route should resolve through studio builder");
        let ticket = response
            .into_inner()
            .endpoint
            .first()
            .and_then(|endpoint| endpoint.ticket.as_ref())
            .map(|ticket| String::from_utf8_lossy(&ticket.ticket.to_vec()).into_owned())
            .expect("graph-neighbors route should emit one ticket");

        assert_eq!(ticket, GRAPH_NEIGHBORS_ROUTE);
    }

    #[tokio::test]
    async fn build_studio_search_flight_service_snapshots_search_route_contracts() {
        let fixture = make_gateway_state_with_search_routes().await;
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));

        let snapshot = json!([
            snapshot_search_route_contract(&service, SEARCH_INTENT_ROUTE, "alpha", 5).await,
            snapshot_search_route_contract(&service, SEARCH_KNOWLEDGE_ROUTE, "Alpha body", 5).await,
            snapshot_attachment_route_contract(&service, "topology", 5).await,
            snapshot_search_route_contract(&service, SEARCH_REFERENCES_ROUTE, "AlphaService", 5)
                .await,
            snapshot_search_route_contract(&service, SEARCH_SYMBOLS_ROUTE, "alpha", 5).await,
            snapshot_search_route_contract(&service, SEARCH_AST_ROUTE, "alpha", 5).await,
            snapshot_definition_route_contract(
                &service,
                "AlphaService",
                "packages/rust/crates/demo/src/lib.rs",
                2,
            )
            .await,
            snapshot_autocomplete_route_contract(&service, "Alpha", 5).await,
        ]);
        assert_studio_flight_snapshot("search_flight_service_route_contracts", snapshot);
    }

    #[tokio::test]
    async fn build_studio_search_flight_service_snapshots_workspace_route_contracts() {
        let fixture = make_gateway_state_with_search_routes().await;
        let service = build_studio_search_flight_service_with_repo_provider(
            "v2",
            Arc::new(RecordingRepoSearchProvider),
            Arc::clone(&fixture.state),
            3,
            RerankScoreWeights::default(),
        )
        .unwrap_or_else(|error| panic!("build studio flight service: {error}"));

        let snapshot = json!([
            snapshot_vfs_resolve_route_contract(&service, "docs/alpha.md").await,
            snapshot_graph_neighbors_route_contract(
                &service,
                "kernel/docs/alpha.md",
                "both",
                1,
                20,
            )
            .await,
        ]);
        assert_studio_flight_snapshot("workspace_flight_service_route_contracts", snapshot);
    }
}
