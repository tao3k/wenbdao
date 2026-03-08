use crate::link_graph::agentic::{LinkGraphAgenticCandidatePair, LinkGraphAgenticWorkerPlan};
use std::collections::HashSet;

pub(super) fn partition_expansion_workers(
    pairs: &[LinkGraphAgenticCandidatePair],
    max_workers: usize,
    max_pairs_per_worker: usize,
) -> Vec<LinkGraphAgenticWorkerPlan> {
    if pairs.is_empty() {
        return Vec::new();
    }
    let bounded_workers = max_workers.max(1);
    let bounded_per_worker = max_pairs_per_worker.max(1);

    pairs
        .chunks(bounded_per_worker)
        .take(bounded_workers)
        .enumerate()
        .map(|(worker_id, chunk)| {
            let mut seed_seen: HashSet<String> = HashSet::new();
            let mut seed_ids: Vec<String> = Vec::new();
            for pair in chunk {
                for seed in [&pair.left_id, &pair.right_id] {
                    if seed_seen.insert(seed.clone()) {
                        seed_ids.push(seed.clone());
                    }
                }
            }
            let pairs = chunk.to_vec();
            let pair_count = pairs.len();
            LinkGraphAgenticWorkerPlan {
                worker_id,
                seed_ids,
                pairs,
                pair_count,
            }
        })
        .collect()
}
