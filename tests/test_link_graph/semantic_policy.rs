use super::semantic_policy_fixture_support::{
    SemanticPolicyFixture, assert_semantic_policy_fixture, parsed_semantic_policy_snapshot,
    planned_payload_semantic_policy_snapshot,
};
use xiuxian_wendao::{
    LinkGraphSearchOptions, LinkGraphSemanticDocumentScope, LinkGraphSemanticSearchPolicy,
    parse_search_query,
};

#[test]
fn test_link_graph_parse_search_query_supports_semantic_policy_directives() {
    let parsed = parse_search_query(
        "summary_only:true min_vector_score:0.62 beta topic",
        LinkGraphSearchOptions::default(),
    );

    let actual = parsed_semantic_policy_snapshot(&parsed);
    assert_semantic_policy_fixture("parse_directives", "result.json", &actual);
}

#[test]
fn test_link_graph_search_planned_payload_records_semantic_policy()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SemanticPolicyFixture::build("planned_payload")?;
    let index = fixture.build_index()?;

    let payload = index.search_planned_payload(
        "beta topic",
        5,
        LinkGraphSearchOptions {
            semantic_policy: LinkGraphSemanticSearchPolicy {
                document_scope: LinkGraphSemanticDocumentScope::SummaryOnly,
                min_vector_score: Some(0.72),
            },
            ..LinkGraphSearchOptions::default()
        },
    );

    let actual = planned_payload_semantic_policy_snapshot(&payload);
    assert_semantic_policy_fixture("planned_payload", "result.json", &actual);
    Ok(())
}
