use super::{
    INCOMING_RANK_FACTOR, LinkGraphDocument, LinkGraphIndex, MAX_GRAPH_RANK_BOOST,
    OUTGOING_RANK_FACTOR,
};
use std::collections::{HashMap, HashSet};

impl LinkGraphIndex {
    pub(in crate::link_graph::index) fn compute_rank_by_id(
        docs_by_id: &HashMap<String, LinkGraphDocument>,
        incoming: &HashMap<String, HashSet<String>>,
        outgoing: &HashMap<String, HashSet<String>>,
    ) -> HashMap<String, f64> {
        let mut raw_scores: HashMap<String, f64> = HashMap::with_capacity(docs_by_id.len());
        let mut max_raw = 0.0_f64;

        for doc_id in docs_by_id.keys() {
            let incoming_degree =
                usize_to_f64_saturating(incoming.get(doc_id).map_or(0_usize, HashSet::len));
            let outgoing_degree =
                usize_to_f64_saturating(outgoing.get(doc_id).map_or(0_usize, HashSet::len));
            let raw = (incoming_degree * INCOMING_RANK_FACTOR
                + outgoing_degree * OUTGOING_RANK_FACTOR)
                .ln_1p();
            max_raw = max_raw.max(raw);
            raw_scores.insert(doc_id.clone(), raw);
        }

        if max_raw > 0.0 {
            for value in raw_scores.values_mut() {
                *value /= max_raw;
            }
        }

        raw_scores
    }

    fn graph_rank(&self, doc_id: &str) -> f64 {
        self.rank_by_id.get(doc_id).copied().unwrap_or(0.0)
    }

    pub(in crate::link_graph::index) fn apply_graph_rank_boost(
        &self,
        doc_id: &str,
        score: f64,
    ) -> f64 {
        let rank = self.graph_rank(doc_id);
        if rank <= 0.0 {
            return score.clamp(0.0, 1.0);
        }
        let bounded = score.clamp(0.0, 1.0);
        (bounded + (1.0 - bounded) * rank * MAX_GRAPH_RANK_BOOST).clamp(0.0, 1.0)
    }
}

fn usize_to_f64_saturating(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}
