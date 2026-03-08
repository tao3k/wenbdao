//! Fixture-backed contracts for bounded agentic expansion planning and execution.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;
#[path = "support/link_graph_agentic_expansion_fixture_support.rs"]
mod link_graph_agentic_expansion_fixture_support;
#[path = "support/link_graph_fixture_tree.rs"]
mod link_graph_fixture_tree;

use link_graph_agentic_expansion_fixture_support::{
    AgenticExpansionFixture, assert_agentic_expansion_fixture, execution_snapshot, plan_snapshot,
};
use xiuxian_wendao::{
    LinkGraphAgenticExecutionConfig, LinkGraphAgenticExpansionConfig, LinkGraphIndex,
};

#[test]
fn test_agentic_expansion_plan_respects_worker_and_pair_budgets()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AgenticExpansionFixture::build("worker_and_pair_budgets")?;
    let index = LinkGraphIndex::build(fixture.root()).map_err(|error| error.clone())?;
    let plan = index.agentic_expansion_plan_with_config(
        None,
        LinkGraphAgenticExpansionConfig {
            max_workers: 2,
            max_candidates: 4,
            max_pairs_per_worker: 2,
            time_budget_ms: 1_000.0,
        },
    );

    let actual = plan_snapshot(&plan);
    assert_agentic_expansion_fixture("worker_and_pair_budgets", &actual);
    Ok(())
}

#[test]
fn test_agentic_expansion_plan_query_narrows_candidates() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = AgenticExpansionFixture::build("query_narrows_candidates")?;
    let index = LinkGraphIndex::build(fixture.root()).map_err(|error| error.clone())?;
    let plan = index.agentic_expansion_plan_with_config(
        Some("alpha"),
        LinkGraphAgenticExpansionConfig {
            max_workers: 3,
            max_candidates: 10,
            max_pairs_per_worker: 3,
            time_budget_ms: 1_000.0,
        },
    );

    let actual = plan_snapshot(&plan);
    assert_agentic_expansion_fixture("query_narrows_candidates", &actual);
    Ok(())
}

#[test]
fn test_agentic_expansion_execute_emits_worker_telemetry_without_persistence()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AgenticExpansionFixture::build("execution_without_persistence")?;
    let index = LinkGraphIndex::build(fixture.root()).map_err(|error| error.clone())?;
    let result = index.agentic_expansion_execute_with_config(
        Some("alpha"),
        LinkGraphAgenticExecutionConfig {
            expansion: LinkGraphAgenticExpansionConfig {
                max_workers: 1,
                max_candidates: 4,
                max_pairs_per_worker: 1,
                time_budget_ms: 1_000.0,
            },
            worker_time_budget_ms: 1_000.0,
            persist_suggestions: false,
            persist_retry_attempts: 2,
            idempotency_scan_limit: 128,
            relation: "related_to".to_string(),
            agent_id: "test-worker".to_string(),
            evidence_prefix: "execution test".to_string(),
            created_at_unix: Some(1_700_001_234.0),
        },
    );

    let actual = execution_snapshot(&result);
    assert_agentic_expansion_fixture("execution_without_persistence", &actual);
    Ok(())
}
