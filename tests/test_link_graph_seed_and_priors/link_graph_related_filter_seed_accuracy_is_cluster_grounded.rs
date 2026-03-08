use std::collections::HashSet;

use serde_json::json;
use xiuxian_wendao::link_graph::{
    LinkGraphPprSubgraphMode, LinkGraphRelatedFilter, LinkGraphRelatedPprOptions,
    LinkGraphSearchFilters, LinkGraphSearchOptions,
};

use super::fixture_contract_support::{
    SeedAndPriorsFixture, assert_seed_and_priors_fixture, hits_snapshot,
};

#[test]
fn test_link_graph_related_filter_seed_accuracy_is_cluster_grounded()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SeedAndPriorsFixture::build("seed_accuracy_cluster_grounded")?;
    let index = fixture.build_index()?;
    let ppr = LinkGraphRelatedPprOptions {
        alpha: Some(0.9),
        max_iter: Some(32),
        tol: Some(1e-6),
        subgraph_mode: Some(LinkGraphPprSubgraphMode::Force),
    };

    let arch_options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            related: Some(LinkGraphRelatedFilter {
                seeds: vec!["docs/arch-seed.md".to_string()],
                max_distance: Some(3),
                ppr: Some(ppr.clone()),
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let arch_hits = index.search_planned("platform note", 16, arch_options).1;
    let arch_stems: HashSet<String> = arch_hits.iter().map(|row| row.stem.clone()).collect();

    let db_options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            related: Some(LinkGraphRelatedFilter {
                seeds: vec!["db-seed".to_string()],
                max_distance: Some(3),
                ppr: Some(ppr),
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let db_hits = index.search_planned("platform note", 16, db_options).1;
    let db_stems: HashSet<String> = db_hits.iter().map(|row| row.stem.clone()).collect();

    let actual = json!({
        "arch": {
            "hits": hits_snapshot(&arch_hits),
            "contains_cluster": {
                "arch-a": arch_stems.contains("arch-a"),
                "arch-b": arch_stems.contains("arch-b"),
                "arch-c": arch_stems.contains("arch-c"),
            },
            "leaks_db_cluster": arch_stems.iter().any(|stem| stem.starts_with("db-")),
        },
        "db": {
            "hits": hits_snapshot(&db_hits),
            "contains_cluster": {
                "db-a": db_stems.contains("db-a"),
                "db-b": db_stems.contains("db-b"),
            },
            "leaks_arch_cluster": db_stems.iter().any(|stem| stem.starts_with("arch-")),
        },
    });

    assert_seed_and_priors_fixture("seed_accuracy_cluster_grounded", &actual);
    Ok(())
}
