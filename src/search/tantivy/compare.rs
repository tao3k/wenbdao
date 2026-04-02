use std::cmp::Ordering;

use crate::search::fuzzy::{FuzzyScore, FuzzySearchOptions, score_candidate_with_query_chars};

use super::fragments::for_each_candidate_fragment;

#[allow(clippy::too_many_arguments)]
pub(super) fn best_match_candidate(
    query: &str,
    query_chars: &[char],
    stored_text: &str,
    options: FuzzySearchOptions,
    candidate_chars: &mut Vec<char>,
    scratch: &mut Vec<usize>,
    seen_ranges: &mut Vec<(usize, usize)>,
    boundary_scratch: &mut Vec<usize>,
) -> Option<(String, FuzzyScore)> {
    let mut best: Option<(&str, FuzzyScore)> = None;

    for_each_candidate_fragment(stored_text, seen_ranges, boundary_scratch, |candidate| {
        let Some(score) = score_candidate_with_query_chars(
            query,
            query_chars,
            candidate,
            options,
            candidate_chars,
            scratch,
        ) else {
            return;
        };

        let replace = match best.as_ref() {
            None => true,
            Some((best_text, best_score)) => {
                compare_candidate(candidate, score, best_text, *best_score).is_lt()
            }
        };

        if replace {
            best = Some((candidate, score));
        }
    });

    best.map(|(candidate, score)| (candidate.to_string(), score))
}

fn compare_candidate(
    left_text: &str,
    left_score: FuzzyScore,
    right_text: &str,
    right_score: FuzzyScore,
) -> Ordering {
    right_score
        .score
        .partial_cmp(&left_score.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left_score.distance.cmp(&right_score.distance))
        .then_with(|| left_text.len().cmp(&right_text.len()))
        .then_with(|| left_text.cmp(right_text))
}

pub(super) fn collect_lowercase_chars(value: &str, target: &mut Vec<char>) {
    target.clear();
    target.extend(value.chars().flat_map(char::to_lowercase));
}
