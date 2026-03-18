//! Graph operations for the studio API.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::link_graph::{LinkGraphDirection, LinkGraphMetadata};

use super::router::{GatewayState, StudioApiError};
use super::types::{
    ClusterInfo, GraphLink, GraphNeighborsResponse, GraphNode, NodeNeighbors, Topology3D,
    TopologyLink, TopologyNode,
};
use super::vfs::{graph_lookup_candidates, studio_display_path};

/// Get immediate neighbors for a node.
pub(crate) async fn node_neighbors(
    state: &GatewayState,
    id: &str,
) -> Result<NodeNeighbors, StudioApiError> {
    let index = state.link_graph_index().await?;
    let metadata = resolve_metadata(state, index.as_ref(), id)
        .ok_or_else(|| StudioApiError::not_found(format!("Node not found: {id}")))?;
    let center_id = metadata.path.clone();
    let incoming = index
        .neighbors(center_id.as_str(), LinkGraphDirection::Incoming, 1, 128)
        .into_iter()
        .map(|neighbor| studio_display_path(state.studio.as_ref(), neighbor.path.as_str()))
        .collect();
    let outgoing = index
        .neighbors(center_id.as_str(), LinkGraphDirection::Outgoing, 1, 128)
        .into_iter()
        .map(|neighbor| studio_display_path(state.studio.as_ref(), neighbor.path.as_str()))
        .collect();
    let two_hop = index
        .neighbors(center_id.as_str(), LinkGraphDirection::Both, 2, 256)
        .into_iter()
        .filter(|neighbor| neighbor.distance > 1)
        .map(|neighbor| studio_display_path(state.studio.as_ref(), neighbor.path.as_str()))
        .collect();

    Ok(NodeNeighbors {
        node_id: studio_display_path(state.studio.as_ref(), center_id.as_str()),
        name: display_name(&metadata),
        node_type: classify_node_type(center_id.as_str()),
        incoming,
        outgoing,
        two_hop,
    })
}

/// Get graph neighbors with configurable depth and direction.
pub(crate) async fn graph_neighbors(
    state: &GatewayState,
    id: &str,
    direction: &str,
    hops: usize,
    limit: usize,
) -> Result<GraphNeighborsResponse, StudioApiError> {
    let index = state.link_graph_index().await?;
    let metadata = resolve_metadata(state, index.as_ref(), id)
        .ok_or_else(|| StudioApiError::not_found(format!("Node not found: {id}")))?;
    let center_id = metadata.path.clone();
    let center_display_id = studio_display_path(state.studio.as_ref(), center_id.as_str());
    let direction = LinkGraphDirection::from_alias(direction);
    let neighbors = index.neighbors(center_id.as_str(), direction, hops, limit);

    let center = GraphNode {
        id: center_display_id.clone(),
        label: display_name(&metadata),
        path: center_display_id.clone(),
        node_type: classify_node_type(center_id.as_str()),
        is_center: true,
        distance: 0,
    };
    let mut seen_nodes = HashSet::from([center_display_id.clone()]);
    let mut nodes = vec![center.clone()];
    let mut links = Vec::new();

    for neighbor in neighbors {
        let neighbor_display_path =
            studio_display_path(state.studio.as_ref(), neighbor.path.as_str());
        if seen_nodes.insert(neighbor_display_path.clone()) {
            nodes.push(GraphNode {
                id: neighbor_display_path.clone(),
                label: display_label(&neighbor.title, &neighbor.stem),
                path: neighbor_display_path.clone(),
                node_type: classify_node_type(neighbor.path.as_str()),
                is_center: false,
                distance: neighbor.distance,
            });
        }

        let (source, target) = match neighbor.direction {
            LinkGraphDirection::Incoming => {
                (neighbor_display_path.clone(), center_display_id.clone())
            }
            LinkGraphDirection::Outgoing | LinkGraphDirection::Both => {
                (center_display_id.clone(), neighbor_display_path.clone())
            }
        };
        links.push(GraphLink {
            source,
            target,
            direction: direction_to_string(neighbor.direction),
            distance: neighbor.distance,
        });
    }

    Ok(GraphNeighborsResponse {
        center,
        total_nodes: nodes.len(),
        total_links: links.len(),
        nodes,
        links,
    })
}

