use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tempfile::tempdir;

use crate::analyzers::{
    ExampleRecord, ModuleRecord, RepoSymbolKind, RepositoryAnalysisOutput, SymbolRecord,
};
use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::gateway::studio::test_support::{assert_wendao_json_snapshot, round_f64};
use crate::link_graph::LinkGraphIndex;
use crate::query_core::context::{GraphBackend, RetrievalBackend, WendaoExecutionContext};
use crate::query_core::execute::{
    SearchPlaneRetrievalBackend, execute_column_mask, execute_graph_neighbors,
    execute_payload_fetch, execute_vector_search,
};
use crate::query_core::graph::graph_projection_from_relation;
use crate::query_core::operators::{
    ColumnMaskOp, ColumnMaskPredicate, GraphDirection, GraphNeighborsOp, PayloadFetchOp,
    RetrievalCorpus, VectorSearchOp,
};
use crate::query_core::service::{query_graph_neighbors_projection, query_repo_code_relation};
use crate::query_core::telemetry::InMemoryWendaoExplainSink;
use crate::query_core::{
    WendaoBackendKind, WendaoExplainEvent, WendaoOperatorKind, WendaoQueryCoreError,
    WendaoRelation, explain_events_summary,
};
use crate::search_plane::{SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneService};

fn repo_document(
    path: &str,
    contents: &str,
    size_bytes: u64,
    modified_unix_ms: u64,
) -> RepoCodeDocument {
    RepoCodeDocument {
        path: path.to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from(contents),
        size_bytes,
        modified_unix_ms,
    }
}

fn sample_repo_analysis(repo_id: &str) -> RepositoryAnalysisOutput {
    RepositoryAnalysisOutput {
        modules: vec![ModuleRecord {
            repo_id: repo_id.to_string(),
            module_id: "module:BaseModelica".to_string(),
            qualified_name: "BaseModelica".to_string(),
            path: "src/BaseModelica.jl".to_string(),
        }],
        symbols: vec![SymbolRecord {
            repo_id: repo_id.to_string(),
            symbol_id: "symbol:reexport".to_string(),
            module_id: Some("module:BaseModelica".to_string()),
            name: "reexport".to_string(),
            qualified_name: "BaseModelica.reexport".to_string(),
            kind: RepoSymbolKind::Function,
            path: "src/BaseModelica.jl".to_string(),
            line_start: Some(7),
            line_end: Some(9),
            signature: Some("reexport()".to_string()),
            audit_status: Some("verified".to_string()),
            verification_state: Some("verified".to_string()),
            attributes: std::collections::BTreeMap::new(),
        }],
        examples: vec![ExampleRecord {
            repo_id: repo_id.to_string(),
            example_id: "example:reexport".to_string(),
            title: "Reexport example".to_string(),
            path: "examples/reexport.jl".to_string(),
            summary: Some("Shows how to reexport ModelingToolkit".to_string()),
        }],
        ..RepositoryAnalysisOutput::default()
    }
}

fn sample_repo_documents() -> Vec<RepoCodeDocument> {
    vec![
        repo_document(
            "src/BaseModelica.jl",
            "module BaseModelica\nexport reexport\nreexport() = nothing\nend\n",
            61,
            10,
        ),
        RepoCodeDocument {
            path: "examples/reexport.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from("using BaseModelica\nreexport()\n"),
            size_bytes: 29,
            modified_unix_ms: 10,
        },
    ]
}

