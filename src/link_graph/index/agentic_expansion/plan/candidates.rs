use super::ExpansionCandidateDoc;
use crate::link_graph::index::{LinkGraphIndex, LinkGraphSearchOptions};
use crate::link_graph::saliency::{learned_saliency_signal_from_state, valkey_saliency_get_many};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

pub(super) fn collect_agentic_expansion_candidates(
    index: &LinkGraphIndex,
    query: Option<&str>,
    max_candidates: usize,
) -> Vec<ExpansionCandidateDoc> {
    let bounded_limit = max_candidates.max(1);
    let mut candidate_ids: Vec<String> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    if let Some(query_text) = query {
        let (_, hits) =
            index.search_planned(query_text, bounded_limit, LinkGraphSearchOptions::default());
        for hit in hits {
            let resolved_id = index
                .resolve_doc_id(&hit.path)
                .or_else(|| index.resolve_doc_id(&hit.stem));
            if let Some(doc_id) = resolved_id.map(str::to_string)
                && seen_ids.insert(doc_id.clone())
            {
                candidate_ids.push(doc_id);
            }
            if candidate_ids.len() >= bounded_limit {
                break;
            }
        }
    }

    if candidate_ids.is_empty() {
        let mut ranked_doc_ids: Vec<(String, f64)> = index
            .docs_by_id
            .keys()
            .map(|doc_id| {
                (
                    doc_id.clone(),
                    index.rank_by_id.get(doc_id).copied().unwrap_or(0.0),
                )
            })
            .collect();
        ranked_doc_ids.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(Ordering::Equal)
                .then(left.0.cmp(&right.0))
        });
        candidate_ids.extend(
            ranked_doc_ids
                .into_iter()
                .take(bounded_limit)
                .map(|row| row.0),
        );
    }

    let saliency_signals = load_saliency_signals(&candidate_ids);

    candidate_ids
        .into_iter()
        .filter_map(|doc_id| {
            index
                .docs_by_id
                .get(&doc_id)
                .map(|doc| ExpansionCandidateDoc {
                    doc_id: doc_id.clone(),
                    rank: index.rank_by_id.get(&doc_id).copied().unwrap_or(0.0),
                    saliency_signal: saliency_signals.get(&doc_id).copied().unwrap_or(0.0),
                    tags: doc.tags_lower.iter().cloned().collect(),
                })
        })
        .collect()
}

pub(super) fn has_direct_edge(index: &LinkGraphIndex, left_id: &str, right_id: &str) -> bool {
    index
        .outgoing
        .get(left_id)
        .is_some_and(|targets| targets.contains(right_id))
        || index
            .outgoing
            .get(right_id)
            .is_some_and(|targets| targets.contains(left_id))
}

pub(super) fn agentic_pair_priority(
    left: &ExpansionCandidateDoc,
    right: &ExpansionCandidateDoc,
) -> f64 {
    let rank_signal = f64::midpoint(left.rank, right.rank).clamp(0.0, 1.0);
    let tag_signal = if left.tags.is_empty() || right.tags.is_empty() {
        0.0
    } else {
        let shared = usize_to_f64_saturating(left.tags.intersection(&right.tags).count());
        let denom = usize_to_f64_saturating(left.tags.len().min(right.tags.len()));
        if denom > 0.0 {
            (shared / denom).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    let semantic_score = (rank_signal * 0.7 + tag_signal * 0.3).clamp(0.0, 1.0);
    let saliency_factor =
        (1.0 + f64::midpoint(left.saliency_signal, right.saliency_signal)).clamp(1.0, 2.0);
    (semantic_score * saliency_factor).clamp(0.0, 1.0)
}

fn load_saliency_signals(candidate_ids: &[String]) -> HashMap<String, f64> {
    let Ok(states) = valkey_saliency_get_many(candidate_ids) else {
        return HashMap::new();
    };

    candidate_ids
        .iter()
        .filter_map(|doc_id| {
            states
                .get(doc_id)
                .map(|state| (doc_id.clone(), learned_saliency_signal_from_state(state)))
        })
        .collect()
}

fn usize_to_f64_saturating(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}
