//! Shared JSON fixture assertions for `xiuxian-wendao` integration tests.
//!
//! This module provides backward-compatible assertions using Insta internally.

use serde_json::Value;

use crate::fixture_read::read_fixture;

fn render_json(value: &Value) -> String {
    format!(
        "{}\n",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    )
}

/// Asserts that actual JSON matches the expected fixture content.
///
/// # Panics
/// Panics if the actual JSON differs from the fixture.
pub fn assert_json_fixture_eq(fixture_root: &str, relative: &str, actual: &Value) {
    let expected = read_fixture(fixture_root, relative);
    let expected_json = serde_json::from_str::<Value>(&expected).unwrap_or_else(|error| {
        panic!("failed to parse expected fixture {fixture_root}/{relative} as JSON: {error}")
    });

    assert_eq!(
        expected_json,
        *actual,
        "fixture mismatch: {fixture_root}/{relative}\n--- expected ---\n{}--- actual ---\n{}",
        render_json(&expected_json),
        render_json(actual),
    );
}
