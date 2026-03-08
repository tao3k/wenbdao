//! Topology regression tests for mixed outbound link structures.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;
#[path = "support/link_graph_fixture_tree.rs"]
mod link_graph_fixture_tree;

use std::collections::HashMap;

use fixture_json_assertions::assert_json_fixture_eq;
use link_graph_fixture_tree::materialize_link_graph_fixture;
use serde_json::json;
use xiuxian_wendao::LinkGraphIndex;

#[test]
fn test_mixed_graph_topology_related_from_weighted_seed() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = materialize_link_graph_fixture(
        "link_graph/mixed_topology/weighted_seed_exposes_linked_entities/input",
    )?;
    let index = LinkGraphIndex::build(fixture.path())?;

    let mut seeds = HashMap::new();
    seeds.insert("note".to_string(), 1.0);
    let (related, _) = index.related_from_weighted_seeds_with_diagnostics(&seeds, 1, 10, None);

    let stems: Vec<String> = related.iter().map(|node| node.stem.clone()).collect();
    let actual = json!({
        "rows": related
            .iter()
            .map(|node| json!({
                "stem": node.stem,
                "path": node.path,
                "distance": node.distance,
            }))
            .collect::<Vec<_>>(),
        "contains": {
            "EntityA": stems.iter().any(|stem| stem == "EntityA"),
            "EntityB": stems.iter().any(|stem| stem == "EntityB"),
        },
    });

    assert_json_fixture_eq(
        "link_graph/mixed_topology/weighted_seed_exposes_linked_entities/expected",
        "result.json",
        &actual,
    );
    Ok(())
}
