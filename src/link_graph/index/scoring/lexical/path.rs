use super::helpers::token_match_ratio;
use crate::link_graph::index::LinkGraphDocument;

pub(in crate::link_graph::index) fn score_path_fields(
    doc: &LinkGraphDocument,
    query: &str,
    query_tokens: &[String],
    case_sensitive: bool,
) -> f64 {
    if query.is_empty() {
        return 0.0;
    }
    let (id_value, stem_value, title_value, path_value): (&str, &str, &str, &str) =
        if case_sensitive {
            (
                doc.id.as_str(),
                doc.stem.as_str(),
                doc.title.as_str(),
                doc.path.as_str(),
            )
        } else {
            (
                doc.id_lower.as_str(),
                doc.stem_lower.as_str(),
                doc.title_lower.as_str(),
                doc.path_lower.as_str(),
            )
        };

    let mut score = 0.0_f64;
    if path_value == query || id_value == query || stem_value == query {
        score = score.max(1.0);
    } else if title_value == query {
        score = score.max(0.95);
    }

    if path_value.contains(query)
        || id_value.contains(query)
        || stem_value.contains(query)
        || title_value.contains(query)
    {
        score = score.max(0.82);
    }

    let path_blob = format!("{path_value} {id_value} {stem_value} {title_value}");
    let token_ratio = token_match_ratio(&path_blob, query_tokens);
    if token_ratio > 0.0 {
        score = score.max(0.50 + token_ratio * 0.45);
    }
    score.clamp(0.0, 1.0)
}
