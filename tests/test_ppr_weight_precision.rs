//! Precision regression for weighted-seed PPR ranking.

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

/// Precision test for Non-uniform Seed Distribution (Ref: `HippoRAG` 2).
///
/// Validates that higher semantic weights on seeds correctly influence
/// the structural diffusion results compared to uniform distribution.
#[test]
fn test_ppr_weight_precision_impact() -> Result<(), Box<dyn std::error::Error>> {
    let fixture =
        materialize_link_graph_fixture("link_graph/ppr_weighting/non_uniform_seed_bias/input")?;
    let index = LinkGraphIndex::build(fixture.path())?;

    let mut seeds_weighted = HashMap::new();
    seeds_weighted.insert("A".to_string(), 0.99);
    seeds_weighted.insert("C".to_string(), 0.01);

    let (related_weighted, _) =
        index.related_from_weighted_seeds_with_diagnostics(&seeds_weighted, 2, 10, None);

    let stems: Vec<String> = related_weighted
        .iter()
        .map(|node| node.stem.clone())
        .collect();
    let pos_b = stems.iter().position(|stem| stem == "B");
    let pos_d = stems.iter().position(|stem| stem == "D");

    let actual = json!({
        "seed_weights": {
            "A": 0.99,
            "C": 0.01,
        },
        "ranked_stems": stems,
        "positions": {
            "B": pos_b,
            "D": pos_d,
        },
        "b_before_d": matches!((pos_b, pos_d), (Some(b), Some(d)) if b < d),
    });

    assert_json_fixture_eq(
        "link_graph/ppr_precision/weighted_seed_precision_impact/expected",
        "result.json",
        &actual,
    );
    Ok(())
}
