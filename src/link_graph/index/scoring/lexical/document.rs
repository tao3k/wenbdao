use super::helpers::{count_substring_occurrences, doc_contains_token};
use crate::link_graph::index::LinkGraphDocument;

fn bounded_usize_to_f64(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}

fn bounded_ratio(numerator: usize, denominator: usize) -> f64 {
    bounded_usize_to_f64(numerator) / bounded_usize_to_f64(denominator)
}

pub(in crate::link_graph::index) fn score_document(
    doc: &LinkGraphDocument,
    query: &str,
    query_tokens: &[String],
    case_sensitive: bool,
) -> f64 {
    if query.is_empty() {
        return 0.0;
    }
    let (id_value, stem_value, title_value, path_value, content_value, tags_value): (
        &str,
        &str,
        &str,
        &str,
        &str,
        &[String],
    ) = if case_sensitive {
        (
            doc.id.as_str(),
            doc.stem.as_str(),
            doc.title.as_str(),
            doc.path.as_str(),
            doc.search_text.as_str(),
            doc.tags.as_slice(),
        )
    } else {
        (
            doc.id_lower.as_str(),
            doc.stem_lower.as_str(),
            doc.title_lower.as_str(),
            doc.path_lower.as_str(),
            doc.search_text_lower.as_str(),
            doc.tags_lower.as_slice(),
        )
    };

    let mut score: f64 = 0.0;
    if id_value == query || stem_value == query {
        score = score.max(1.0);
    }
    if title_value == query {
        score = score.max(0.95);
    }
    if tags_value.iter().any(|tag| tag == query) {
        score = score.max(0.85);
    }
    if id_value.contains(query)
        || stem_value.contains(query)
        || title_value.contains(query)
        || path_value.contains(query)
        || tags_value.iter().any(|tag| tag.contains(query))
        || content_value.contains(query)
    {
        score = score.max(0.7);
    }

    if query_tokens.len() >= 2 {
        let mut phrase_hits = 0usize;
        if id_value.contains(query) || stem_value.contains(query) || path_value.contains(query) {
            phrase_hits += 2;
        }
        if title_value.contains(query) {
            phrase_hits += 3;
        }
        phrase_hits += tags_value
            .iter()
            .filter(|tag| tag.contains(query))
            .count()
            .saturating_mul(2);

        let content_occurrences = count_substring_occurrences(content_value, query);
        phrase_hits += content_occurrences.min(6);

        if phrase_hits > 0 {
            let mut phrase_score = 0.70 + bounded_usize_to_f64(phrase_hits.min(8)) * 0.03;
            if doc.word_count > 0 && content_occurrences > 0 {
                let density = (bounded_usize_to_f64(content_occurrences)
                    * bounded_usize_to_f64(query_tokens.len())
                    / bounded_usize_to_f64(doc.word_count))
                .clamp(0.0, 0.08);
                phrase_score += density;
            }
            score = score.max(phrase_score.clamp(0.0, 0.97));
        }
    }

    if !query_tokens.is_empty() {
        let mut matched = 0usize;
        for token in query_tokens {
            if token.is_empty() {
                continue;
            }
            if doc_contains_token(doc, token, case_sensitive) {
                matched += 1;
            }
        }
        if matched > 0 {
            let ratio = bounded_ratio(matched, query_tokens.len());
            let token_score = if query_tokens.len() >= 2 {
                0.33 + ratio * 0.42
            } else {
                0.45 + ratio * 0.45
            };
            score = score.max(token_score);
        }
    }
    score.clamp(0.0, 1.0)
}
