use super::super::LinkGraphIndex;
use super::types::RelatedPprKernelResult;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

struct PassageEntityAdjacency {
    passage_entities_by_idx: HashMap<usize, Vec<usize>>,
    entity_passages_by_idx: HashMap<usize, Vec<usize>>,
}

struct RestartState {
    teleport: Vec<f64>,
    restart_nodes: Vec<(usize, f64)>,
}

struct KernelIterationOutcome {
    scores: Vec<f64>,
    iteration_count: usize,
    final_residual: f64,
    timed_out: bool,
}

impl LinkGraphIndex {
    pub(super) fn run_related_ppr_kernel(
        &self,
        graph_nodes: &[String],
        seeds: &HashMap<String, f64>,
        alpha: f64,
        max_iter: usize,
        tol: f64,
        deadline: Option<Instant>,
    ) -> Option<RelatedPprKernelResult> {
        if graph_nodes.is_empty() {
            return None;
        }
        let node_to_idx = build_node_index(graph_nodes);
        let passage_entity_adjacency = build_passage_entity_adjacency(self, &node_to_idx);
        let adjacency = build_adjacency(self, graph_nodes, &node_to_idx, &passage_entity_adjacency);
        let restart_state = build_restart_state(graph_nodes.len(), seeds, &node_to_idx)?;
        let outcome =
            run_kernel_iterations(&adjacency, &restart_state, alpha, max_iter, tol, deadline);

        let scores_by_doc_id: HashMap<String, f64> = graph_nodes
            .iter()
            .enumerate()
            .map(|(idx, doc_id)| (doc_id.clone(), outcome.scores[idx]))
            .collect();

        Some(RelatedPprKernelResult {
            scores_by_doc_id,
            iteration_count: outcome.iteration_count,
            final_residual: outcome.final_residual,
            timed_out: outcome.timed_out,
        })
    }
}

fn build_node_index(graph_nodes: &[String]) -> HashMap<String, usize> {
    graph_nodes
        .iter()
        .cloned()
        .enumerate()
        .map(|(idx, doc_id)| (doc_id, idx))
        .collect()
}

fn build_passage_entity_adjacency(
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

fn dedup_index_lists(index_map: &mut HashMap<usize, Vec<usize>>) {
    for edges in index_map.values_mut() {
        edges.sort_unstable();
        edges.dedup();
    }
}

fn build_adjacency(
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

fn build_restart_state(
    graph_node_count: usize,
    seeds: &HashMap<String, f64>,
    node_to_idx: &HashMap<String, usize>,
) -> Option<RestartState> {
    let mut teleport = vec![0.0_f64; graph_node_count];
    let mut total_seed_weight = 0.0_f64;

    for (seed_id, &weight) in seeds {
        if let Some(seed_idx) = node_to_idx.get(seed_id).copied() {
            let safe_weight = weight.max(0.0);
            teleport[seed_idx] = safe_weight;
            total_seed_weight += safe_weight;
        }
    }

    if total_seed_weight <= 0.0 {
        return None;
    }

    let mut restart_nodes: Vec<(usize, f64)> = Vec::with_capacity(seeds.len());
    for (idx, value) in teleport.iter_mut().enumerate() {
        *value /= total_seed_weight;
        if *value > 0.0 {
            restart_nodes.push((idx, *value));
        }
    }

    Some(RestartState {
        teleport,
        restart_nodes,
    })
}

fn run_kernel_iterations(
    adjacency: &[Vec<usize>],
    restart_state: &RestartState,
    alpha: f64,
    max_iter: usize,
    tol: f64,
    deadline: Option<Instant>,
) -> KernelIterationOutcome {
    let mut scores = restart_state.teleport.clone();
    let mut next_scores = vec![0.0_f64; adjacency.len()];
    let mut iteration_count = 0_usize;
    let mut final_residual = 0.0_f64;
    let mut timed_out = false;

    for _ in 0..max_iter {
        next_scores.fill(0.0);
        let restart_scale = (1.0 - alpha).clamp(0.0, 1.0);
        for (idx, restart) in &restart_state.restart_nodes {
            next_scores[*idx] = restart_scale * *restart;
        }

        let mut dangling_mass = 0.0_f64;
        for (source_idx, outgoing) in adjacency.iter().enumerate() {
            let source_score = scores[source_idx];
            if source_score <= 0.0 {
                continue;
            }
            if outgoing.is_empty() {
                dangling_mass += source_score;
                continue;
            }
            let step = alpha * source_score / usize_to_f64_saturating(outgoing.len());
            for &target_idx in outgoing {
                next_scores[target_idx] += step;
            }
        }

        if dangling_mass > 0.0 {
            let leak = alpha * dangling_mass;
            for (idx, restart) in &restart_state.restart_nodes {
                next_scores[*idx] += leak * *restart;
            }
        }

        let residual: f64 = next_scores
            .iter()
            .zip(scores.iter())
            .map(|(next, current)| (next - current).abs())
            .sum();
        iteration_count += 1;
        final_residual = residual;
        std::mem::swap(&mut scores, &mut next_scores);
        if residual <= tol {
            break;
        }
        if LinkGraphIndex::deadline_exceeded(deadline) {
            timed_out = true;
            break;
        }
    }

    KernelIterationOutcome {
        scores,
        iteration_count,
        final_residual,
        timed_out,
    }
}

fn usize_to_f64_saturating(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}

fn passage_entity_edges_enabled() -> bool {
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
