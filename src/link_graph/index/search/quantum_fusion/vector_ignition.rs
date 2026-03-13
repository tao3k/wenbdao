use super::semantic_ignition::{QuantumSemanticIgnition, QuantumSemanticIgnitionFuture};
use crate::link_graph::models::{QuantumAnchorHit, QuantumSemanticSearchRequest};
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
}

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
