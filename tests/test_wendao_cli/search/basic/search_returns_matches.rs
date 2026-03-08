use super::*;

use fixture_contract_support::{
    SearchBasicFixture, assert_search_basic_fixture, search_payload_snapshot,
};

#[test]
fn test_wendao_search_returns_matches() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchBasicFixture::build("returns_matches")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("search")
        .arg("beta")
        .arg("--limit")
        .arg("5")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let payload: Value = serde_json::from_str(&stdout)?;
    let actual = search_payload_snapshot(&payload);
    assert_search_basic_fixture("returns_matches", "result.json", &actual);

    Ok(())
}
