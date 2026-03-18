use std::sync::Arc;

/// Typed semantic document exported from `LinkGraphIndex` for downstream retrieval runtimes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkGraphSemanticDocument {
    /// Stable anchor identifier used to recover semantic paths.
    pub anchor_id: String,
    /// Canonical document identifier owning this semantic document.
    pub doc_id: String,
    /// Relative markdown path for traceability.
    pub path: String,
    /// Semantic document kind used by downstream document-scope filters.
    pub kind: LinkGraphSemanticDocumentKind,
    /// Complete logical ancestry path recovered from `PageIndex`.
    pub semantic_path: Vec<String>,
    /// Text payload exported for semantic indexing.
    pub content: Arc<str>,
    /// Optional source line range when the document maps to one concrete section.
    pub line_range: Option<(usize, usize)>,
}

/// Semantic document kind exported from `LinkGraphIndex`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkGraphSemanticDocumentKind {
    /// One document-level summary row.
    Summary,
    /// One section-level semantic row derived from `PageIndex`.
    Section,
    /// Agent reasoning trace captured during workflow execution (V6.1 Sovereign Memory).
    CognitiveTrace,
}

impl LinkGraphSemanticDocumentKind {
    /// Return the canonical metadata label used by vector retrieval adapters.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Section => "section",
            Self::CognitiveTrace => "cognitive_trace",
        }
    }
}

/// Cognitive trace artifact for sovereign memory (V6.1).
///
/// Represents a persistent reasoning trace that connects Intent → Reasoning → Outcome.
/// This enables historical sovereignty: querying the knowledge graph for the reasoning
/// chain that led to any commit or decision.
#[derive(Debug, Clone, PartialEq)]
pub struct CognitiveTraceRecord {
    /// Unique identifier for this trace.
    pub trace_id: String,
    /// Session identifier from Qianji execution.
    pub session_id: Option<String>,
    /// Node identifier from the compiled flow graph.
    pub node_id: String,
    /// The original user intent/prompt.
    pub intent: String,
    /// Aggregated reasoning content (thoughts + text deltas).
    pub reasoning: Arc<str>,
    /// Final outcome or conclusion.
    pub outcome: Option<Arc<str>>,
    /// Associated commit hash if the trace led to code changes.
    pub commit_sha: Option<String>,
    /// Timestamp when the trace was captured.
    pub timestamp_ms: u64,
    /// Cognitive coherence score during execution.
    pub coherence_score: Option<f32>,
    /// Whether early halt was triggered.
    pub early_halt_triggered: bool,
}

impl CognitiveTraceRecord {
    /// Create a new cognitive trace record.
    #[must_use]
    pub fn new(
        trace_id: String,
        session_id: Option<String>,
        node_id: String,
        intent: String,
    ) -> Self {
        Self {
            trace_id,
            session_id,
            node_id,
            intent,
            reasoning: Arc::<str>::from(""),
            outcome: None,
            commit_sha: None,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .try_into()
                .unwrap_or(0),
            coherence_score: None,
            early_halt_triggered: false,
        }
    }

    /// Convert to a semantic document for Wendao ingestion.
    #[must_use]
    pub fn to_semantic_document(&self, doc_id: &str, path: &str) -> LinkGraphSemanticDocument {
        LinkGraphSemanticDocument {
            anchor_id: format!("trace:{}", self.trace_id),
            doc_id: doc_id.to_string(),
            path: path.to_string(),
            kind: LinkGraphSemanticDocumentKind::CognitiveTrace,
            semantic_path: vec!["Cognitive Traces".to_string(), self.node_id.clone()],
            content: self.reasoning.clone(),
            line_range: None,
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/link_graph/models/records/semantic_document.rs"]
mod tests;
