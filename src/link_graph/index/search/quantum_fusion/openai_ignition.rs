use super::semantic_ignition::{QuantumSemanticIgnition, QuantumSemanticIgnitionFuture};
#[cfg(feature = "julia")]
use crate::analyzers::{
    PluginArrowRequestRow, RepoIntelligenceError, build_plugin_arrow_request_batch,
};
use crate::link_graph::models::{QuantumAnchorHit, QuantumSemanticSearchRequest};
#[cfg(feature = "julia")]
use arrow::record_batch::RecordBatch;
use thiserror::Error;
use xiuxian_llm::embedding::openai_compat::embed_openai_compatible;
use xiuxian_vector::{SearchOptions, VectorStore, VectorStoreError, distance_to_score};

/// Semantic ignition adapter backed by an OpenAI-compatible embeddings API plus
/// the Rust vector store.
#[derive(Clone)]
pub struct OpenAiCompatibleSemanticIgnition {
    store: VectorStore,
    table_name: String,
    search_options: SearchOptions,
    backend_name: String,
    embedding_client: reqwest::Client,
    embedding_base_url: String,
    embedding_model: Option<String>,
}

impl OpenAiCompatibleSemanticIgnition {
    /// Create an OpenAI-compatible semantic ignition adapter.
    ///
    /// `embedding_base_url` is normalized by `xiuxian-llm` into
    /// `{base}/v1/embeddings` at request time.
    pub fn new(
        store: VectorStore,
        table_name: impl Into<String>,
        embedding_base_url: impl Into<String>,
    ) -> Self {
        Self {
            store,
            table_name: table_name.into(),
            search_options: SearchOptions::default(),
            backend_name: "openai-compatible+xiuxian-vector".to_string(),
            embedding_client: reqwest::Client::new(),
            embedding_base_url: embedding_base_url.into(),
            embedding_model: None,
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

    /// Override the HTTP client used for embedding calls.
    ///
    /// This can be used to inject authentication headers for provider
    /// gateways that require API keys.
    #[must_use]
    pub fn with_embedding_client(mut self, client: reqwest::Client) -> Self {
        self.embedding_client = client;
        self
    }

    /// Set an explicit embedding model name for the OpenAI-compatible request.
    #[must_use]
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = Some(model.into());
        self
    }

    /// Override the base URL used for OpenAI-compatible embedding calls.
    #[must_use]
    pub fn with_embedding_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.embedding_base_url = base_url.into();
        self
    }

    #[cfg(feature = "julia")]
    async fn resolve_query_vector(
        &self,
        request: QuantumSemanticSearchRequest<'_>,
    ) -> Result<Vec<f32>, OpenAiCompatibleSemanticIgnitionError> {
        let query_vector = request.query_vector.to_vec();
        if !query_vector.is_empty() {
            return Ok(query_vector);
        }

        let query_text = request
            .query_text
            .filter(|value| !value.trim().is_empty())
            .ok_or(OpenAiCompatibleSemanticIgnitionError::MissingQuerySignal)?;
        let texts = vec![query_text.to_string()];
        let vectors = embed_openai_compatible(
            &self.embedding_client,
            self.embedding_base_url.as_str(),
            &texts,
            self.embedding_model.as_deref(),
        )
        .await
        .ok_or(OpenAiCompatibleSemanticIgnitionError::EmbeddingUnavailable)?;
        let mut vectors = vectors.into_iter();
        let vector = vectors
            .next()
            .ok_or(OpenAiCompatibleSemanticIgnitionError::EmbeddingUnavailable)?;
        if vector.is_empty() {
            return Err(OpenAiCompatibleSemanticIgnitionError::EmptyEmbeddingVector);
        }
        Ok(vector)
    }