/// Build a deterministic graph topology payload.
pub(crate) async fn topology_3d(state: &GatewayState) -> Result<Topology3D, StudioApiError> {
    let index = state.link_graph_index().await?;
    let docs = index.toc(128);
    if docs.is_empty() {
        return Ok(Topology3D {
            nodes: Vec::new(),
            links: Vec::new(),
            clusters: Vec::new(),
        });
    }

    let mut nodes = Vec::with_capacity(docs.len());
    let mut clusters = HashMap::<String, Vec<[f32; 3]>>::new();
    for (idx, doc) in docs.iter().enumerate() {
        let angle = (usize_to_f32_saturating(idx) / usize_to_f32_saturating(docs.len()))
            * std::f32::consts::TAU;
        let radius = 14.0 + (usize_to_f32_saturating(idx % 5) * 2.5);
        let position = [
            radius * angle.cos(),
            radius * angle.sin(),
            usize_to_f32_saturating(idx % 7) - 3.0,
        ];
        let cluster_id = cluster_id_for_path(doc.path.as_str());
        clusters
            .entry(cluster_id.clone())
            .or_default()
            .push(position);
        nodes.push(TopologyNode {
            id: doc.path.clone(),
            name: display_label(&doc.title, &doc.stem),
            node_type: classify_node_type(doc.path.as_str()),
            position,
            cluster_id: Some(cluster_id),
        });
    }

    let mut links = Vec::new();
    let mut seen_edges = HashSet::new();
    for doc in &docs {
        for neighbor in index.neighbors(doc.path.as_str(), LinkGraphDirection::Outgoing, 1, 32) {
            let edge_key = format!("{}->{}", doc.path, neighbor.path);
            if seen_edges.insert(edge_key) {
                links.push(TopologyLink {
                    from: doc.path.clone(),
                    to: neighbor.path,
                    label: None,
                });
            }
        }
    }

    let clusters = clusters
        .into_iter()
        .map(|(id, positions)| {
            let node_count = positions.len();
            let (sum_x, sum_y, sum_z) = positions.iter().fold((0.0, 0.0, 0.0), |acc, point| {
                (acc.0 + point[0], acc.1 + point[1], acc.2 + point[2])
            });
            ClusterInfo {
                id: id.clone(),
                name: id.clone(),
                centroid: [
                    sum_x / usize_to_f32_saturating(node_count),
                    sum_y / usize_to_f32_saturating(node_count),
                    sum_z / usize_to_f32_saturating(node_count),
                ],
                node_count,
                color: cluster_color(id.as_str()),
            }
        })
        .collect();

    Ok(Topology3D {
        nodes,
        links,
        clusters,
    })
}

fn resolve_metadata(
    state: &GatewayState,
    index: &crate::link_graph::LinkGraphIndex,
    id: &str,
) -> Option<LinkGraphMetadata> {
    graph_lookup_candidates(state.studio.as_ref(), id)
        .into_iter()
        .find_map(|candidate| {
            index.metadata(candidate.as_str()).or_else(|| {
                index
                    .resolve_metadata_candidates(candidate.as_str())
                    .into_iter()
                    .next()
            })
        })
}

fn display_name(metadata: &LinkGraphMetadata) -> String {
    display_label(&metadata.title, &metadata.stem)
}

fn display_label(title: &str, fallback: &str) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn classify_node_type(path: &str) -> String {
    if Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
        || path.contains("skill")
    {
        "skill".to_string()
    } else if path.contains("knowledge") {
        "knowledge".to_string()
    } else if has_case_insensitive_extension(path, "md")
        || has_case_insensitive_extension(path, "markdown")
        || has_case_insensitive_extension(path, "bpmn")
    {
        "doc".to_string()
    } else {
        "other".to_string()
    }
}

fn direction_to_string(direction: LinkGraphDirection) -> String {
    match direction {
        LinkGraphDirection::Incoming => "incoming".to_string(),
        LinkGraphDirection::Outgoing => "outgoing".to_string(),
        LinkGraphDirection::Both => "bidirectional".to_string(),
    }
}

fn cluster_id_for_path(path: &str) -> String {
    path.split('/').next().unwrap_or("root").to_string()
}

fn cluster_color(cluster_id: &str) -> String {
    let palette = [
        "#7dcfff", "#73daca", "#f7768e", "#e0af68", "#bb9af7", "#9ece6a",
    ];
    let mut hash = 0usize;
    for byte in cluster_id.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(usize::from(*byte));
    }
    palette[hash % palette.len()].to_string()
}

fn has_case_insensitive_extension(path: &str, extension: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
}

fn usize_to_f32_saturating(value: usize) -> f32 {
    u16::try_from(value).map_or(f32::from(u16::MAX), f32::from)
}

#[cfg(test)]
#[path = "../../../tests/unit/gateway/studio/graph.rs"]
mod tests;