fn snapshot_retrieval_rows(relation: &WendaoRelation) -> Vec<serde_json::Value> {
    relation
        .batches()
        .iter()
        .flat_map(|batch| {
            xiuxian_vector::retrieval_rows_from_record_batch(batch)
                .expect("decode retrieval rows")
                .into_iter()
                .map(|row| {
                    serde_json::json!({
                        "id": row.id,
                        "path": row.path,
                        "repo": row.repo,
                        "title": row.title,
                        "score": row.score.map(round_f64),
                        "source": row.source,
                        "snippet": row.snippet,
                        "doc_type": row.doc_type,
                        "match_reason": row.match_reason,
                        "best_section": row.best_section,
                        "language": row.language,
                        "line": row.line,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn vector_search_op_defaults_are_stable() {
    let op = VectorSearchOp::default();
    assert_eq!(op.limit, 10);
    assert_eq!(op.corpus, RetrievalCorpus::RepoContent);
    assert!(op.repo_id.is_empty());
    assert!(op.search_term.is_empty());
    assert!(op.kind_filters.is_empty());
}

#[test]
fn graph_neighbors_op_defaults_are_stable() {
    let op = GraphNeighborsOp::default();
    assert_eq!(op.direction, GraphDirection::Both);
    assert_eq!(op.hops, 1);
    assert_eq!(op.limit, 20);
}

#[test]
fn explain_events_summary_captures_operator_and_row_counts() {
    let summary = explain_events_summary(&[WendaoExplainEvent {
        operator_kind: WendaoOperatorKind::GraphNeighbors,
        backend_kind: WendaoBackendKind::LinkGraphBackend,
        legacy_adapter: true,
        input_row_count: Some(1),
        output_row_count: Some(3),
        payload_fetch: false,
        narrow_phase_surviving_count: None,
        payload_phase_fetched_count: None,
        note: Some("link-graph backend".to_string()),
    }]);

    assert!(summary.contains("operator=GraphNeighbors"));
    assert!(summary.contains("backend=LinkGraphBackend"));
    assert!(summary.contains("rows=1->3"));
}

struct StubGraphBackend {
    relation: WendaoRelation,
}

#[async_trait]
impl GraphBackend for StubGraphBackend {
    async fn graph_neighbors(
        &self,
        _op: &GraphNeighborsOp,
    ) -> Result<WendaoRelation, WendaoQueryCoreError> {
        Ok(self.relation.clone())
    }
}

struct StubPayloadRetrievalBackend;

#[async_trait]
impl RetrievalBackend for StubPayloadRetrievalBackend {
    async fn vector_search(
        &self,
        _op: &VectorSearchOp,
    ) -> Result<WendaoRelation, WendaoQueryCoreError> {
        Err(WendaoQueryCoreError::Backend(
            "stub payload backend does not implement vector_search".to_string(),
        ))
    }

    async fn payload_fetch(
        &self,
        relation: &WendaoRelation,
        op: &PayloadFetchOp,
    ) -> Result<WendaoRelation, WendaoQueryCoreError> {
        let batches = relation
            .batches()
            .iter()
            .map(|batch| {
                xiuxian_vector::payload_fetch_record_batch(batch, &op.columns, op.ids.as_ref())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let schema = batches
            .first()
            .map(|batch| batch.schema())
            .ok_or_else(|| WendaoQueryCoreError::InvalidRelation("missing payload batch".into()))?;
        Ok(WendaoRelation::new(schema, batches))
    }
}

#[tokio::test]
async fn vector_search_routes_through_search_plane_adapter_and_returns_relation() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let service = Arc::new(SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:query-core"),
        SearchMaintenancePolicy::default(),
    ));
    service
        .publish_repo_content_chunks_with_revision(
            "alpha/repo",
            &[repo_document("src/lib.rs", "fn alpha() {}\n", 14, 10)],
            Some("rev-1"),
        )
        .await
        .expect("publish repo content");

    let telemetry = Arc::new(InMemoryWendaoExplainSink::new());
    let ctx = WendaoExecutionContext::default()
        .with_retrieval_backend(Arc::new(SearchPlaneRetrievalBackend::new(Arc::clone(
            &service,
        ))))
        .with_explain_sink(telemetry.clone());
    let relation = execute_vector_search(
        &ctx,
        &VectorSearchOp {
            corpus: RetrievalCorpus::RepoContent,
            repo_id: "alpha/repo".to_string(),
            search_term: "alpha".to_string(),
            language_filters: HashSet::new(),
            kind_filters: HashSet::new(),
            limit: 5,
        },
    )
    .await
    .expect("execute vector search");

    assert_eq!(relation.row_count(), 1);
    let events = telemetry.events();
    assert_eq!(events.len(), 1);
    assert!(events[0].legacy_adapter);
}

#[tokio::test]
async fn graph_neighbors_routes_through_link_graph_adapter_and_returns_relation() {
    let batch = arrow::record_batch::RecordBatch::try_new(
        Arc::new(arrow::datatypes::Schema::new(vec![
            arrow::datatypes::Field::new("node_id", arrow::datatypes::DataType::Utf8, false),
            arrow::datatypes::Field::new("path", arrow::datatypes::DataType::Utf8, false),
            arrow::datatypes::Field::new("title", arrow::datatypes::DataType::Utf8, true),
            arrow::datatypes::Field::new("distance", arrow::datatypes::DataType::UInt64, false),
            arrow::datatypes::Field::new("direction", arrow::datatypes::DataType::Utf8, false),
        ])),
        vec![
            Arc::new(arrow::array::StringArray::from(vec!["alpha", "beta"]))
                as arrow::array::ArrayRef,
            Arc::new(arrow::array::StringArray::from(vec!["alpha.md", "beta.md"]))
                as arrow::array::ArrayRef,
            Arc::new(arrow::array::StringArray::from(vec![
                Some("Alpha"),
                Some("Beta"),
            ])) as arrow::array::ArrayRef,
            Arc::new(arrow::array::UInt64Array::from(vec![0, 1])) as arrow::array::ArrayRef,
            Arc::new(arrow::array::StringArray::from(vec!["center", "both"]))
                as arrow::array::ArrayRef,
        ],
    )
    .expect("graph batch");
    let relation = WendaoRelation::new(batch.schema(), vec![batch]);
    let telemetry = Arc::new(InMemoryWendaoExplainSink::new());
    let ctx = WendaoExecutionContext::default()
        .with_graph_backend(Arc::new(StubGraphBackend { relation }))
        .with_explain_sink(telemetry.clone());

    let relation = execute_graph_neighbors(
        &ctx,
        &GraphNeighborsOp {
            node_id: "alpha.md".to_string(),
            direction: GraphDirection::Both,
            hops: 1,
            limit: 10,
        },
    )
    .await
    .expect("graph neighbors");

    assert!(relation.row_count() >= 2);
    let events = telemetry.events();
    assert_eq!(events.len(), 1);
    assert!(events[0].legacy_adapter);
}

#[tokio::test]
async fn column_mask_filters_before_payload_fetch_and_emits_phase_counts() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let service = Arc::new(SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:query-core-mask"),
        SearchMaintenancePolicy::default(),
    ));
    service
        .publish_repo_content_chunks_with_revision(
            "alpha/repo",
            &[
                repo_document("src/lib.rs", "fn alpha() {}\n", 14, 10),
                repo_document("src/util.rs", "fn beta() {}\n", 13, 10),
            ],
            Some("rev-1"),
        )
        .await
        .expect("publish repo content");

    let telemetry = Arc::new(InMemoryWendaoExplainSink::new());
    let ctx = WendaoExecutionContext::default()
        .with_retrieval_backend(Arc::new(SearchPlaneRetrievalBackend::new(Arc::clone(
            &service,
        ))))
        .with_explain_sink(telemetry.clone());
    let relation = execute_vector_search(
        &ctx,
        &VectorSearchOp {
            corpus: RetrievalCorpus::RepoContent,
            repo_id: "alpha/repo".to_string(),
            search_term: "fn".to_string(),
            language_filters: HashSet::new(),
            kind_filters: HashSet::new(),
            limit: 10,
        },
    )
    .await
    .expect("execute vector search");

    let masked = execute_column_mask(
        &ctx,
        &ColumnMaskOp {
            relation,
            predicates: vec![ColumnMaskPredicate::PathContains("util".to_string())],
            limit: Some(1),
        },
    )
    .expect("column mask");
    assert_eq!(masked.row_count(), 1);

    let fetched = execute_payload_fetch(
        &ctx,
        &PayloadFetchOp {
            relation: masked,
            columns: vec!["id".to_string(), "path".to_string()],
            ids: Some(BTreeSet::from(["src/util.rs".to_string()])),
        },
    )
    .await
    .expect("payload fetch");
    assert_eq!(fetched.row_count(), 0);

    let events = telemetry.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[1].narrow_phase_surviving_count, Some(1));
    assert_eq!(events[2].payload_phase_fetched_count, Some(0));
}

#[tokio::test]
async fn payload_fetch_projects_requested_columns() {
    let telemetry = Arc::new(InMemoryWendaoExplainSink::new());
    let ctx = WendaoExecutionContext::default().with_explain_sink(telemetry.clone());
    let batch = xiuxian_vector::retrieval_rows_to_record_batch(&[xiuxian_vector::RetrievalRow {
        id: "alpha".to_string(),
        path: "src/lib.rs".to_string(),
        repo: Some("alpha/repo".to_string()),
        title: Some("Alpha".to_string()),
        score: Some(0.9),
        source: "test".to_string(),
        snippet: Some("fn alpha()".to_string()),
        doc_type: Some("file".to_string()),
        match_reason: Some("repo_content_search".to_string()),
        best_section: Some("3: fn alpha()".to_string()),
        language: Some("rust".to_string()),
        line: Some(3),
    }])
    .expect("build retrieval batch");
    let relation = crate::query_core::WendaoRelation::new(batch.schema(), vec![batch]);
    let backend = Arc::new(StubPayloadRetrievalBackend);
    let ctx = ctx.with_retrieval_backend(backend);

    let fetched = execute_payload_fetch(
        &ctx,
        &PayloadFetchOp {
            relation,
            columns: vec!["id".to_string(), "path".to_string()],
            ids: None,
        },
    )
    .await
    .expect("payload fetch");
    let field_names = fetched
        .schema()
        .fields()
        .iter()
        .map(|field| field.name().as_str())
        .collect::<Vec<_>>();
    assert_eq!(field_names, vec!["id", "path"]);
}

#[tokio::test]
async fn query_repo_code_relation_prefers_repo_entity_corpus() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:query-core-repo-code-entity"),
        SearchMaintenancePolicy::default(),
    );
    service
        .publish_repo_entities_with_revision(
            "alpha/repo",
            &sample_repo_analysis("alpha/repo"),
            &sample_repo_documents(),
            Some("rev-1"),
        )
        .await
        .expect("publish repo entities");
    service
        .publish_repo_content_chunks_with_revision(
            "alpha/repo",
            &sample_repo_documents(),
            Some("rev-1"),
        )
        .await
        .expect("publish repo content");

    let telemetry = Arc::new(InMemoryWendaoExplainSink::new());
    let result = query_repo_code_relation(
        &service,
        "alpha/repo",
        "reexport",
        &HashSet::new(),
        &HashSet::new(),
        true,
        true,
        10,
        Some(telemetry.clone()),
    )
    .await
    .expect("query repo code relation");

    assert_eq!(result.corpus, RetrievalCorpus::RepoEntity);
    assert!(result.relation.row_count() > 0);
    assert_wendao_json_snapshot(
        "query_core_repo_code_relation_prefers_repo_entity_corpus",
        serde_json::json!({
            "corpus": format!("{:?}", result.corpus),
            "rows": snapshot_retrieval_rows(&result.relation),
        }),
    );
    let events = telemetry.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].operator_kind, WendaoOperatorKind::VectorSearch);
    assert_eq!(events[1].operator_kind, WendaoOperatorKind::ColumnMask);
    assert_eq!(events[2].operator_kind, WendaoOperatorKind::PayloadFetch);
}

