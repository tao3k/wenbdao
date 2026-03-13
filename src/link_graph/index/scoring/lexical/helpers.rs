use crate::link_graph::index::LinkGraphDocument;

pub(super) fn doc_contains_token(
    doc: &LinkGraphDocument,
    token: &str,
    case_sensitive: bool,
) -> bool {
    if token.is_empty() {
        return false;
    }
    if case_sensitive {
        doc.id.contains(token)
            || doc.stem.contains(token)
            || doc.title.contains(token)
            || doc.path.contains(token)
            || doc.tags.iter().any(|tag| tag.contains(token))
            || doc.search_text.contains(token)
    } else {
        doc.id_lower.contains(token)
            || doc.stem_lower.contains(token)
            || doc.title_lower.contains(token)
            || doc.path_lower.contains(token)
            || doc.tags_lower.iter().any(|tag| tag.contains(token))
            || doc.search_text_lower.contains(token)
    }
}

pub(super) fn count_substring_occurrences(haystack: &str, needle: &str) -> usize {
    if haystack.is_empty() || needle.is_empty() {
        return 0;
    }
    haystack.match_indices(needle).count()
}

pub(in crate::link_graph::index) fn token_match_ratio(
    haystack: &str,
    query_tokens: &[String],
) -> f64 {
    if query_tokens.is_empty() {
        return 0.0;
    }
    let mut matched = 0usize;
    for token in query_tokens {
        if token.is_empty() {
            continue;
        }
        if haystack.contains(token) {
            matched += 1;
        }
    }
    (matched as f64 / query_tokens.len() as f64).clamp(0.0, 1.0)
}
