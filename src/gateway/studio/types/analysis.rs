use serde::{Deserialize, Serialize};
use specta::Type;

use super::retrieval::RetrievalChunk;

/// Kind of an analysis node.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisNodeKind {
    /// Markdown section heading.
    Section,
    /// Task list item.
    Task,
    /// Observation/evidence block.
    Observation,
    /// Symbolic link or relation.
    Relation,
    /// Document-level node.
    Document,
    /// Code block node.
    CodeBlock,
    /// Markdown table node.
    Table,
    /// Display math node.
    Math,
    /// Semantic reference site.
    Reference,
    /// Property box node.
    Property,
    /// Symbolic entity node.
    Symbol,
}

/// Kind of an analysis edge.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisEdgeKind {
    /// Parent-child hierarchy.
    Parent,
    /// Semantic reference or mention.
    Mentions,
    /// Document membership.
    Contains,
    /// Next task in sequence.
    NextStep,
    /// Explicit document reference.
    References,
}

/// Metadata about an analysis edge evidence.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisEvidence {
    /// Evidence file path.
    pub path: String,
    /// 1-based start line.
    pub line_start: usize,
    /// 1-based end line.
    pub line_end: usize,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
}

/// A single node in the structural IR of a document.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisNode {
    /// Node identifier.
    pub id: String,
    /// Node kind.
    pub kind: AnalysisNodeKind,
    /// Display label.
    pub label: String,
    /// Nesting depth.
    pub depth: usize,
    /// 1-based start line.
    pub line_start: usize,
    /// 1-based end line.
    pub line_end: usize,
    /// Optional parent node identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

/// A relationship edge in the document IR.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisEdge {
    /// Edge identifier.
    pub id: String,
    /// Source node identifier.
    pub source_id: String,
    /// Target node identifier.
    pub target_id: String,
    /// Relationship kind.
    pub kind: AnalysisEdgeKind,
    /// Display label.
    pub label: String,
    /// Evidence metadata.
    pub evidence: AnalysisEvidence,
}

/// Shared retrieval chunk used by markdown analysis surfaces.
pub type MarkdownRetrievalAtom = RetrievalChunk;

/// Full response for Markdown analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownAnalysisResponse {
    /// Analyzed file path.
    pub path: String,
    /// Content fingerprint.
    pub document_hash: String,
    /// Total number of nodes.
    pub node_count: usize,
    /// Total number of edges.
    pub edge_count: usize,
    /// IR nodes.
    pub nodes: Vec<AnalysisNode>,
    /// IR edges.
    pub edges: Vec<AnalysisEdge>,
    /// Mermaid diagram projections.
    pub projections: Vec<MermaidProjection>,
    /// Compact retrieval atoms for document / section surfaces.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retrieval_atoms: Vec<MarkdownRetrievalAtom>,
    /// Analysis diagnostics.
    pub diagnostics: Vec<String>,
}

/// Mermaid projection view kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum MermaidViewKind {
    /// Hierarchical document outline.
    Outline,
    /// Task dependency graph.
    Tasks,
    /// Semantic entity relations.
    Knowledge,
}

/// A single Mermaid diagram projection.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MermaidProjection {
    /// Type of projection.
    pub kind: MermaidViewKind,
    /// Generated Mermaid source.
    pub source: String,
    /// Number of nodes in projection.
    pub node_count: usize,
    /// Number of edges in projection.
    pub edge_count: usize,
}