#[tokio::test]
async fn query_repo_code_relation_falls_back_to_repo_content_when_entity_lane_is_disabled() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:query-core-repo-code-content"),
        SearchMaintenancePolicy::default(),
    );
    service
        .publish_repo_content_chunks_with_revision(
            "alpha/repo",
            &[RepoCodeDocument {
                path: "src/BaseModelica.jl".to_string(),
                language: Some("julia".to_string()),
                contents: Arc::<str>::from(
                    "module BaseModelica\nusing Reexport\n@reexport using ModelingToolkit\nend\n",
                ),
                size_bytes: 67,
                modified_unix_ms: 0,
            }],
            Some("rev-1"),
        )
        .await
        .expect("publish repo content");

    let telemetry = Arc::new(InMemoryWendaoExplainSink::new());
    let result = query_repo_code_relation(
        &service,
        "alpha/repo",
        "@reexport",
        &HashSet::new(),
        &HashSet::new(),
        false,
        true,
        10,
        Some(telemetry.clone()),
    )
    .await
    .expect("query repo code relation");

    assert_eq!(result.corpus, RetrievalCorpus::RepoContent);
    assert_eq!(result.relation.row_count(), 1);
    let rows = xiuxian_vector::retrieval_rows_from_record_batch(&result.relation.batches()[0])
        .expect("decode retrieval rows");
    assert_eq!(rows[0].path, "src/BaseModelica.jl");
    assert_wendao_json_snapshot(
        "query_core_repo_code_relation_falls_back_to_repo_content",
        serde_json::json!({
            "corpus": format!("{:?}", result.corpus),
            "rows": snapshot_retrieval_rows(&result.relation),
        }),
    );
    let events = telemetry.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[2].operator_kind, WendaoOperatorKind::PayloadFetch);
}

