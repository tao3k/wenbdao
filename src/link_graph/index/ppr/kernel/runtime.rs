use crate::link_graph::index::LinkGraphIndex;
use crate::link_graph::index::ppr::types::RelatedPprKernelResult;
use std::collections::HashMap;
use std::time::Instant;

use super::adjacency::{build_adjacency, build_node_index, build_passage_entity_adjacency};
use super::iteration::{build_restart_state, run_kernel_iterations};

impl LinkGraphIndex {
    pub(in crate::link_graph::index::ppr) fn run_related_ppr_kernel(
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
