//! Studio API types for TypeScript bindings and HTTP endpoints.
//!
//! This module defines all types used by the Qianji Studio frontend API,
//! including VFS operations, graph queries, search, and UI configuration.

use serde::{Deserialize, Serialize};
use specta::{Type, TypeCollection};

// === VFS Types ===

/// A single entry in the VFS (file or directory).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsEntry {
    /// Full path relative to the VFS root.
    pub path: String,
    /// File or directory name.
    pub name: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Last modified timestamp (Unix seconds).
    pub modified: u64,
    /// MIME content type guess for files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// Category classification for VFS entries.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum VfsCategory {
    /// Directory/folder.
    Folder,
    /// Skill definition file.
    Skill,
    /// Documentation file.
    Doc,
    /// Knowledge base file.
    Knowledge,
    /// Other/uncategorized file.
    Other,
}

/// A scanned entry with metadata for VFS tree display.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsScanEntry {
    /// Full path relative to the VFS root.
    pub path: String,
    /// File or directory name.
    pub name: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// Category classification for UI styling.
    pub category: VfsCategory,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Last modified timestamp (Unix seconds).
    pub modified: u64,
    /// MIME content type guess for files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Whether the file has YAML frontmatter.
    pub has_frontmatter: bool,
    /// Wendao document ID if indexed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wendao_id: Option<String>,
}

/// Result of a VFS scan operation.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsScanResult {
    /// All entries found during the scan.
    pub entries: Vec<VfsScanEntry>,
    /// Total number of files scanned.
    pub file_count: usize,
    /// Total number of directories scanned.
    pub dir_count: usize,
    /// Time taken to scan in milliseconds.
    pub scan_duration_ms: u64,
}

/// Response for VFS file content read.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsContentResponse {
    /// Path of the file read.
    pub path: String,
    /// File content as UTF-8 string.
    pub content: String,
    /// MIME content type.
    pub content_type: String,
}

// === Graph Types ===

/// Neighbors of a node in the graph.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NodeNeighbors {
    /// Node identifier.
    pub node_id: String,
    /// Display name of the node.
    pub name: String,
    /// Type of the node (e.g., "doc", "skill").
    pub node_type: String,
    /// IDs of nodes with incoming edges to this node.
    pub incoming: Vec<String>,
    /// IDs of nodes this node points to.
    pub outgoing: Vec<String>,
    /// IDs of nodes reachable within 2 hops.
    pub two_hop: Vec<String>,
}

/// A node in the graph visualization.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    /// Unique node identifier.
    pub id: String,
    /// Display label.
    pub label: String,
    /// File path if applicable.
    pub path: String,
    /// Node type for styling.
    pub node_type: String,
    /// Whether this is the center of the query.
    pub is_center: bool,
    /// Distance from the center node.
    pub distance: usize,
}

/// A link between graph nodes.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphLink {
    /// Source node ID.
    pub source: String,
    /// Target node ID.
    pub target: String,
    /// Direction of the link ("incoming", "outgoing", "bidirectional").
    pub direction: String,
    /// Distance from center for filtering.
    pub distance: usize,
}

/// Response for graph neighbors query.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphNeighborsResponse {
    /// The center node of the query.
    pub center: GraphNode,
    /// All nodes in the neighborhood.
    pub nodes: Vec<GraphNode>,
    /// All links between nodes.
    pub links: Vec<GraphLink>,
    /// Total number of nodes.
    pub total_nodes: usize,
    /// Total number of links.
    pub total_links: usize,
}

/// A node in the 3D topology visualization.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TopologyNode {
    /// Unique node identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Node type for styling.
    pub node_type: String,
    /// 3D position [x, y, z].
    pub position: [f32; 3],
    /// Cluster ID if clustered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster_id: Option<String>,
}

/// A link in the 3D topology.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TopologyLink {
    /// Source node ID.
    pub from: String,
    /// Target node ID.
    pub to: String,
    /// Optional link label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Information about a cluster in the 3D topology.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClusterInfo {
    /// Cluster identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Centroid position [x, y, z].
    pub centroid: [f32; 3],
    /// Number of nodes in cluster.
    pub node_count: usize,
    /// Color for rendering (CSS color string).
    pub color: String,
}

