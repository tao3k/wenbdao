//! Fixture-backed contracts for skill reference semantic classification.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;

use xiuxian_wendao::{EntityType, RelationType, classify_skill_reference};

use fixture_json_assertions::assert_json_fixture_eq;

#[test]
fn skill_reference_classification_matrix_contract() {
    let actual = serde_json::json!({
        "persona_hint": semantics_projection(&classify_skill_reference(Some("persona"), None, "steward.md")),
        "qianji_flow_hint": semantics_projection(&classify_skill_reference(Some("qianji-flow"), None, "flow.toml")),
        "attachment_inferred": semantics_projection(&classify_skill_reference(None, None, "logo.png")),
        "tool_config_type": semantics_projection(&classify_skill_reference(None, Some("tool"), "router.py")),
        "api_config_type": semantics_projection(&classify_skill_reference(None, Some("api"), "spec.json")),
        "template_by_extension": semantics_projection(&classify_skill_reference(None, None, "draft_agenda.j2")),
        "document_by_extension": semantics_projection(&classify_skill_reference(None, None, "rules.md")),
        "concept_fallback": semantics_projection(&classify_skill_reference(None, None, "agenda")),
    });

    assert_json_fixture_eq(
        "skill_semantics/reference_classification/expected",
        "result.json",
        &actual,
    );
}

fn semantics_projection(semantics: &xiuxian_wendao::SkillReferenceSemantics) -> serde_json::Value {
    serde_json::json!({
        "relation": relation_label(&semantics.relation),
        "entity": entity_label(&semantics.entity),
        "reference_type": semantics.reference_type,
    })
}

fn relation_label(value: &RelationType) -> String {
    value.to_string()
}

fn entity_label(value: &EntityType) -> String {
    value.to_string()
}
