use super::support::fixture_path;

#[test]
fn org_contract_directory_is_reserved_for_future_parser() {
    let dir = fixture_path("parser/org");
    assert!(
        dir.exists(),
        "expected parser/org fixture directory to exist for upcoming org parser coverage"
    );
}

#[test]
#[ignore = "org parser contracts will be enabled when org parser lands"]
fn org_parser_contract_placeholder() {
    // Intentionally empty: this test reserves the contract slot for future org parser coverage.
}
