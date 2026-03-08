use super::super::super::state::ParsedDirectiveState;
use crate::link_graph::models::LinkGraphSemanticDocumentScope;
use crate::link_graph::query::helpers::parse_bool;

fn parse_score(value: &str) -> Option<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|score| score.is_finite() && (0.0..=1.0).contains(score))
}

pub(super) fn apply(key: &str, value: &str, state: &mut ParsedDirectiveState) -> bool {
    match key {
        "semantic_scope" | "semantic_document_scope" => {
            state.semantic_document_scope = LinkGraphSemanticDocumentScope::from_alias(value);
            true
        }
        "summary_only" | "semantic_summary_only" => {
            state.semantic_document_scope = Some(if parse_bool(value).unwrap_or(true) {
                LinkGraphSemanticDocumentScope::SummaryOnly
            } else {
                LinkGraphSemanticDocumentScope::All
            });
            true
        }
        "min_vector_score" | "semantic_min_vector_score" => {
            state.semantic_min_vector_score = parse_score(value);
            true
        }
        _ => false,
    }
}
