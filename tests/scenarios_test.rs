//! Unified scenario tests for xiuxian-wendao.
//!
//! Single entry point for all scenario-based tests using ScenarioFramework.
//! Scenarios are defined in `tests/scenarios/` with insta-managed snapshots.

use std::path::PathBuf;

use xiuxian_testing::ScenarioFramework;

mod support;
use support::{GraphRunner, PageIndexRunner, SearchRunner};

/// Get the manifest directory for this crate.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn run_all_scenarios() {
    let manifest = manifest_dir();
    let scenarios_root = manifest.join("tests").join("scenarios");
    let snapshot_path = manifest.join("tests").join("snapshots");

    let mut framework = ScenarioFramework::with_snapshot_path(&snapshot_path);

    // Register all runners (unit structs don't need new())
    framework.register(Box::new(PageIndexRunner));
    framework.register(Box::new(SearchRunner));
    framework.register(Box::new(GraphRunner));

    // Run all scenarios with registered runners at this crate's scenarios root
    let count = framework.run_all_at(&scenarios_root).expect("scenario tests should pass");
    assert!(count > 0, "should run at least one scenario");
}