/// Complete 3D topology for visualization.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Topology3D {
    /// All nodes in the topology.
    pub nodes: Vec<TopologyNode>,
    /// All links between nodes.
    pub links: Vec<TopologyLink>,
    /// Cluster information for grouping.
    pub clusters: Vec<ClusterInfo>,
}

// === State Types ===

/// State of a research/workflow node.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    /// Node is idle.
    Idle,
    /// Node is active.
    Active,
    /// Node is processing.
    Processing,
    /// Node completed successfully.
    Success,
    /// Node is waiting.
    Wait,
}

/// Event for research state updates.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ResearchStateEvent {
    /// A node was activated.
    NodeActivated {
        /// Node identifier.
        node_id: String,
        /// New state.
        state: NodeState,
    },
    /// A workflow step started.
    StepStarted {
        /// Step identifier.
        step_id: String,
        /// Timestamp in milliseconds.
        timestamp: u64,
    },
    /// A workflow step completed.
    StepCompleted {
        /// Step identifier.
        step_id: String,
        /// Whether the step succeeded.
        success: bool,
        /// Duration in milliseconds.
        duration_ms: u64,
    },
    /// The topology was updated.
    TopologyUpdated {
        /// Total node count.
        node_count: usize,
        /// Total link count.
        link_count: usize,
    },
}

// === Search Types ===

/// A knowledge search result.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSearchResult {
    /// Result identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Relevance score.
    pub score: f64,
    /// Text snippet.
    pub snippet: String,
    /// Source path.
    pub source: String,
}

/// A search hit from the graph index.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    /// Document stem (filename without extension).
    pub stem: String,
    /// Document title if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Full file path.
    pub path: String,
    /// Document type classification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,
    /// Associated tags.
    pub tags: Vec<String>,
    /// Relevance score.
    pub score: f64,
    /// Best matching section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_section: Option<String>,
    /// Reason for the match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_reason: Option<String>,
}

/// Response for search queries.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    /// Original query string.
    pub query: String,
    /// Search hits.
    pub hits: Vec<SearchHit>,
    /// Total number of hits.
    pub hit_count: usize,
    /// Confidence score from graph analysis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_confidence_score: Option<f64>,
    /// Selected search mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_mode: Option<String>,
}

/// Type of autocomplete suggestion.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum AutocompleteSuggestionType {
    /// Document title suggestion.
    Title,
    /// Tag suggestion.
    Tag,
    /// Document stem suggestion.
    Stem,
}

/// A single autocomplete suggestion.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AutocompleteSuggestion {
    /// Suggestion text.
    pub text: String,
    /// Type of suggestion.
    pub suggestion_type: AutocompleteSuggestionType,
    /// Associated path if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Document type if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,
}

/// Response for autocomplete queries.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AutocompleteResponse {
    /// Prefix that was completed.
    pub prefix: String,
    /// Suggested completions.
    pub suggestions: Vec<AutocompleteSuggestion>,
}

// === UI Config Types ===

/// UI configuration for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UiConfig {
    /// Paths to expand by default in the file tree.
    pub index_paths: Vec<String>,
}

// === Error Types ===

/// API error response.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// Error code for programmatic handling.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Additional error details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Collect all studio types for TypeScript export.
#[must_use]
pub fn studio_type_collection() -> TypeCollection {
    TypeCollection::default()
        .register::<VfsEntry>()
        .register::<VfsCategory>()
        .register::<VfsScanEntry>()
        .register::<VfsScanResult>()
        .register::<VfsContentResponse>()
        .register::<NodeNeighbors>()
        .register::<GraphNode>()
        .register::<GraphLink>()
        .register::<GraphNeighborsResponse>()
        .register::<TopologyNode>()
        .register::<TopologyLink>()
        .register::<ClusterInfo>()
        .register::<Topology3D>()
        .register::<NodeState>()
        .register::<ResearchStateEvent>()
        .register::<KnowledgeSearchResult>()
        .register::<SearchHit>()
        .register::<SearchResponse>()
        .register::<AutocompleteSuggestionType>()
        .register::<AutocompleteSuggestion>()
        .register::<AutocompleteResponse>()
        .register::<UiConfig>()
        .register::<ApiError>()
}
