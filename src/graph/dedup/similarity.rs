use super::super::KnowledgeGraph;
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

impl KnowledgeGraph {
    /// Calculate similarity between two entity names (0.0 to 1.0).
    #[must_use]
    pub fn name_similarity(name1: &str, name2: &str) -> f32 {
        let n1 = normalize_name(name1);
        let n2 = normalize_name(name2);

        if n1 == n2 {
            return 1.0;
        }

        // Exact substring match
        if n1.contains(&n2) || n2.contains(&n1) {
            return 0.9;
        }

        // Levenshtein-based similarity
        let max_len = std::cmp::max(n1.len(), n2.len());
        if max_len == 0 {
            return 1.0;
        }

        let distance = levenshtein_distance(&n1, &n2);
        let distance_u16 = u16::try_from(distance).unwrap_or(u16::MAX);
        let max_len_u16 = u16::try_from(max_len).unwrap_or(u16::MAX);
        let similarity = 1.0 - (f32::from(distance_u16) / f32::from(max_len_u16));

        // Apply bonus for word overlap
        let words1: HashSet<&str> = n1.split_whitespace().collect();
        let words2: HashSet<&str> = n2.split_whitespace().collect();
        let overlap_count = words1.intersection(&words2).count();
        let overlap = f32::from(u16::try_from(overlap_count).unwrap_or(u16::MAX));
        let word_bonus = if !words1.is_empty() && !words2.is_empty() {
            let total_words = u16::try_from(words1.len() + words2.len()).unwrap_or(u16::MAX);
            overlap / f32::from(total_words) * 0.2
        } else {
            0.0
        };

        (similarity + word_bonus).clamp(0.0, 1.0)
    }
}

/// Normalize entity name for comparison (Unicode NFKC + lowercase).
fn normalize_name(name: &str) -> String {
    let normalized: String = name.nfkc().collect();
    normalized
        .to_lowercase()
        .trim()
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
}

/// Calculate Levenshtein distance between two strings.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let (m, n) = (a_chars.len(), b_chars.len());

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = usize::from(a_chars[i - 1] != b_chars[j - 1]);
            let deletion = prev[j] + 1;
            let insertion = curr[j - 1] + 1;
            let substitution = prev[j - 1] + cost;
            curr[j] = deletion.min(insertion).min(substitution);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}
