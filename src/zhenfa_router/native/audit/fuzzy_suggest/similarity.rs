use crate::search::normalized_score;

use super::pattern::PatternSkeleton;

/// Calculate similarity score between two pattern skeletons.
///
/// Returns a score in [0.0, 1.0] where:
/// - 1.0 = perfect match
/// - 0.0 = no similarity
///
/// Weights (adjusted for fuzzy matching):
/// - Keywords: 35% (language keywords like fn, def, class)
/// - Structure: 25% (punctuation and operators)
/// - Identifiers: 30% (function/type names)
/// - Metavariables: 10% (pattern variables)
pub(crate) fn calculate_skeleton_similarity(
    original: &PatternSkeleton,
    candidate: &PatternSkeleton,
) -> f32 {
    // For keywords, we only care if they share the same base keyword (fn, def, class, etc.)
    // This is more lenient than full Jaccard similarity
    let keyword_score = if original.keywords.is_empty() || candidate.keywords.is_empty() {
        0.0
    } else {
        // Check if they share at least one important keyword (fn, def, class, struct, etc.)
        let has_common_keyword = original
            .keywords
            .iter()
            .any(|k| candidate.keywords.contains(k));
        if has_common_keyword {
            // Bonus for matching the primary keyword
            0.8 + (jaccard_similarity(&original.keywords, &candidate.keywords) * 0.2)
        } else {
            jaccard_similarity(&original.keywords, &candidate.keywords)
        }
    };

    // For structure, use a more lenient comparison
    // We care about basic structure (parentheses, braces) more than exact matches
    let structure_score = if original.structure.is_empty() && candidate.structure.is_empty() {
        1.0
    } else if original.structure.is_empty() || candidate.structure.is_empty() {
        0.3 // Give partial credit if one has structure but both are functions
    } else {
        // Check for basic structural elements (parentheses for functions)
        let orig_has_parens = original.structure.contains(&"(".to_string())
            && original.structure.contains(&")".to_string());
        let cand_has_parens = candidate.structure.contains(&"(".to_string())
            && candidate.structure.contains(&")".to_string());

        if orig_has_parens && cand_has_parens {
            // Both have function-like structure, give good score
            0.6 + (jaccard_similarity(&original.structure, &candidate.structure) * 0.4)
        } else {
            jaccard_similarity(&original.structure, &candidate.structure)
        }
    };

    let identifier_score = jaccard_similarity(&original.identifiers, &candidate.identifiers);
    let metavar_score = jaccard_similarity(&original.metavariables, &candidate.metavariables);

    // Weighted average with adjusted weights
    (keyword_score * 0.35)
        + (structure_score * 0.25)
        + (identifier_score * 0.30)
        + (metavar_score * 0.10)
}

/// Calculate Jaccard similarity between two sets.
pub(crate) fn jaccard_similarity<T: std::hash::Hash + Eq + std::borrow::Borrow<T>>(
    a: &[T],
    b: &[T],
) -> f32 {
    use std::collections::HashSet;

    let set_a: HashSet<&T> = a.iter().collect();
    let set_b: HashSet<&T> = b.iter().collect();

    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        0.0
    } else {
        bounded_ratio(intersection, union)
    }
}

/// Convert small heuristic counts into `f32` without unchecked casts.
pub(crate) fn bounded_ratio(numerator: usize, denominator: usize) -> f32 {
    let numerator = bounded_usize_to_f32(numerator);
    let denominator = bounded_usize_to_f32(denominator);
    numerator / denominator
}

/// Saturate large counts because fuzzy matching only needs stable heuristic ratios.
pub(crate) fn bounded_usize_to_f32(value: usize) -> f32 {
    u16::try_from(value).map_or(f32::from(u16::MAX), f32::from)
}

/// Calculate string similarity based on edit distance.
pub(crate) fn string_similarity(a: &str, b: &str) -> f32 {
    normalized_score(a, b, false)
}

/// Preserve the legacy helper shape expected by the unit tests.
#[cfg(test)]
pub(crate) fn levenshtein_distance(a: &str, b: &str) -> usize {
    crate::levenshtein_distance(a, b)
}
