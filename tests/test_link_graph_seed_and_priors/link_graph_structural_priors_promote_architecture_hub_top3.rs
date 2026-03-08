use xiuxian_wendao::link_graph::{
    LinkGraphEdgeType, LinkGraphSearchFilters, LinkGraphSearchOptions,
};

use super::fixture_contract_support::{
    SeedAndPriorsFixture, assert_seed_and_priors_fixture, structural_prior_snapshot,
};

#[test]
fn test_link_graph_structural_priors_promote_architecture_hub_top3()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SeedAndPriorsFixture::build("structural_priors_architecture_hub")?;
    let index = fixture.build_index()?;
    let boosted_hits = index
        .search_planned(
            "Architecture decision ledger",
            5,
            LinkGraphSearchOptions::default(),
        )
        .1;

    let no_semantic_edge_options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            edge_types: vec![LinkGraphEdgeType::Structural],
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let baseline_hits = index
        .search_planned("Architecture decision ledger", 5, no_semantic_edge_options)
        .1;

    let actual = structural_prior_snapshot(&boosted_hits, &baseline_hits)?;
    assert_seed_and_priors_fixture("structural_priors_architecture_hub", &actual);
    Ok(())
}
