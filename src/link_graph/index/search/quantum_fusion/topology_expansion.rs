use super::scored_context::QuantumContextCandidate;
use super::scoring::topology_score_from_ranked;
use super::semantic_anchor::ResolvedQuantumAnchor;
use crate::link_graph::index::LinkGraphIndex;
use crate::link_graph::models::QuantumFusionOptions;

impl LinkGraphIndex {
    pub(super) fn expand_quantum_context_candidates(
        &self,
        resolved_anchors: &[ResolvedQuantumAnchor],
        options: &QuantumFusionOptions,
    ) -> Vec<QuantumContextCandidate> {
        let mut candidates = Vec::new();
        for anchor in resolved_anchors {
            let ranked = self.quantum_related_ranked_doc_ids(
                anchor.doc_id.as_str(),
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
                doc_id: anchor.doc_id.clone(),
                path: anchor.path.clone(),
                semantic_path: anchor.semantic_path.clone(),
                trace_label: anchor.trace_label.clone(),
                related_clusters,
                vector_score: anchor.vector_score,
                topology_score,
            });
        }

        candidates
    }
}
