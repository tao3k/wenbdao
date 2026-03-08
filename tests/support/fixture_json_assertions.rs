//! Shared JSON fixture assertions for `xiuxian-wendao` integration tests.

use serde_json::Value;

use super::fixture_read::read_fixture;

fn render_json(value: &Value) -> String {
    format!(
        "{}\n",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    )
}

pub(crate) fn assert_json_fixture_eq(fixture_root: &str, relative: &str, actual: &Value) {
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
