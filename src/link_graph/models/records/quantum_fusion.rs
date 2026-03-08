use crate::link_graph::LinkGraphRelatedPprOptions;
use serde::{Deserialize, Serialize};

/// One semantic ignition hit fed into Wendao's hybrid retriever.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumAnchorHit {
    /// Stable anchor identifier, typically a `PageIndex` node id.
    pub anchor_id: String,
    /// Semantic score produced by the upstream vector stage.
    pub vector_score: f64,
}

/// Backend-agnostic semantic search request for hybrid retrieval.
#[derive(Debug, Clone, Copy)]
pub struct QuantumSemanticSearchRequest<'a> {
    /// Optional raw query text for hybrid backends that use text and vector inputs together.
    pub query_text: Option<&'a str>,
    /// Precomputed semantic query vector.
    pub query_vector: &'a [f32],
    /// Maximum number of semantic anchors to request.
    pub limit: usize,
    /// Optional minimum semantic score required before topology diffusion runs.
    pub min_vector_score: Option<f64>,
}

impl QuantumSemanticSearchRequest<'_> {
    /// Return a normalized copy with trimmed text and a non-zero limit.
    #[must_use]
    pub fn normalized(self) -> Self {
        Self {
            query_text: self
                .query_text
                .map(str::trim)
                .filter(|text| !text.is_empty()),
            query_vector: self.query_vector,
            limit: self.limit.max(1),
            min_vector_score: self.min_vector_score.map(|score| score.clamp(0.0, 1.0)),
        }
    }

    /// Return whether the request carries no usable text or vector signal.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.query_text.is_none() && self.query_vector.is_empty()
    }

    /// Return whether a semantic score should seed topology diffusion.
    #[must_use]
    pub fn allows_vector_score(self, score: f64) -> bool {
        self.min_vector_score
            .is_none_or(|minimum| score.clamp(0.0, 1.0) >= minimum)
    }
}

/// Runtime options for quantum-fusion retrieval orchestration.
#[derive(Debug, Clone, Default)]
pub struct QuantumFusionOptions {
    /// Weight assigned to semantic ignition scores.
    pub alpha: f64,
    /// Maximum graph traversal distance for topology diffusion.
    pub max_distance: usize,
    /// Maximum number of related cluster ids to retain per anchor.
    pub related_limit: usize,
    /// Optional PPR tuning overrides.
    pub ppr: Option<LinkGraphRelatedPprOptions>,
}

impl QuantumFusionOptions {
    /// Return a normalized copy with bounded weights and limits.
    #[must_use]
    pub fn normalized(&self) -> Self {
        Self {
            alpha: self.alpha.clamp(0.0, 1.0),
            max_distance: self.max_distance.max(1),
            related_limit: self.related_limit.max(1),
            ppr: self.ppr.clone(),
        }
    }
}

/// Traceable hybrid-retrieval context assembled from semantic and topology layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumContext {
    /// Original anchor identifier selected by semantic ignition.
    pub anchor_id: String,
    /// Complete logical ancestry path recovered from `PageIndex`.
    pub semantic_path: Vec<String>,
    /// Canonical related cluster ids recovered from topology diffusion.
    pub related_clusters: Vec<String>,
    /// Final fused saliency score.
    pub saliency_score: f64,
    /// Raw semantic ignition score.
    pub vector_score: f64,
    /// Raw topology diffusion score.
    pub topology_score: f64,
}

impl QuantumContext {
    /// Render the canonical traceability label for downstream prompts.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!("[Path: {}]", self.semantic_path.join(" > "))
    }
}
