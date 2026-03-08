use super::*;

#[test]
fn link_graph_parse_search_query_supports_semantic_policy_directives() {
    let parsed = parse_search_query(
        "semantic_scope:summary semantic_min_vector_score:0.55 beta topic",
        LinkGraphSearchOptions::default(),
    );

    assert_eq!(parsed.query, "beta topic");
    assert_eq!(
        parsed.options.semantic_policy.document_scope,
        LinkGraphSemanticDocumentScope::SummaryOnly
    );
    assert_eq!(parsed.options.semantic_policy.min_vector_score, Some(0.55));
}
