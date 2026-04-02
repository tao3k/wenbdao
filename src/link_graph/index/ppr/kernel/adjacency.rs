use crate::link_graph::index::LinkGraphIndex;
use std::collections::HashMap;
use std::sync::OnceLock;

use super::types::PassageEntityAdjacency;

pub(crate) fn build_node_index(graph_nodes: &[String]) -> HashMap<String, usize> {
    graph_nodes
        .iter()
        .cloned()
        .enumerate()
        .map(|(idx, doc_id)| (doc_id, idx))
        .collect()
}

pub(crate) fn build_passage_entity_adjacency(
    index: &LinkGraphIndex,
    node_to_idx: &HashMap<String, usize>,
) -> PassageEntityAdjacency {
    let mut passage_entities_by_idx: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut entity_passages_by_idx: HashMap<usize, Vec<usize>> = HashMap::new();
    if !passage_entity_edges_enabled() {
        return PassageEntityAdjacency {
            passage_entities_by_idx,
            entity_passages_by_idx,
        };
    }

    for (passage_id, passage) in &index.passages_by_id {
        let Some(passage_idx) = node_to_idx.get(passage_id).copied() else {
            continue;
        };
        for entity_id in &passage.entities {
            let Some(entity_idx) = node_to_idx.get(entity_id).copied() else {
                continue;
            };
            if entity_idx == passage_idx {
                continue;
            }
            passage_entities_by_idx
                .entry(passage_idx)
                .or_default()
                .push(entity_idx);
            entity_passages_by_idx
                .entry(entity_idx)
                .or_default()
                .push(passage_idx);
        }
    }

    dedup_index_lists(&mut passage_entities_by_idx);
    dedup_index_lists(&mut entity_passages_by_idx);

    PassageEntityAdjacency {
        passage_entities_by_idx,
        entity_passages_by_idx,
    }
}

pub(crate) fn build_adjacency(
    index: &LinkGraphIndex,
    graph_nodes: &[String],
    node_to_idx: &HashMap<String, usize>,
    passage_entity_adjacency: &PassageEntityAdjacency,
) -> Vec<Vec<usize>> {
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); graph_nodes.len()];
    let mut seen_epoch_by_idx: Vec<u32> = vec![0; graph_nodes.len()];
    let mut epoch: u32 = 1;

    for (source_idx, source_id) in graph_nodes.iter().enumerate() {
        if epoch == u32::MAX {
            seen_epoch_by_idx.fill(0);
            epoch = 1;
        }
        let current_epoch = epoch;
        epoch += 1;
        let outgoing_len = index
            .outgoing
            .get(source_id)
            .map_or(0, std::collections::HashSet::len);
        let incoming_len = index
            .incoming
            .get(source_id)
            .map_or(0, std::collections::HashSet::len);
        let mut neighbor_indices: Vec<usize> = Vec::with_capacity(outgoing_len + incoming_len);

        if let Some(targets) = index.outgoing.get(source_id) {
            for target_id in targets {
                if let Some(target_idx) = node_to_idx.get(target_id).copied() {
                    push_unique_neighbor(
                        &mut neighbor_indices,
                        &mut seen_epoch_by_idx,
                        current_epoch,
                        source_idx,
                        target_idx,
                    );
                }
            }
        }

        if let Some(sources) = index.incoming.get(source_id) {
            for source_neighbor_id in sources {
                if let Some(source_neighbor_idx) = node_to_idx.get(source_neighbor_id).copied() {
                    push_unique_neighbor(
                        &mut neighbor_indices,
                        &mut seen_epoch_by_idx,
                        current_epoch,
                        source_idx,
                        source_neighbor_idx,
                    );
                }
            }
        }

        if let Some(entity_indices) = passage_entity_adjacency
            .passage_entities_by_idx
            .get(&source_idx)
        {
            for &entity_idx in entity_indices {
                push_unique_neighbor(
                    &mut neighbor_indices,
                    &mut seen_epoch_by_idx,
                    current_epoch,
                    source_idx,
                    entity_idx,
                );
            }
        }

        if let Some(passage_indices) = passage_entity_adjacency
            .entity_passages_by_idx
            .get(&source_idx)
        {
            for &passage_idx in passage_indices {
                push_unique_neighbor(
                    &mut neighbor_indices,
                    &mut seen_epoch_by_idx,
                    current_epoch,
                    source_idx,
                    passage_idx,
                );
            }
        }

        adjacency[source_idx] = neighbor_indices;
    }

    adjacency
}

pub(crate) fn passage_entity_edges_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("XIUXIAN_WENDAO_LINK_GRAPH_ENABLE_PASSAGE_ENTITY_EDGES")
            .ok()
            .is_some_and(|raw| {
                matches!(
                    raw.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
    })
}

fn dedup_index_lists(index_map: &mut HashMap<usize, Vec<usize>>) {
    for edges in index_map.values_mut() {
        edges.sort_unstable();
        edges.dedup();
    }
}

fn push_unique_neighbor(
    neighbor_indices: &mut Vec<usize>,
    seen_epoch_by_idx: &mut [u32],
    current_epoch: u32,
    source_idx: usize,
    candidate_idx: usize,
) {
    if candidate_idx == source_idx || seen_epoch_by_idx[candidate_idx] == current_epoch {
        return;
    }
    seen_epoch_by_idx[candidate_idx] = current_epoch;
    neighbor_indices.push(candidate_idx);
}
