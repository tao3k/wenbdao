//! Weighted-seed PPR behavior checks.

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
fn test_ppr_non_uniform_seed_bias() -> Result<(), Box<dyn std::error::Error>> {
    let fixture =
        materialize_link_graph_fixture("link_graph/ppr_weighting/non_uniform_seed_bias/input")?;
    let index = LinkGraphIndex::build(fixture.path())?;

    let mut seeds = HashMap::new();
    seeds.insert("A".to_string(), 0.9);
    seeds.insert("C".to_string(), 0.1);

    let (related, _) = index.related_from_weighted_seeds_with_diagnostics(&seeds, 2, 10, None);
    let stems: Vec<String> = related.iter().map(|node| node.stem.clone()).collect();

    let pos_b = stems.iter().position(|stem| stem == "B");
    let pos_d = stems.iter().position(|stem| stem == "D");
    let actual = json!({
        "rows": related
            .iter()
            .map(|node| json!({
                "stem": node.stem,
                "path": node.path,
                "distance": node.distance,
            }))
            .collect::<Vec<_>>(),
        "positions": {
            "B": pos_b,
            "D": pos_d,
        },
        "b_before_d": matches!((pos_b, pos_d), (Some(b), Some(d)) if b < d),
    });

    assert_json_fixture_eq(
        "link_graph/ppr_weighting/non_uniform_seed_bias/expected",
        "result.json",
        &actual,
    );
    Ok(())
}
