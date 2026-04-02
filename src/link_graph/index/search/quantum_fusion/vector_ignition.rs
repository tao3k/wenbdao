use super::semantic_ignition::{QuantumSemanticIgnition, QuantumSemanticIgnitionFuture};
#[cfg(feature = "julia")]
use crate::analyzers::{
    PluginArrowRequestRow, RepoIntelligenceError, build_plugin_arrow_request_batch,
};
use crate::link_graph::models::{QuantumAnchorHit, QuantumSemanticSearchRequest};
#[cfg(feature = "julia")]
use arrow::record_batch::RecordBatch;
#[cfg(feature = "julia")]
use thiserror::Error;
use xiuxian_vector::{SearchOptions, VectorStore, VectorStoreError, distance_to_score};

/// Semantic ignition adapter backed by the Rust vector store.
#[derive(Clone)]
pub struct VectorStoreSemanticIgnition {
    store: VectorStore,
    table_name: String,
    search_options: SearchOptions,
    backend_name: String,
}

impl VectorStoreSemanticIgnition {
    /// Create a vector-backed ignition adapter for the given table.
    pub fn new(store: VectorStore, table_name: impl Into<String>) -> Self {
        Self {
            store,
            table_name: table_name.into(),
            search_options: SearchOptions::default(),
            backend_name: "xiuxian-vector".to_string(),
        }
    }

    /// Override the search options passed to the vector store.
    #[must_use]
    pub fn with_search_options(mut self, options: SearchOptions) -> Self {
        self.search_options = options;
        self
    }

    /// Override the backend name surfaced in telemetry.
    #[must_use]
    pub fn with_backend_name(mut self, backend_name: impl Into<String>) -> Self {
        self.backend_name = backend_name.into();
        self
    }

    /// Build a WendaoArrow `v1` plugin rerank request batch for the provided
    /// anchors.
    ///
    /// The request reuses `anchor_id` as the stable `doc_id` field because the
    /// quantum-fusion candidate identity is anchor-granular, not document-granular.
    ///
    /// # Errors
    ///
    /// Returns [`VectorStoreJuliaRequestError`] when candidate
    /// embeddings cannot
    /// be fetched from the vector store or the WendaoArrow request batch cannot
    /// be assembled.
    #[cfg(feature = "julia")]
    pub async fn build_plugin_rerank_request_batch(
        &self,
        request: QuantumSemanticSearchRequest<'_>,
        anchors: &[QuantumAnchorHit],
    ) -> Result<RecordBatch, VectorStorePluginRerankRequestError> {
        let query_vector = request.query_vector;
        if anchors.is_empty() {
            return Err(VectorStorePluginRerankRequestError::Build(
                RepoIntelligenceError::AnalysisFailed {
                    message: "cannot build Julia rerank request from an empty anchor set"
                        .to_string(),
                },
            ));
        }

        let ids = anchors
            .iter()
            .map(|anchor| anchor.anchor_id.clone())
            .collect::<Vec<_>>();
        let embeddings = self
            .store
            .fetch_embeddings_by_ids(self.table_name.as_str(), &ids)
            .await
            .map_err(VectorStorePluginRerankRequestError::VectorStore)?;

        let mut rows = Vec::with_capacity(anchors.len());
        for anchor in anchors {
            let embedding = embeddings
                .get(anchor.anchor_id.as_str())
                .cloned()
                .ok_or_else(|| VectorStorePluginRerankRequestError::MissingEmbedding {
                    anchor_id: anchor.anchor_id.clone(),
                })?;
            rows.push(PluginArrowRequestRow {
                doc_id: anchor.anchor_id.clone(),
                vector_score: anchor.vector_score,
                embedding,
            });
        }

        build_plugin_arrow_request_batch(&rows, query_vector)
            .map_err(VectorStorePluginRerankRequestError::Build)
    }

