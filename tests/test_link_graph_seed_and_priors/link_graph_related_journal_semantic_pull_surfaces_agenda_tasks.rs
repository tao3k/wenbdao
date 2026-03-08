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
fn test_link_graph_related_journal_semantic_pull_surfaces_agenda_tasks()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SeedAndPriorsFixture::build("journal_semantic_pull_surfaces_agenda")?;
    let index = fixture.build_index()?;
    let options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            related: Some(LinkGraphRelatedFilter {
                seeds: vec!["docs/journal/journal-entry-2026-02-26.md".to_string()],
                max_distance: Some(3),
                ppr: Some(LinkGraphRelatedPprOptions {
                    alpha: Some(0.9),
                    max_iter: Some(64),
                    tol: Some(1e-6),
                    subgraph_mode: Some(LinkGraphPprSubgraphMode::Force),
                }),
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };

    let hits = index
        .search_planned("checkpoint token carryover", 16, options)
        .1;
    let agenda_hit = hits
        .iter()
        .find(|row| row.path == "docs/agenda/agenda-tasks-2026-02-26.md")
        .ok_or_else(|| {
            std::io::Error::other("expected seeded related search to surface linked agenda doc")
        })?;
    let stems: HashSet<String> = hits.iter().map(|row| row.stem.clone()).collect();

    let actual = json!({
        "hits": hits_snapshot(&hits),
        "agenda_hit": {
            "path": agenda_hit.path,
            "best_section": agenda_hit.best_section,
            "match_reason": agenda_hit.match_reason,
            "best_section_contains_tasks": agenda_hit
                .best_section
                .as_deref()
                .is_some_and(|section| section.contains("Tasks")),
        },
        "contains_unrelated_agenda": stems.contains("2026-02-27"),
    });

    assert_seed_and_priors_fixture("journal_semantic_pull_surfaces_agenda", &actual);
    Ok(())
}