#[tokio::test]
async fn query_graph_neighbors_projection_returns_nodes_and_links() {
    let root = tempdir().expect("tempdir");
    std::fs::write(root.path().join("alpha.md"), "# Alpha\n\nSee [[beta]].\n")
        .expect("write alpha");
    std::fs::write(root.path().join("beta.md"), "# Beta\n\nBody.\n").expect("write beta");

    let index = Arc::new(LinkGraphIndex::build(root.path()).expect("build link graph"));
    let projection = query_graph_neighbors_projection(
        Arc::clone(&index),
        "alpha",
        GraphDirection::Both,
        1,
        10,
        None,
    )
    .await
    .expect("query graph neighbors projection");

    assert_eq!(projection.center.path, "alpha.md");
    assert!(projection.nodes.iter().any(|node| node.path == "beta.md"));
    assert!(
        projection
            .links
            .iter()
            .any(|link| { link.source_path == "alpha.md" && link.target_path == "beta.md" })
    );
    assert_wendao_json_snapshot("query_core_graph_neighbors_projection", &projection);
}

#[test]
fn graph_projection_from_relation_extracts_unique_paths_by_distance() {
    let root = tempdir().expect("tempdir");
    std::fs::write(root.path().join("alpha.md"), "# Alpha\n\nSee [[beta]].\n")
        .expect("write alpha");
    std::fs::write(root.path().join("beta.md"), "# Beta\n\nBody.\n").expect("write beta");
    let index = LinkGraphIndex::build(root.path()).expect("build link graph");

    let batch = arrow::record_batch::RecordBatch::try_new(
        Arc::new(arrow::datatypes::Schema::new(vec![
            arrow::datatypes::Field::new("node_id", arrow::datatypes::DataType::Utf8, false),
            arrow::datatypes::Field::new("path", arrow::datatypes::DataType::Utf8, false),
            arrow::datatypes::Field::new("title", arrow::datatypes::DataType::Utf8, true),
            arrow::datatypes::Field::new("distance", arrow::datatypes::DataType::UInt64, false),
            arrow::datatypes::Field::new("direction", arrow::datatypes::DataType::Utf8, false),
        ])),
        vec![
            Arc::new(arrow::array::StringArray::from(vec![
                "alpha", "beta", "beta",
            ])) as arrow::array::ArrayRef,
            Arc::new(arrow::array::StringArray::from(vec![
                "alpha.md", "beta.md", "beta.md",
            ])) as arrow::array::ArrayRef,
            Arc::new(arrow::array::StringArray::from(vec![
                Some("Alpha"),
                Some("Beta"),
                Some("Beta"),
            ])) as arrow::array::ArrayRef,
            Arc::new(arrow::array::UInt64Array::from(vec![0, 1, 1])) as arrow::array::ArrayRef,
            Arc::new(arrow::array::StringArray::from(vec![
                "center", "both", "both",
            ])) as arrow::array::ArrayRef,
        ],
    )
    .expect("graph batch");
    let relation = WendaoRelation::new(batch.schema(), vec![batch]);
    let projection = graph_projection_from_relation(&index, &relation).expect("graph projection");

    assert_eq!(projection.nodes.len(), 2);
    assert_eq!(
        projection.paths_at_distance(Some(1)),
        vec!["beta.md".to_string()]
    );
}
