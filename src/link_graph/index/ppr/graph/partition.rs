use super::super::super::{LinkGraphIndex, LinkGraphPprSubgraphMode};
use super::super::constants::RELATED_PPR_PARTITION_TRIGGER_NODES;
use std::collections::HashSet;
use std::time::Instant;

impl LinkGraphIndex {
    pub(in crate::link_graph::index) fn deadline_exceeded(deadline: Option<Instant>) -> bool {
        deadline.is_some_and(|cutoff| Instant::now() >= cutoff)
    }

    pub(in crate::link_graph::index) fn should_partition_related_ppr(
        subgraph_mode: LinkGraphPprSubgraphMode,
        restrict_to_horizon: bool,
        graph_node_count: usize,
        seed_count: usize,
    ) -> bool {
        if !restrict_to_horizon || seed_count <= 1 {
            return false;
        }
        match subgraph_mode {
            LinkGraphPprSubgraphMode::Disabled => false,
            LinkGraphPprSubgraphMode::Force => true,
            LinkGraphPprSubgraphMode::Auto => {
                graph_node_count >= RELATED_PPR_PARTITION_TRIGGER_NODES
            }
        }
    }

    pub(in crate::link_graph::index) fn build_related_ppr_partitions(
        &self,
        seed_ids: &HashSet<String>,
        max_distance: usize,
        universe: &HashSet<String>,
        max_partitions: usize,
    ) -> Vec<Vec<String>> {
        let mut ordered_seeds: Vec<String> = seed_ids.iter().cloned().collect();
        self.sort_doc_ids_for_runtime(&mut ordered_seeds);
        if ordered_seeds.is_empty() {
            return Vec::new();
        }

        let mut seed_groups: Vec<HashSet<String>> = Vec::new();
        let capped_partitions = max_partitions.max(1);
        let direct_limit = capped_partitions.saturating_sub(1);
        if ordered_seeds.len() <= capped_partitions {
            for seed_id in ordered_seeds {
                let mut group: HashSet<String> = HashSet::new();
                group.insert(seed_id);
                seed_groups.push(group);
            }
        } else {
            for seed_id in ordered_seeds.iter().take(direct_limit) {
                let mut group: HashSet<String> = HashSet::new();
                group.insert(seed_id.clone());
                seed_groups.push(group);
            }
            let mut tail_group: HashSet<String> = HashSet::new();
            for seed_id in ordered_seeds.iter().skip(direct_limit) {
                tail_group.insert(seed_id.clone());
            }
            if !tail_group.is_empty() {
                seed_groups.push(tail_group);
            }
        }

        let mut partitions: Vec<Vec<String>> = Vec::new();
        let mut seen_keys: HashSet<String> = HashSet::new();
        for group in seed_groups {
            let horizon = self.collect_bidirectional_distance_map(&group, max_distance);
            if horizon.is_empty() {
                continue;
            }
            let mut nodes: Vec<String> = horizon
                .keys()
                .filter(|doc_id| universe.contains(*doc_id))
                .cloned()
                .collect();
            if nodes.is_empty() {
                continue;
            }
            self.sort_doc_ids_for_runtime(&mut nodes);
            let key = nodes.join("\x1f");
            if seen_keys.insert(key) {
                partitions.push(nodes);
            }
        }
        if partitions.is_empty() {
            let mut nodes: Vec<String> = universe.iter().cloned().collect();
            self.sort_doc_ids_for_runtime(&mut nodes);
            partitions.push(nodes);
        }
        partitions
    }
}