    /// Build a WendaoArrow `v1` plugin rerank request batch for one
    /// OpenAI-compatible semantic-ignition result set.
    ///
    /// # Errors
    ///
    /// Returns [`OpenAiCompatibleJuliaRequestError`] when the effective
    /// query vector cannot be resolved, candidate embeddings cannot be fetched,
    /// or the WendaoArrow request batch cannot be assembled.
    #[cfg(feature = "julia")]
    pub async fn build_plugin_rerank_request_batch(
        &self,
        request: QuantumSemanticSearchRequest<'_>,
        anchors: &[QuantumAnchorHit],
    ) -> Result<RecordBatch, OpenAiCompatiblePluginRerankRequestError> {
        if anchors.is_empty() {
            return Err(OpenAiCompatiblePluginRerankRequestError::Build(
                RepoIntelligenceError::AnalysisFailed {
                    message: "cannot build Julia rerank request from an empty anchor set"
                        .to_string(),
                },
            ));
        }
        let query_vector = self
            .resolve_query_vector(request)
            .await
            .map_err(OpenAiCompatiblePluginRerankRequestError::Ignition)?;
        let ids = anchors
            .iter()
            .map(|anchor| anchor.anchor_id.clone())
            .collect::<Vec<_>>();
        let embeddings = self
            .store
            .fetch_embeddings_by_ids(self.table_name.as_str(), &ids)
            .await
            .map_err(OpenAiCompatiblePluginRerankRequestError::VectorStore)?;

        let mut rows = Vec::with_capacity(anchors.len());
        for anchor in anchors {
            let embedding = embeddings
                .get(anchor.anchor_id.as_str())
                .cloned()
                .ok_or_else(
                    || OpenAiCompatiblePluginRerankRequestError::MissingEmbedding {
                        anchor_id: anchor.anchor_id.clone(),
                    },
                )?;
            rows.push(PluginArrowRequestRow {
                doc_id: anchor.anchor_id.clone(),
                vector_score: anchor.vector_score,
                embedding,
            });
        }

        build_plugin_arrow_request_batch(&rows, &query_vector)
            .map_err(OpenAiCompatiblePluginRerankRequestError::Build)
    }

    /// Compatibility shim for the legacy Julia-named rerank request builder.
    ///
    /// # Errors
    ///
    /// Returns [`OpenAiCompatiblePluginRerankRequestError`] when the effective
    /// query vector cannot be resolved, candidate embeddings cannot be fetched,
    /// or the WendaoArrow request batch cannot be assembled.
    #[cfg(feature = "julia")]
    pub async fn build_julia_rerank_request_batch(
        &self,
        request: QuantumSemanticSearchRequest<'_>,
        anchors: &[QuantumAnchorHit],
    ) -> Result<RecordBatch, OpenAiCompatibleJuliaRequestError> {
        self.build_plugin_rerank_request_batch(request, anchors)
            .await
    }
}