    /// Compatibility shim for the legacy Julia-named rerank request builder.
    ///
    /// # Errors
    ///
    /// Returns [`VectorStorePluginRerankRequestError`] when candidate
    /// embeddings cannot
    /// be fetched from the vector store or the WendaoArrow request batch cannot
    /// be assembled.
    #[cfg(feature = "julia")]
    pub async fn build_julia_rerank_request_batch(
        &self,
        request: QuantumSemanticSearchRequest<'_>,
        anchors: &[QuantumAnchorHit],
    ) -> Result<RecordBatch, VectorStoreJuliaRequestError> {
        self.build_plugin_rerank_request_batch(request, anchors)
            .await
    }
}

/// Error returned when the vector-backed semantic ignition cannot assemble one
/// WendaoArrow plugin rerank request batch.
#[cfg(feature = "julia")]
#[derive(Debug, Error)]
pub enum VectorStorePluginRerankRequestError {
    /// Fetching candidate embeddings from the vector store failed.
    #[error("failed to fetch candidate embeddings for plugin rerank request")]
    VectorStore(#[source] VectorStoreError),
    /// One anchor id from the semantic search result set had no stored vector.
    #[error("missing embedding for plugin rerank anchor `{anchor_id}`")]
    MissingEmbedding {
        /// Anchor id that could not be resolved into a stored embedding row.
        anchor_id: String,
    },
    /// WendaoArrow request batch construction failed.
    #[error("failed to build plugin rerank request batch")]
    Build(#[source] RepoIntelligenceError),
}

#[cfg(feature = "julia")]
pub type VectorStoreJuliaRequestError = VectorStorePluginRerankRequestError;

impl QuantumSemanticIgnition for VectorStoreSemanticIgnition {
    type Error = VectorStoreError;

    fn backend_name(&self) -> &str {
        self.backend_name.as_str()
    }

    fn search_anchors<'a>(
        &'a self,
        request: QuantumSemanticSearchRequest<'a>,
    ) -> QuantumSemanticIgnitionFuture<'a, Self::Error> {
        let store = self.store.clone();
        let table_name = self.table_name.clone();
        let options = self.search_options.clone();
        let query_vector = request.query_vector.to_vec();
        let limit = request.candidate_limit.max(1);

        Box::pin(async move {
            if query_vector.is_empty() || limit == 0 {
                return Ok(Vec::new());
            }
            let results = store
                .search_optimized(&table_name, query_vector, limit, options)
                .await?;
            Ok(results
                .into_iter()
                .map(|result| QuantumAnchorHit {
                    anchor_id: result.id,
                    vector_score: distance_to_score(result.distance),
                })
                .collect())
        })
    }
}

#[cfg(all(test, feature = "julia"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn build_plugin_rerank_request_batch_uses_anchor_ids_as_request_doc_ids() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("vector_ignition_julia");
        let db_path_str = db_path.to_string_lossy();
        let mut store = VectorStore::new(db_path_str.as_ref(), Some(3))
            .await
            .expect("create vector store");
        store
            .replace_documents(
                "anchors",
                vec!["doc-1#alpha".to_string(), "doc-2#beta".to_string()],
                vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]],
                vec!["alpha".to_string(), "beta".to_string()],
                vec!["{}".to_string(), "{}".to_string()],
            )
            .await
            .expect("seed vector table");

        let ignition = VectorStoreSemanticIgnition::new(store, "anchors");
        let request = QuantumSemanticSearchRequest {
            query_text: Some("demo"),
            query_vector: &[9.0, 8.0, 7.0],
            candidate_limit: 2,
            min_vector_score: None,
            max_vector_score: None,
        };
        let batch = ignition
            .build_plugin_rerank_request_batch(
                request,
                &[
                    QuantumAnchorHit {
                        anchor_id: "doc-1#alpha".to_string(),
                        vector_score: 0.31,
                    },
                    QuantumAnchorHit {
                        anchor_id: "doc-2#beta".to_string(),
                        vector_score: 0.42,
                    },
                ],
            )
            .await
            .expect("request batch should build");

        let doc_ids = batch
            .column_by_name("doc_id")
            .and_then(|column| column.as_any().downcast_ref::<arrow::array::StringArray>())
            .expect("doc_id column");
        assert_eq!(doc_ids.value(0), "doc-1#alpha");
        assert_eq!(doc_ids.value(1), "doc-2#beta");
    }
}
