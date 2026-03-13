use super::super::{
    LinkGraphIndex, LinkGraphPprSubgraphMode, LinkGraphRelatedPprDiagnostics,
    LinkGraphRelatedPprOptions, doc_sort_key,
};
use super::runtime::resolve_related_ppr_runtime;
use super::types::{RelatedPprComputation, RelatedPprKernelResult};
use crate::link_graph::runtime_config::resolve_link_graph_related_runtime;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

impl LinkGraphIndex {
    pub(in crate::link_graph::index) fn related_ppr_compute(
        &self,
        seed_ids: &HashSet<String>,
        max_distance: usize,
        options: Option<&LinkGraphRelatedPprOptions>,
    ) -> Option<RelatedPprComputation> {
        let total_start = Instant::now();
        if seed_ids.is_empty() {
            return None;
        }
        let runtime = resolve_link_graph_related_runtime();
        let candidate_cap = runtime.max_candidates.max(1);
        let max_partitions = runtime.max_partitions.max(1);
        let time_budget_ms = runtime.time_budget_ms.max(1.0);
        let budget_duration = Duration::from_secs_f64(time_budget_ms / 1000.0);
        let deadline = Some(total_start + budget_duration);

        let bounded_distance = max_distance.max(1);
        let raw_horizon_distances =
            self.collect_bidirectional_distance_map(seed_ids, bounded_distance);
        if raw_horizon_distances.is_empty() {
            return None;
        }
        let raw_candidate_count =
            Self::candidate_count_from_horizon(&raw_horizon_distances, seed_ids);
        let candidate_capped = raw_candidate_count > candidate_cap;
        let horizon_distances = if candidate_capped {
            self.trim_horizon_candidates(&raw_horizon_distances, seed_ids, candidate_cap)
        } else {
            raw_horizon_distances
        };

        let (alpha, max_iter, tol, subgraph_mode) = resolve_related_ppr_runtime(options);
        let restrict_to_horizon = match subgraph_mode {
            LinkGraphPprSubgraphMode::Disabled => false,
            LinkGraphPprSubgraphMode::Force => true,
            LinkGraphPprSubgraphMode::Auto => horizon_distances.len() < self.docs_by_id.len(),
        };

        let graph_nodes =
            self.build_graph_nodes_for_related_ppr(&horizon_distances, restrict_to_horizon);
        if graph_nodes.is_empty() {
            return None;
        }
        let candidate_count = Self::candidate_count_from_horizon(&horizon_distances, seed_ids);

        let mut fused_scores_by_doc_id: HashMap<String, f64> = HashMap::new();
        let mut iteration_count = 0_usize;
        let mut final_residual = 0.0_f64;
        let mut subgraph_count = 0_usize;
        let mut partition_sizes: Vec<usize> = Vec::new();
        let mut partition_duration_ms = 0.0_f64;
        let mut kernel_duration_ms = 0.0_f64;
        let mut fusion_duration_ms = 0.0_f64;
        let mut timed_out = false;

        let mut should_partition = Self::should_partition_related_ppr(
            subgraph_mode,
            restrict_to_horizon,
            graph_nodes.len(),
            seed_ids.len(),
        );
        if should_partition && Self::deadline_exceeded(deadline) {
            timed_out = true;
            should_partition = false;
        }
        let mut seed_weights: HashMap<String, f64> = HashMap::with_capacity(seed_ids.len());
        for id in seed_ids {
            seed_weights.insert(id.clone(), 1.0);
        }

        if should_partition {
            let partition_start = Instant::now();
            let universe: HashSet<String> = graph_nodes.iter().cloned().collect();
            let partitions = self.build_related_ppr_partitions(
                seed_ids,
                bounded_distance,
                &universe,
                max_partitions,
            );
            partition_duration_ms = partition_start.elapsed().as_secs_f64() * 1000.0;
            partition_sizes = partitions.iter().map(Vec::len).collect();

            let kernel_start = Instant::now();
            let kernels: Vec<RelatedPprKernelResult> = partitions
                .par_iter()
                .filter_map(|partition_nodes| {
                    self.run_related_ppr_kernel(
                        partition_nodes,
                        &seed_weights,
                        alpha,
                        max_iter,
                        tol,
                        deadline,
                    )
                })
                .collect();
            kernel_duration_ms = kernel_start.elapsed().as_secs_f64() * 1000.0;
            if Self::deadline_exceeded(deadline) {
                timed_out = true;
            }

            let fusion_start = Instant::now();
            for kernel in kernels {
                subgraph_count += 1;
                iteration_count = iteration_count.max(kernel.iteration_count);
                final_residual = final_residual.max(kernel.final_residual);
                timed_out |= kernel.timed_out;
                for (doc_id, score) in kernel.scores_by_doc_id {
                    let current = fused_scores_by_doc_id.entry(doc_id).or_insert(0.0);
                    *current = current.max(score);
                }
            }
            fusion_duration_ms = fusion_start.elapsed().as_secs_f64() * 1000.0;
        }
        if subgraph_count == 0 {
            let kernel_start = Instant::now();
            let kernel = self.run_related_ppr_kernel(
                &graph_nodes,
                &seed_weights,
                alpha,
                max_iter,
                tol,
                deadline,
            )?;
            kernel_duration_ms = kernel_start.elapsed().as_secs_f64() * 1000.0;
            subgraph_count = 1;
            iteration_count = kernel.iteration_count;
            final_residual = kernel.final_residual;
            timed_out |= kernel.timed_out;
            fused_scores_by_doc_id = kernel.scores_by_doc_id;
            partition_sizes = vec![graph_nodes.len()];
        }
        let partition_max_node_count = partition_sizes.iter().copied().max().unwrap_or(0);
        let partition_min_node_count = partition_sizes.iter().copied().min().unwrap_or(0);
        let partition_avg_node_count = if partition_sizes.is_empty() {
            0.0
        } else {
            partition_sizes.iter().sum::<usize>() as f64 / partition_sizes.len() as f64
        };

        let mut ranked: Vec<(String, usize, f64)> = horizon_distances
            .into_iter()
            .filter(|(doc_id, distance)| *distance > 0 && !seed_ids.contains(doc_id))
            .filter_map(|(doc_id, distance)| {
                fused_scores_by_doc_id
                    .get(&doc_id)
                    .copied()
                    .map(|score| (doc_id, distance, score))
            })
            .collect();

        ranked.sort_by(|left, right| {
            right
                .2
                .partial_cmp(&left.2)
                .unwrap_or(Ordering::Equal)
                .then(left.1.cmp(&right.1))
                .then_with(
                    || match (self.docs_by_id.get(&left.0), self.docs_by_id.get(&right.0)) {
                        (Some(a), Some(b)) => doc_sort_key(a).cmp(&doc_sort_key(b)),
                        _ => left.0.cmp(&right.0),
                    },
                )
        });

        let diagnostics = LinkGraphRelatedPprDiagnostics {
            alpha,
            max_iter,
            tol,
            iteration_count,
            final_residual,
            candidate_count,
            candidate_cap,
            candidate_capped,
            graph_node_count: graph_nodes.len(),
            subgraph_count,
            partition_max_node_count,
            partition_min_node_count,
            partition_avg_node_count,
            total_duration_ms: total_start.elapsed().as_secs_f64() * 1000.0,
            partition_duration_ms,
            kernel_duration_ms,
            fusion_duration_ms,
            subgraph_mode,
            horizon_restricted: restrict_to_horizon,
            time_budget_ms,
            timed_out,
        };
        Some(RelatedPprComputation {
            ranked_doc_ids: ranked,
            diagnostics,
        })
    }

    pub(in crate::link_graph::index) fn related_ppr_ranked_doc_ids(
        &self,
        seed_ids: &HashSet<String>,
        max_distance: usize,
        options: Option<&LinkGraphRelatedPprOptions>,
    ) -> Vec<(String, usize, f64)> {
        self.related_ppr_compute(seed_ids, max_distance, options)
            .map(|row| row.ranked_doc_ids)
            .unwrap_or_default()
    }
}
