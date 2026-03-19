use std::collections::HashMap;

use super::super::types::{
    AnalysisEdge, AnalysisEdgeKind, AnalysisNode, AnalysisNodeKind, MermaidProjection,
    MermaidViewKind,
};

pub(super) fn build_mermaid_projections(
    path: &str,
    nodes: &[AnalysisNode],
    edges: &[AnalysisEdge],
) -> Vec<MermaidProjection> {
    vec![
        build_mindmap_projection(path, nodes, edges),
        build_flowchart_projection(nodes, edges),
        build_graph_projection(nodes, edges),
    ]
}

fn build_mindmap_projection(
    path: &str,
    nodes: &[AnalysisNode],
    edges: &[AnalysisEdge],
) -> MermaidProjection {
    let mut source = String::from("mindmap\n");
    source.push_str("  root((");
    source.push_str(escape_mermaid_label(path).as_str());
    source.push_str("))\n");
    for node in nodes
        .iter()
        .filter(|node| !matches!(node.kind, AnalysisNodeKind::Document))
    {
        let indent = " ".repeat((node.depth + 1) * 2);
        source.push_str(indent.as_str());
        source.push_str(escape_mermaid_label(node.label.as_str()).as_str());
        source.push('\n');
    }
    MermaidProjection {
        kind: MermaidViewKind::Mindmap,
        source,
        node_count: nodes.len(),
        edge_count: edges.len(),
        complexity_score: complexity_score(nodes.len(), edges.len()),
        diagnostics: projection_diagnostics(nodes.len(), edges.len()),
    }
}

fn build_flowchart_projection(nodes: &[AnalysisNode], edges: &[AnalysisEdge]) -> MermaidProjection {
    let aliases = node_aliases(nodes);
    let mut source = String::from("flowchart TD\n");
    for node in nodes {
        let Some(alias) = aliases.get(node.id.as_str()) else {
            continue;
        };
        source.push_str("  ");
        source.push_str(alias.as_str());
        source.push_str("[\"");
        source.push_str(escape_mermaid_label(node.label.as_str()).as_str());
        source.push_str("\"]\n");
    }
    for edge in edges {
        let Some(source_alias) = aliases.get(edge.source_id.as_str()) else {
            continue;
        };
        let Some(target_alias) = aliases.get(edge.target_id.as_str()) else {
            continue;
        };
        source.push_str("  ");
        source.push_str(source_alias.as_str());
        source.push_str(" -->|");
        source.push_str(edge_label(edge.kind, edge.label.as_deref()).as_str());
        source.push_str("| ");
        source.push_str(target_alias.as_str());
        source.push('\n');
    }

    MermaidProjection {
        kind: MermaidViewKind::Flowchart,
        source,
        node_count: nodes.len(),
        edge_count: edges.len(),
        complexity_score: complexity_score(nodes.len(), edges.len()),
        diagnostics: projection_diagnostics(nodes.len(), edges.len()),
    }
}

fn build_graph_projection(nodes: &[AnalysisNode], edges: &[AnalysisEdge]) -> MermaidProjection {
    let aliases = node_aliases(nodes);
    let mut source = String::from("graph LR\n");
    for node in nodes {
        let Some(alias) = aliases.get(node.id.as_str()) else {
            continue;
        };
        source.push_str("  ");
        source.push_str(alias.as_str());
        source.push_str("[\"");
        source.push_str(escape_mermaid_label(node.label.as_str()).as_str());
        source.push_str("\"]\n");
    }
    for edge in edges.iter().filter(|edge| {
        matches!(
            edge.kind,
            AnalysisEdgeKind::References | AnalysisEdgeKind::NextStep | AnalysisEdgeKind::Contains
        )
    }) {
        let Some(source_alias) = aliases.get(edge.source_id.as_str()) else {
            continue;
        };
        let Some(target_alias) = aliases.get(edge.target_id.as_str()) else {
            continue;
        };
        source.push_str("  ");
        source.push_str(source_alias.as_str());
        source.push_str(" -->|");
        source.push_str(edge_label(edge.kind, edge.label.as_deref()).as_str());
        source.push_str("| ");
        source.push_str(target_alias.as_str());
        source.push('\n');
    }

    MermaidProjection {
        kind: MermaidViewKind::Graph,
        source,
        node_count: nodes.len(),
        edge_count: edges.len(),
        complexity_score: complexity_score(nodes.len(), edges.len()),
        diagnostics: projection_diagnostics(nodes.len(), edges.len()),
    }
}

fn node_aliases(nodes: &[AnalysisNode]) -> HashMap<&str, String> {
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), format!("N{index}")))
        .collect::<HashMap<_, _>>()
}

fn edge_label(kind: AnalysisEdgeKind, label: Option<&str>) -> String {
    let fallback = match kind {
        AnalysisEdgeKind::Contains => "contains",
        AnalysisEdgeKind::References => "references",
        AnalysisEdgeKind::NextStep => "next",
    };
    escape_mermaid_label(label.unwrap_or(fallback))
}

fn escape_mermaid_label(value: &str) -> String {
    value.replace('"', "'")
}

fn complexity_score(nodes: usize, edges: usize) -> f64 {
    usize_to_f64(nodes) + (usize_to_f64(edges) * 1.25)
}

fn projection_diagnostics(nodes: usize, edges: usize) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if nodes > 180 {
        diagnostics.push("high node count may reduce Mermaid render readability".to_string());
    }
    if edges > 260 {
        diagnostics.push("high edge count may cause graph overlap in small panels".to_string());
    }
    diagnostics
}

fn usize_to_f64(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}
