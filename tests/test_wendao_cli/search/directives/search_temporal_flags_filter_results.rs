use crate::test_wendao_cli::support::wendao_cmd;
use serde_json::Value;

use super::fixture_contract_support::{
    SearchDirectivesFixture, assert_search_directives_fixture, search_payload_snapshot,
};

#[test]
fn test_wendao_search_temporal_flags_filter_results() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchDirectivesFixture::build("temporal_flags_filter_results")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("search")
        .arg(".md")
        .arg("--limit")
        .arg("10")
        .arg("--sort-term")
        .arg("created_asc")
        .arg("--created-after")
        .arg("1704153600")
        .arg("--created-before")
        .arg("1704758400")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search with temporal flags failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
    let actual = search_payload_snapshot(&payload);
    assert_search_directives_fixture("temporal_flags_filter_results", "result.json", &actual);
    Ok(())
}
