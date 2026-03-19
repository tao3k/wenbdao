use crate::gateway::studio::types::{
    AnalysisEdge, AnalysisEdgeKind, AnalysisEvidence, AnalysisNode, AnalysisNodeKind,
};
use crate::zhenfa_router::native::sentinel::extract_pattern_symbols;

use super::compile::CompiledDocument;

struct EdgeSpec<'a> {
    path: &'a str,
    source_id: String,
    target_id: String,
    kind: AnalysisEdgeKind,
    label: Option<String>,
    line_no: usize,
    confidence: f64,
}

pub(crate) fn enrich_property_drawers(path: &str, content: &str, compiled: &mut CompiledDocument) {
    let mut next_edge_seq = compiled.edges.len() + 1;
    let section_nodes = compiled
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                AnalysisNodeKind::Document | AnalysisNodeKind::Section
            )
        })
        .map(|node| (node.line_start, node.depth, node.id.clone()))
        .collect::<Vec<_>>();

    let mut in_code_fence = false;
    let mut lines = content.lines().enumerate().peekable();
    while let Some((index, raw_line)) = lines.next() {
        let line_no = index + 1;
        let trimmed = raw_line.trim();

        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence || trimmed != ":PROPERTIES:" {
            continue;
        }

        let parent = property_parent(&section_nodes, line_no);
        let parent_depth = parent.1;
        let parent_id = parent.2.as_str();

        for (property_index, property_line) in lines.by_ref() {
            let property_line_no = property_index + 1;
            let property_trimmed = property_line.trim();
            if property_trimmed == ":END:" {
                break;
            }
            let Some((key, value)) = parse_property_line(property_trimmed) else {
                continue;
            };

            let (node_kind, node_id) = property_node_descriptor(key, property_line_no);
            push_node(
                compiled,
                node_id.clone(),
                node_kind,
                format!(":{key}: {value}"),
                parent_depth + 1,
                property_line_no,
                Some(parent_id.to_string()),
            );
            push_edge(
                compiled,
                &mut next_edge_seq,
                EdgeSpec {
                    path,
                    source_id: parent_id.to_string(),
                    target_id: node_id.clone(),
                    kind: AnalysisEdgeKind::Contains,
                    label: Some("contains".to_string()),
                    line_no: property_line_no,
                    confidence: 1.0,
                },
            );
            next_edge_seq += 1;

            if is_observation_key(key) {
                append_observation_symbols(
                    compiled,
                    &mut next_edge_seq,
                    path,
                    value,
                    property_line_no,
                    parent_depth + 2,
                    node_id.as_str(),
                );
            }
        }
    }
}

fn property_node_descriptor(key: &str, line_no: usize) -> (AnalysisNodeKind, String) {
    if is_observation_key(key) {
        (
            AnalysisNodeKind::Observation,
            format!("observe:{line_no}:{}", slugify(key)),
        )
    } else {
        (
            AnalysisNodeKind::Property,
            format!("prop:{line_no}:{}", slugify(key)),
        )
    }
}

fn append_observation_symbols(
    compiled: &mut CompiledDocument,
    next_edge_seq: &mut usize,
    path: &str,
    value: &str,
    line_no: usize,
    depth: usize,
    observation_id: &str,
) {
    for symbol in extract_pattern_symbols(value) {
        let symbol_id = format!("symbol:{line_no}:{}", slugify(symbol.as_str()));
        push_node(
            compiled,
            symbol_id.clone(),
            AnalysisNodeKind::Symbol,
            symbol.clone(),
            depth,
            line_no,
            Some(observation_id.to_string()),
        );
        push_edge(
            compiled,
            next_edge_seq,
            EdgeSpec {
                path,
                source_id: observation_id.to_string(),
                target_id: symbol_id,
                kind: AnalysisEdgeKind::References,
                label: Some(symbol),
                line_no,
                confidence: 0.95,
            },
        );
        *next_edge_seq += 1;
    }
}

fn push_node(
    compiled: &mut CompiledDocument,
    id: String,
    kind: AnalysisNodeKind,
    label: String,
    depth: usize,
    line_no: usize,
    parent_id: Option<String>,
) {
    compiled.nodes.push(AnalysisNode {
        id,
        kind,
        label,
        depth,
        line_start: line_no,
        line_end: line_no,
        parent_id,
    });
}

fn push_edge(compiled: &mut CompiledDocument, next_edge_seq: &mut usize, spec: EdgeSpec<'_>) {
    compiled.edges.push(AnalysisEdge {
        id: format!("edge:{}", *next_edge_seq),
        kind: spec.kind,
        source_id: spec.source_id,
        target_id: spec.target_id,
        label: spec.label,
        evidence: AnalysisEvidence {
            path: spec.path.to_string(),
            line_start: spec.line_no,
            line_end: spec.line_no,
            confidence: spec.confidence,
        },
    });
}

fn property_parent(
    section_nodes: &[(usize, usize, String)],
    line_no: usize,
) -> &(usize, usize, String) {
    section_nodes
        .iter()
        .filter(|(start_line, _, _)| *start_line < line_no)
        .max_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)))
        .unwrap_or_else(|| &section_nodes[0])
}

fn parse_property_line(line: &str) -> Option<(&str, &str)> {
    if !line.starts_with(':') {
        return None;
    }
    let remainder = &line[1..];
    let key_end = remainder.find(':')?;
    let key = remainder[..key_end].trim();
    let value = remainder[key_end + 1..].trim();
    if key.is_empty() || value.is_empty() {
        return None;
    }
    Some((key, value))
}

fn is_observation_key(key: &str) -> bool {
    key == "OBSERVE" || key.starts_with("OBSERVE_")
}

fn slugify(input: &str) -> String {
    input
        .chars()
        .flat_map(char::to_lowercase)
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
