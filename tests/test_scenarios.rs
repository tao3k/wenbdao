//! Scenario-based snapshot tests for xiuxian-wendao.
//!
//! Uses the reusable ScenarioFramework with Insta for snapshot testing.
//!
//! # Scenario Structure
//!
//! ```text
//! tests/fixtures/scenarios/001_page_index_hierarchy/
//! ├── input/
//! │   └── docs/
//! │       └── alpha.md
//! ├── expected/
//! │   └── tree.json
//! └── scenario.toml
//! ```

mod support;

use support::{GraphRunner, PageIndexRunner, SearchRunner};
use xiuxian_testing::ScenarioFramework;

// ============================================================================
// Test: Page Index Scenarios
// ============================================================================

#[test]
fn test_page_index_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(PageIndexRunner));
    framework.run_category("page_index").unwrap();
}

// ============================================================================
// Test: Search Scenarios
// ============================================================================

#[test]
fn test_search_core_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(SearchRunner));
    framework.run_category("search_core").unwrap();
}

#[test]
fn test_search_filters_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(SearchRunner));
    framework.run_category("search_filters").unwrap();
}

#[test]
fn test_search_match_strategies_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(SearchRunner));
    framework.run_category("search_match_strategies").unwrap();
}

#[test]
fn test_tree_scope_filters_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(SearchRunner));
    framework.run_category("tree_scope_filters").unwrap();
}

// ============================================================================
// Test: Graph Navigation Scenarios
// ============================================================================

#[test]
fn test_graph_navigation_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("graph_navigation").unwrap();
}

#[test]
fn test_mixed_topology_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("mixed_topology").unwrap();
}

// ============================================================================
// Test: Build Scope Scenarios
// ============================================================================

#[test]
fn test_build_scope_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("build_scope").unwrap();
}

// ============================================================================
// Test: Cache Build Scenarios
// ============================================================================

#[test]
fn test_cache_build_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("cache_build").unwrap();
}

// ============================================================================
// Test: Markdown Attachments Scenarios
// ============================================================================

#[test]
fn test_markdown_attachments_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("markdown_attachments").unwrap();
}

// ============================================================================
// Test: Refresh Scenarios
// ============================================================================

#[test]
fn test_refresh_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("refresh").unwrap();
}

// ============================================================================
// Test: Seed and Priors Scenarios
// ============================================================================

#[test]
fn test_seed_and_priors_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("seed_and_priors").unwrap();
}

// ============================================================================
// Test: Semantic Policy Scenarios
// ============================================================================

#[test]
fn test_semantic_policy_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("semantic_policy").unwrap();
}

// ============================================================================
// Test: PPR Scenarios
// ============================================================================

#[test]
fn test_ppr_precision_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("ppr_precision").unwrap();
}

#[test]
fn test_ppr_weighting_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("ppr_weighting").unwrap();
}

// ============================================================================
// Test: Agentic Expansion Scenarios
// ============================================================================

#[test]
fn test_agentic_expansion_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("agentic_expansion").unwrap();
}

// ============================================================================
// Test: Quantum Fusion Scenarios
// ============================================================================

#[test]
fn test_quantum_fusion_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("quantum_fusion").unwrap();
}

#[test]
fn test_hybrid_quantum_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(GraphRunner));
    framework.run_category("hybrid").unwrap();
}
