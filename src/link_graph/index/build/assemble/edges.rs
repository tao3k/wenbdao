use std::collections::{HashMap, HashSet};

use crate::link_graph::index::build::assemble::types::EdgeTables;
use crate::link_graph::parser::{ParsedNote, normalize_alias};

pub(crate) fn build_edge_tables(
    parsed_notes: &[ParsedNote],
    alias_to_doc_id: &HashMap<String, String>,
) -> EdgeTables {
    let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
    let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();
    let mut edge_count = 0usize;

    for parsed in parsed_notes {
        let from_id = &parsed.doc.id;

        for raw_target in &parsed.link_targets {
            let normalized = normalize_alias(raw_target);
            if normalized.is_empty() {
                continue;
            }
            let Some(to_id) = alias_to_doc_id.get(&normalized).cloned() else {
                continue;
            };
            if &to_id == from_id {
                continue;
            }
            let inserted = outgoing
                .entry(from_id.clone())
                .or_default()
                .insert(to_id.clone());
            if inserted {
                incoming.entry(to_id).or_default().insert(from_id.clone());
                edge_count += 1;
            }
        }

        for section in &parsed.sections {
            let source_node_id = if section.heading_path.is_empty() {
                from_id.clone()
            } else {
                format!("{}#{}", from_id, section.heading_path.replace(" / ", "/"))
            };
            let pd_edges = crate::link_graph::index::build::property_drawer_edges::extract_property_drawer_edges(
                &source_node_id,
                &section.attributes,
            );

            for edge in pd_edges {
                let normalized_target = normalize_alias(&edge.to);
                let Some(to_id) = alias_to_doc_id.get(&normalized_target).cloned() else {
                    continue;
                };
                if to_id == *from_id {
                    continue;
                }
                let inserted = outgoing
                    .entry(edge.from.clone())
                    .or_default()
                    .insert(to_id.clone());
                if inserted {
                    incoming.entry(to_id).or_default().insert(edge.from.clone());
                    edge_count += 1;
                }
            }
        }
    }

    EdgeTables {
        outgoing,
        incoming,
        edge_count,
    }
}
