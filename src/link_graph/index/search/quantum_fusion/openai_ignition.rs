use super::semantic_ignition::{QuantumSemanticIgnition, QuantumSemanticIgnitionFuture};
use crate::link_graph::models::{QuantumAnchorHit, QuantumSemanticSearchRequest};
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
