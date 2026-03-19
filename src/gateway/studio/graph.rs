//! Graph operations for the studio API.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::link_graph::parser::code_observation::CodeObservation;
use crate::link_graph::{LinkGraphDirection, LinkGraphMetadata};

use super::analysis;
use super::pathing;
use super::router::{GatewayState, StudioApiError};
use super::search::{DefinitionMatchMode, DefinitionResolveOptions, resolve_best_definition};
use super::types::{
    AnalysisEdgeKind, AnalysisNodeKind, ClusterInfo, GraphLink, GraphNeighborsResponse, GraphNode,
    MarkdownAnalysisResponse, NodeNeighbors, StudioNavigationTarget, Topology3D, TopologyLink,
    TopologyNode,
};
use super::vfs::{graph_lookup_candidates, studio_display_path};

#[derive(Default)]
struct MarkdownSymbolNavigationHints {
    scopes: Vec<String>,
    languages: Vec<String>,
}

/// Get immediate neighbors for a node.
pub(crate) async fn node_neighbors(
    state: &GatewayState,
    id: &str,
) -> Result<NodeNeighbors, StudioApiError> {
    let index = state.link_graph_index().await?;
    let metadata = resolve_metadata(state, index.as_ref(), id)
        .ok_or_else(|| StudioApiError::not_found(format!("Node not found: {id}")))?;
    let center_id = metadata.path.clone();
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let center_display_id = studio_display_path(state.studio.as_ref(), center_id.as_str());
    if !pathing::path_matches_project_file_filters(
        project_root.as_path(),
        config_root.as_path(),
        projects.as_slice(),
        center_display_id.as_str(),
    ) {
        return Err(StudioApiError::not_found(format!("Node not found: {id}")));
    }
    let incoming = index
        .neighbors(center_id.as_str(), LinkGraphDirection::Incoming, 1, 128)
        .into_iter()
        .filter_map(|neighbor| {
            let display_path = studio_display_path(state.studio.as_ref(), neighbor.path.as_str());
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                display_path.as_str(),
            )
            .then_some(display_path)
        })
        .collect();
    let outgoing = index
        .neighbors(center_id.as_str(), LinkGraphDirection::Outgoing, 1, 128)
        .into_iter()
        .filter_map(|neighbor| {
            let display_path = studio_display_path(state.studio.as_ref(), neighbor.path.as_str());
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                display_path.as_str(),
            )
            .then_some(display_path)
        })
        .collect();
    let two_hop = index
        .neighbors(center_id.as_str(), LinkGraphDirection::Both, 2, 256)
        .into_iter()
        .filter(|neighbor| neighbor.distance > 1)
        .filter_map(|neighbor| {
            let display_path = studio_display_path(state.studio.as_ref(), neighbor.path.as_str());
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                display_path.as_str(),
            )
            .then_some(display_path)
        })
        .collect();

    Ok(NodeNeighbors {
        node_id: center_display_id,
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
    let Some(metadata) = resolve_metadata(state, index.as_ref(), id) else {
        let mut fallback = markdown_graph_fallback(state, id).await?;
        let _ = decorate_markdown_graph_navigation(state, &mut fallback).await;
        return Ok(fallback);
    };
    let center_id = metadata.path.clone();
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let center_display_id = studio_display_path(state.studio.as_ref(), center_id.as_str());
    if !pathing::path_matches_project_file_filters(
        project_root.as_path(),
        config_root.as_path(),
        projects.as_slice(),
        center_display_id.as_str(),
    ) {
        return markdown_graph_fallback(state, id).await;
    }
    let direction = LinkGraphDirection::from_alias(direction);
    let neighbors = index
        .neighbors(center_id.as_str(), direction, hops, limit)
        .into_iter()
        .filter_map(|neighbor| {
            let display_path = studio_display_path(state.studio.as_ref(), neighbor.path.as_str());
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                display_path.as_str(),
            )
            .then_some((neighbor, display_path))
        })
        .collect::<Vec<_>>();

    let center = GraphNode {
        id: center_display_id.clone(),
        label: display_name(&metadata),
        path: center_display_id.clone(),
        navigation_target: Some(super::vfs::resolve_navigation_target(
            state.studio.as_ref(),
            center_display_id.as_str(),
        )),
        node_type: classify_node_type(center_id.as_str()),
        is_center: true,
        distance: 0,
    };
    let mut seen_nodes = HashSet::from([center_display_id.clone()]);
    let mut nodes = vec![center.clone()];
    let mut links = Vec::new();

    for (neighbor, neighbor_display_path) in neighbors {
        if seen_nodes.insert(neighbor_display_path.clone()) {
            nodes.push(GraphNode {
                id: neighbor_display_path.clone(),
                label: display_label(&neighbor.title, &neighbor.stem),
                path: neighbor_display_path.clone(),
                navigation_target: Some(super::vfs::resolve_navigation_target(
                    state.studio.as_ref(),
                    neighbor_display_path.as_str(),
                )),
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
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let docs = index
        .toc(128)
        .into_iter()
        .filter(|doc| {
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                doc.path.as_str(),
            )
        })
        .collect::<Vec<_>>();
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
        for neighbor in index
            .neighbors(doc.path.as_str(), LinkGraphDirection::Outgoing, 1, 32)
            .into_iter()
            .filter(|neighbor| {
                pathing::path_matches_project_file_filters(
                    project_root.as_path(),
                    config_root.as_path(),
                    projects.as_slice(),
                    neighbor.path.as_str(),
                )
            })
        {
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

async fn markdown_graph_fallback(
    state: &GatewayState,
    path: &str,
) -> Result<GraphNeighborsResponse, StudioApiError> {
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    if !pathing::path_matches_project_file_filters(
        project_root.as_path(),
        config_root.as_path(),
        projects.as_slice(),
        path,
    ) {
        return Err(StudioApiError::not_found(format!("Node not found: {path}")));
    }

    let analysis = analysis::analyze_markdown(state.studio.as_ref(), path)
        .await
        .map_err(|_| StudioApiError::not_found(format!("Node not found: {path}")))?;
    let mut response = graph_neighbors_from_markdown_analysis(&analysis);
    decorate_markdown_graph_navigation(state, &mut response).await?;
    Ok(response)
}

fn graph_neighbors_from_markdown_analysis(
    analysis: &MarkdownAnalysisResponse,
) -> GraphNeighborsResponse {
    let analysis_center_node = analysis
        .nodes
        .iter()
        .find(|node| matches!(node.kind, AnalysisNodeKind::Document))
        .or_else(|| analysis.nodes.iter().min_by_key(|node| node.depth));

    let base_navigation_target = StudioNavigationTarget {
        path: analysis.path.clone(),
        category: classify_node_type(analysis.path.as_str()),
        project_name: None,
        root_label: None,
        line: None,
        line_end: None,
        column: None,
    };
    let mut nodes = analysis
        .nodes
        .iter()
        .map(|node| GraphNode {
            id: node.id.clone(),
            label: node.label.clone(),
            path: analysis.path.clone(),
            navigation_target: Some(StudioNavigationTarget {
                line: Some(node.line_start),
                line_end: Some(node.line_end),
                column: Some(1),
                ..base_navigation_target.clone()
            }),
            node_type: markdown_graph_node_type(node.kind),
            is_center: analysis_center_node.is_some_and(|center| center.id == node.id),
            distance: node.depth,
        })
        .collect::<Vec<_>>();

    if nodes.is_empty() {
        nodes.push(GraphNode {
            id: analysis.path.clone(),
            label: Path::new(analysis.path.as_str())
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(analysis.path.as_str())
                .to_string(),
            path: analysis.path.clone(),
            navigation_target: Some(StudioNavigationTarget {
                line: Some(1),
                line_end: Some(1),
                column: Some(1),
                ..base_navigation_target.clone()
            }),
            node_type: "doc".to_string(),
            is_center: true,
            distance: 0,
        });
    }

    if !nodes.iter().any(|node| node.is_center)
        && let Some(first) = nodes.first_mut()
    {
        first.is_center = true;
    }

    let node_depths = analysis
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.depth))
        .collect::<HashMap<_, _>>();
    let node_ids = analysis
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    let links = analysis
        .edges
        .iter()
        .filter(|edge| {
            node_ids.contains(edge.source_id.as_str()) && node_ids.contains(edge.target_id.as_str())
        })
        .map(|edge| GraphLink {
            source: edge.source_id.clone(),
            target: edge.target_id.clone(),
            direction: markdown_graph_direction(edge.kind),
            distance: usize::max(
                *node_depths.get(edge.source_id.as_str()).unwrap_or(&0),
                *node_depths.get(edge.target_id.as_str()).unwrap_or(&0),
            ),
        })
        .collect::<Vec<_>>();
    let center = nodes
        .iter()
        .find(|node| node.is_center)
        .cloned()
        .unwrap_or_else(|| nodes[0].clone());

    GraphNeighborsResponse {
        center,
        total_nodes: nodes.len(),
        total_links: links.len(),
        nodes,
        links,
    }
}

async fn decorate_markdown_graph_navigation(
    state: &GatewayState,
    response: &mut GraphNeighborsResponse,
) -> Result<(), StudioApiError> {
    let ast_index = state.studio.ast_index().await?;
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let navigation_hints = markdown_symbol_navigation_hints(response);

    for node in &mut response.nodes {
        if !node.id.contains("symbol:") {
            continue;
        }

        let Some(definition) = resolve_best_definition(
            project_root.as_path(),
            config_root.as_path(),
            projects.as_slice(),
            ast_index.as_slice(),
            node.label.as_str(),
            DefinitionResolveOptions {
                scope_patterns: navigation_hints
                    .get(node.id.as_str())
                    .map(|hints| hints.scopes.as_slice()),
                languages: navigation_hints
                    .get(node.id.as_str())
                    .map(|hints| hints.languages.as_slice()),
                match_mode: DefinitionMatchMode::ExactOnly,
                include_markdown: false,
                ..DefinitionResolveOptions::default()
            },
        ) else {
            continue;
        };

        let mut navigation_target = definition.navigation_target.clone();
        navigation_target.path =
            studio_display_path(state.studio.as_ref(), definition.path.as_str());
        navigation_target.line = Some(definition.line_start);
        navigation_target.line_end = Some(definition.line_end);
        navigation_target.column = Some(1);
        node.navigation_target = Some(navigation_target);
    }

    if let Some(center) = response.nodes.iter().find(|node| node.is_center).cloned() {
        response.center = center;
    }

    Ok(())
}

fn markdown_symbol_navigation_hints(
    response: &GraphNeighborsResponse,
) -> HashMap<String, MarkdownSymbolNavigationHints> {
    let observation_hints = response
        .nodes
        .iter()
        .filter_map(|node| {
            observation_hints_from_label(node.label.as_str()).map(|hints| (node.id.as_str(), hints))
        })
        .collect::<HashMap<_, _>>();

    let mut symbol_hints = HashMap::<String, MarkdownSymbolNavigationHints>::new();
    for link in &response.links {
        let Some(hints) = observation_hints.get(link.source.as_str()) else {
            continue;
        };
        let entry = symbol_hints.entry(link.target.clone()).or_default();
        if let Some(scope) = hints.scope.as_ref() {
            entry.scopes.push(scope.clone());
        }
        entry.languages.push(hints.language.clone());
    }
    symbol_hints
}

fn observation_hints_from_label(label: &str) -> Option<CodeObservation> {
    let value = observation_value_from_label(label)?;
    CodeObservation::parse(value)
}

fn observation_value_from_label(label: &str) -> Option<&str> {
    if !label.starts_with(':') {
        return None;
    }
    let remainder = &label[1..];
    let key_end = remainder.find(':')?;
    let key = remainder[..key_end].trim();
    if key != "OBSERVE" && !key.starts_with("OBSERVE_") {
        return None;
    }
    let value = remainder[key_end + 1..].trim();
    if value.is_empty() { None } else { Some(value) }
}

fn markdown_graph_node_type(kind: AnalysisNodeKind) -> String {
    match kind {
        AnalysisNodeKind::Document | AnalysisNodeKind::Section => "doc".to_string(),
        AnalysisNodeKind::Symbol
        | AnalysisNodeKind::Property
        | AnalysisNodeKind::Observation
        | AnalysisNodeKind::Task
        | AnalysisNodeKind::Reference => "knowledge".to_string(),
        AnalysisNodeKind::CodeBlock => "other".to_string(),
    }
}

fn markdown_graph_direction(kind: AnalysisEdgeKind) -> String {
    match kind {
        AnalysisEdgeKind::Contains => "outgoing".to_string(),
        AnalysisEdgeKind::References => "references".to_string(),
        AnalysisEdgeKind::NextStep => "next_step".to_string(),
    }
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
