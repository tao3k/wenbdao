//! Fixture-backed contracts for embedded dynamic semantic URI discovery.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;

use xiuxian_wendao::embedded_discover_canonical_uris;

use fixture_json_assertions::assert_json_fixture_eq;

#[test]
fn embedded_dynamic_discovery_queries_contract() -> Result<(), Box<dyn std::error::Error>> {
    let actual = serde_json::json!({
        "empty": embedded_discover_canonical_uris("")?,
        "query_prefix_only": embedded_discover_canonical_uris("query:")?,
        "reference_type_template": embedded_discover_canonical_uris("reference_type:template")?,
        "type_alias_template": embedded_discover_canonical_uris("type:template")?,
        "config_id_agenda_flow": embedded_discover_canonical_uris("id:agenda_flow")?,
        "config_id_soul_forge_flow": embedded_discover_canonical_uris("id:soul_forge_flow")?,
        "semantic_carryover": embedded_discover_canonical_uris("carryover:>=1")?,
        "query_prefixed_semantic_carryover": embedded_discover_canonical_uris("query: carryover:>=1")?,
        "unknown_config_id": embedded_discover_canonical_uris("id:missing")?,
    });

    assert_json_fixture_eq(
        "wendao_registry/dynamic_discovery/expected",
        "result.json",
        &actual,
    );
    Ok(())
}