/// Error returned when OpenAI-compatible semantic ignition cannot produce
/// anchor hits.
#[derive(Debug, Error)]
pub enum OpenAiCompatibleSemanticIgnitionError {
    /// Request did not provide either a precomputed vector or query text.
    #[error("semantic request missing both query_vector and query_text")]
    MissingQuerySignal,
    /// OpenAI-compatible embedding request failed or returned invalid payload.
    #[error("openai-compatible embedding unavailable")]
    EmbeddingUnavailable,
    /// OpenAI-compatible embedding succeeded but returned an empty vector.
    #[error("openai-compatible embedding returned empty vector")]
    EmptyEmbeddingVector,
    /// Vector store search failed.
    #[error("vector store search failed: {0}")]
    VectorStore(#[from] VectorStoreError),
}

/// Error returned when OpenAI-compatible semantic ignition cannot assemble one
/// WendaoArrow plugin rerank request batch.
#[cfg(feature = "julia")]
#[derive(Debug, Error)]
pub enum OpenAiCompatiblePluginRerankRequestError {
    /// Effective query-vector resolution failed.
    #[error("failed to resolve query vector for plugin rerank request")]
    Ignition(#[source] OpenAiCompatibleSemanticIgnitionError),
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
pub type OpenAiCompatibleJuliaRequestError = OpenAiCompatiblePluginRerankRequestError;

impl QuantumSemanticIgnition for OpenAiCompatibleSemanticIgnition {
    type Error = OpenAiCompatibleSemanticIgnitionError;

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
        let limit = request.candidate_limit.max(1);
        let embedding_client = self.embedding_client.clone();
        let embedding_base_url = self.embedding_base_url.clone();
        let embedding_model = self.embedding_model.clone();
        let query_text = request.query_text.map(str::to_string);
        let query_vector = request.query_vector.to_vec();

        Box::pin(async move {
            let effective_query_vector = if query_vector.is_empty() {
                let query_text = query_text
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or(OpenAiCompatibleSemanticIgnitionError::MissingQuerySignal)?;
                let texts = vec![query_text.to_string()];
                let vectors = embed_openai_compatible(
                    &embedding_client,
                    embedding_base_url.as_str(),
                    &texts,
                    embedding_model.as_deref(),
                )
                .await
                .ok_or(OpenAiCompatibleSemanticIgnitionError::EmbeddingUnavailable)?;
                let mut vectors = vectors.into_iter();
                let vector = vectors
                    .next()
                    .ok_or(OpenAiCompatibleSemanticIgnitionError::EmbeddingUnavailable)?;
                if vector.is_empty() {
                    return Err(OpenAiCompatibleSemanticIgnitionError::EmptyEmbeddingVector);
                }
                vector
            } else {
                query_vector
            };

            let results = store
                .search_optimized(&table_name, effective_query_vector, limit, options)
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
    async fn build_plugin_rerank_request_batch_uses_explicit_query_vector() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("openai_ignition_julia");
        let db_path_str = db_path.to_string_lossy();
        let mut store = VectorStore::new(db_path_str.as_ref(), Some(3))
            .await
            .expect("create vector store");
        store
            .replace_documents(
                "anchors",
                vec!["doc-1#alpha".to_string()],
                vec![vec![1.0, 2.0, 3.0]],
                vec!["alpha".to_string()],
                vec!["{}".to_string()],
            )
            .await
            .expect("seed vector table");

        let ignition =
            OpenAiCompatibleSemanticIgnition::new(store, "anchors", "http://127.0.0.1:9999");
        let request = QuantumSemanticSearchRequest {
            query_text: Some("demo"),
            query_vector: &[9.0, 8.0, 7.0],
            candidate_limit: 1,
            min_vector_score: None,
            max_vector_score: None,
        };
        let batch = ignition
            .build_plugin_rerank_request_batch(
                request,
                &[QuantumAnchorHit {
                    anchor_id: "doc-1#alpha".to_string(),
                    vector_score: 0.31,
                }],
            )
            .await
            .expect("request batch should build");

        assert_eq!(batch.num_rows(), 1);
        assert!(batch.column_by_name("query_embedding").is_some());
    }

    #[tokio::test]
    async fn build_plugin_rerank_request_batch_rejects_missing_query_signal() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("openai_ignition_julia_error");
        let db_path_str = db_path.to_string_lossy();
        let store = VectorStore::new(db_path_str.as_ref(), Some(3))
            .await
            .expect("create vector store");

        let ignition =
            OpenAiCompatibleSemanticIgnition::new(store, "anchors", "http://127.0.0.1:9999");
        let error = ignition
            .build_plugin_rerank_request_batch(
                QuantumSemanticSearchRequest {
                    query_text: None,
                    query_vector: &[],
                    candidate_limit: 1,
                    min_vector_score: None,
                    max_vector_score: None,
                },
                &[QuantumAnchorHit {
                    anchor_id: "doc-1#alpha".to_string(),
                    vector_score: 0.31,
                }],
            )
            .await
            .expect_err("missing query signal should fail");

        assert!(matches!(
            error,
            OpenAiCompatiblePluginRerankRequestError::Ignition(
                OpenAiCompatibleSemanticIgnitionError::MissingQuerySignal
            )
        ));
    }
}
