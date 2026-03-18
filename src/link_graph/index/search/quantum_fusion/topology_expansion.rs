use super::scored_context::QuantumContextCandidate;
use super::scoring::topology_score_from_ranked;
use super::semantic_anchor::ResolvedQuantumAnchor;
use crate::link_graph::index::LinkGraphIndex;
use crate::link_graph::models::QuantumFusionOptions;
use crate::link_graph::saliency::{
    DEFAULT_SALIENCY_BASE, LinkGraphSaliencyPolicy, learned_saliency_signal_from_state,
    valkey_saliency_get_many,
};
use std::collections::HashMap;

const QUANTUM_RELATED_LIMIT_MAX_MULTIPLIER: usize = 2;

impl LinkGraphIndex {
    pub(super) fn expand_quantum_context_candidates(
        &self,
        resolved_anchors: &[ResolvedQuantumAnchor],
        options: &QuantumFusionOptions,
    ) -> Vec<QuantumContextCandidate> {
        let related_limits = resolve_quantum_related_limits(
            &resolved_anchors
                .iter()
                .map(|anchor| anchor.doc_id.clone())
                .collect::<Vec<_>>(),
            options.related_limit,
        );
        let mut candidates = Vec::new();
        for anchor in resolved_anchors {
            let effective_related_limit = quantum_related_limit_for_doc(
                anchor.doc_id.as_str(),
                &related_limits,
                options.related_limit,
            );
            let ranked = self.quantum_related_ranked_doc_ids(
                anchor.doc_id.as_str(),
                anchor.vector_score,
                options,
            );
            let related_clusters = collect_related_clusters(&ranked, effective_related_limit);
            let topology_score = topology_score_from_ranked(&ranked, effective_related_limit);

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

pub(super) fn resolve_quantum_related_limits(
    seed_doc_ids: &[String],
    base_limit: usize,
) -> HashMap<String, usize> {
    let normalized_base_limit = base_limit.max(1);
    let mut ordered_seed_doc_ids = seed_doc_ids
        .iter()
        .map(|doc_id| doc_id.trim())
        .filter(|doc_id| !doc_id.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    ordered_seed_doc_ids.sort_unstable();
    ordered_seed_doc_ids.dedup();

    let fallback_limits = ordered_seed_doc_ids
        .iter()
        .cloned()
        .map(|doc_id| (doc_id, normalized_base_limit))
        .collect::<HashMap<_, _>>();
    let Ok(states) = valkey_saliency_get_many(&ordered_seed_doc_ids) else {
        return fallback_limits;
    };

    ordered_seed_doc_ids
        .into_iter()
        .map(|doc_id| {
            let effective_limit = states.get(&doc_id).map_or(normalized_base_limit, |state| {
                boosted_quantum_related_limit(
                    normalized_base_limit,
                    learned_saliency_signal_from_state(state),
                )
            });
            (doc_id, effective_limit)
        })
        .collect()
}

pub(super) fn quantum_related_limit_for_doc(
    doc_id: &str,
    related_limits: &HashMap<String, usize>,
    base_limit: usize,
) -> usize {
    related_limits
        .get(doc_id)
        .copied()
        .unwrap_or_else(|| base_limit.max(1))
}

pub(super) fn collect_related_clusters(
    ranked: &[(String, usize, f64)],
    related_limit: usize,
) -> Vec<String> {
    ranked
        .iter()
        .take(related_limit.max(1))
        .map(|(doc_id, _, _)| doc_id.clone())
        .collect()
}

fn boosted_quantum_related_limit(base_limit: usize, saliency_signal: f64) -> usize {
    let normalized_base_limit = base_limit.max(1);
    let maximum_signal = LinkGraphSaliencyPolicy::default()
        .maximum
        .max(DEFAULT_SALIENCY_BASE);
    let bounded_signal = saliency_signal.clamp(DEFAULT_SALIENCY_BASE, maximum_signal);
    let normalized_window_signal = (bounded_signal - DEFAULT_SALIENCY_BASE)
        / (maximum_signal - DEFAULT_SALIENCY_BASE).max(f64::EPSILON);
    let extra_window = ceil_nonnegative_f64_to_usize(
        normalized_window_signal * usize_to_f64_saturating(normalized_base_limit),
    );
    normalized_base_limit.saturating_add(extra_window).clamp(
        normalized_base_limit,
        normalized_base_limit.saturating_mul(QUANTUM_RELATED_LIMIT_MAX_MULTIPLIER),
    )
}

fn usize_to_f64_saturating(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}

fn ceil_nonnegative_f64_to_usize(value: f64) -> usize {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }

    let capped = value.ceil().min(f64::from(u32::MAX));
    let integer = format!("{capped:.0}").parse::<u32>().unwrap_or(u32::MAX);
    usize::try_from(integer).unwrap_or(usize::MAX)
}
