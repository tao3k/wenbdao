use crate::test_wendao_cli::support::wendao_cmd;
use serde_json::Value;

use super::fixture_contract_support::{
    SearchBasicFixture, assert_search_basic_fixture, search_payload_snapshot,
};

#[test]
fn test_wendao_search_path_fuzzy_emits_section_context() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchBasicFixture::build("path_fuzzy_section_context")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("search")
        .arg("architecture graph engine")
        .arg("--limit")
        .arg("5")
        .arg("--match-strategy")
        .arg("path_fuzzy")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search with path_fuzzy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let payload: Value = serde_json::from_str(&stdout)?;
    let actual = search_payload_snapshot(&payload);
    assert_search_basic_fixture("path_fuzzy_section_context", "result.json", &actual);

    Ok(())
}
