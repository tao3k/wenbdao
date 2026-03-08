use super::scored_context::QuantumContextCandidate;
use super::scoring::topology_score_from_ranked;
use super::semantic_anchor::ResolvedQuantumAnchor;
use crate::link_graph::index::LinkGraphIndex;
use crate::link_graph::models::QuantumFusionOptions;
use std::collections::HashMap;

impl LinkGraphIndex {
    pub(super) fn expand_quantum_context_candidates(
        &self,
        resolved_anchors: &[ResolvedQuantumAnchor],
        options: &QuantumFusionOptions,
    ) -> Vec<QuantumContextCandidate> {
        let mut candidates = Vec::new();
        for anchor in resolved_anchors {
            let ranked = self.quantum_related_ranked_doc_ids(
                anchor.seed_doc_id.as_str(),
                anchor.vector_score,
                options,
            );
            let related_clusters = ranked
                .iter()
                .take(options.related_limit)
                .map(|(doc_id, _, _)| doc_id.clone())
                .collect::<Vec<_>>();
            let topology_score = topology_score_from_ranked(&ranked, options.related_limit);

            candidates.push(QuantumContextCandidate {
                batch_row: anchor.batch_row,
                batch_anchor_id: anchor.batch_anchor_id.clone(),
                anchor_id: anchor.anchor_id.clone(),
                semantic_path: anchor.semantic_path.clone(),
                related_clusters,
                vector_score: anchor.vector_score,
                topology_score,
            });
        }

        candidates
    }

    fn quantum_related_ranked_doc_ids(
        &self,
        seed_doc_id: &str,
        vector_score: f64,
        options: &QuantumFusionOptions,
    ) -> Vec<(String, usize, f64)> {
        let mut seeds = HashMap::new();
        seeds.insert(seed_doc_id.to_string(), vector_score.max(0.000_001));
        self.related_ppr_ranked_doc_ids(&seeds, options.max_distance, options.ppr.as_ref())
    }
}
