use super::super::LinkGraphIndex;
use super::types::RelatedPprKernelResult;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

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
        let mut node_to_idx: HashMap<String, usize> = HashMap::with_capacity(graph_nodes.len());
        for (idx, doc_id) in graph_nodes.iter().enumerate() {
            node_to_idx.insert(doc_id.clone(), idx);
        }

        let mut passage_entities_by_idx: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut entity_passages_by_idx: HashMap<usize, Vec<usize>> = HashMap::new();
        if passage_entity_edges_enabled() {
            for (passage_id, passage) in &self.passages_by_id {
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
            for edges in passage_entities_by_idx.values_mut() {
                edges.sort_unstable();
                edges.dedup();
            }
            for edges in entity_passages_by_idx.values_mut() {
                edges.sort_unstable();
                edges.dedup();
            }
        }

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
            let outgoing_len = self
                .outgoing
                .get(source_id)
                .map_or(0, |targets| targets.len());
            let incoming_len = self
                .incoming
                .get(source_id)
                .map_or(0, |sources| sources.len());
            let mut neighbor_indices: Vec<usize> = Vec::with_capacity(outgoing_len + incoming_len);

            if let Some(targets) = self.outgoing.get(source_id) {
                for target_id in targets {
                    if let Some(target_idx) = node_to_idx.get(target_id).copied() {
                        if target_idx != source_idx
                            && seen_epoch_by_idx[target_idx] != current_epoch
                        {
                            seen_epoch_by_idx[target_idx] = current_epoch;
                            neighbor_indices.push(target_idx);
                        }
                    }
                }
            }

            if let Some(sources) = self.incoming.get(source_id) {
                for source_id in sources {
                    if let Some(source_neighbor_idx) = node_to_idx.get(source_id).copied() {
                        if source_neighbor_idx != source_idx
                            && seen_epoch_by_idx[source_neighbor_idx] != current_epoch
                        {
                            seen_epoch_by_idx[source_neighbor_idx] = current_epoch;
                            neighbor_indices.push(source_neighbor_idx);
                        }
                    }
                }
            }

            if let Some(entity_indices) = passage_entities_by_idx.get(&source_idx) {
                for &entity_idx in entity_indices {
                    if seen_epoch_by_idx[entity_idx] != current_epoch {
                        seen_epoch_by_idx[entity_idx] = current_epoch;
                        neighbor_indices.push(entity_idx);
                    }
                }
            }

            if let Some(passage_indices) = entity_passages_by_idx.get(&source_idx) {
                for &passage_idx in passage_indices {
                    if seen_epoch_by_idx[passage_idx] != current_epoch {
                        seen_epoch_by_idx[passage_idx] = current_epoch;
                        neighbor_indices.push(passage_idx);
                    }
                }
            }

            adjacency[source_idx] = neighbor_indices;
        }

        let mut teleport = vec![0.0_f64; graph_nodes.len()];
        let mut total_seed_weight = 0.0_f64;

        // 2026 Refinement: Map weighted seeds into teleport vector
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
        // L1 Normalization: ensure sum(teleport) == 1.0
        for (idx, value) in teleport.iter_mut().enumerate() {
            *value /= total_seed_weight;
            if *value > 0.0 {
                restart_nodes.push((idx, *value));
            }
        }

        let mut scores = teleport.clone();
        let mut next_scores = vec![0.0_f64; graph_nodes.len()];
        let mut iteration_count = 0_usize;
        let mut final_residual = 0.0_f64;
        let mut timed_out = false;
        for _ in 0..max_iter {
            next_scores.fill(0.0);
            let restart_scale = (1.0 - alpha).clamp(0.0, 1.0);
            for (idx, restart) in &restart_nodes {
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
                let step = alpha * source_score / outgoing.len() as f64;
                for &target_idx in outgoing {
                    next_scores[target_idx] += step;
                }
            }

            if dangling_mass > 0.0 {
                let leak = alpha * dangling_mass;
                for (idx, restart) in &restart_nodes {
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
            if Self::deadline_exceeded(deadline) {
                timed_out = true;
                break;
            }
        }

        let scores_by_doc_id: HashMap<String, f64> = graph_nodes
            .iter()
            .enumerate()
            .map(|(idx, doc_id)| (doc_id.clone(), scores[idx]))
            .collect();

        Some(RelatedPprKernelResult {
            scores_by_doc_id,
            iteration_count,
            final_residual,
            timed_out,
        })
    }
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
