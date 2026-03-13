//! Graph operations for the studio API.

use super::router::{StudioApiError, StudioState};
use super::types::{GraphLink, GraphNeighborsResponse, GraphNode, NodeNeighbors};

/// Get immediate neighbors for a node.
pub(crate) async fn node_neighbors(
    state: &StudioState,
    id: &str,
) -> Result<NodeNeighbors, StudioApiError> {
    let _index = state.graph_index().await?;

    // Placeholder implementation - return empty neighbors
    Ok(NodeNeighbors {
        node_id: id.to_string(),
        name: id.to_string(),
        node_type: "unknown".to_string(),
        incoming: vec![],
        outgoing: vec![],
        two_hop: vec![],
    })
}

/// Get graph neighbors with configurable depth and direction.
pub(crate) async fn graph_neighbors(
    state: &StudioState,
    id: &str,
    direction: &str,
    hops: usize,
    limit: usize,
) -> Result<GraphNeighborsResponse, StudioApiError> {
    let _index = state.graph_index().await?;

    // Placeholder implementation
    let center = GraphNode {
        id: id.to_string(),
        label: id.to_string(),
        path: id.to_string(),
        node_type: "unknown".to_string(),
        is_center: true,
        distance: 0,
    };

    Ok(GraphNeighborsResponse {
        center,
        nodes: vec![],
        links: vec![],
        total_nodes: 1,
        total_links: 0,
    })
}
