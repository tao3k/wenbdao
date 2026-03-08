use super::*;

use fixture_contract_support::{
    SearchBasicFixture, assert_search_basic_fixture, search_payload_snapshot,
};

#[test]
fn test_wendao_search_strategy_and_path_sort_flags() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchBasicFixture::build("strategy_and_path_sort")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("search")
        .arg(".md")
        .arg("--limit")
        .arg("5")
        .arg("--match-strategy")
        .arg("fts")
        .arg("--sort-term")
        .arg("path_asc")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search with strategy/path sort failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let payload: Value = serde_json::from_str(&stdout)?;
    let actual = search_payload_snapshot(&payload);
    assert_search_basic_fixture("strategy_and_path_sort", "result.json", &actual);

    Ok(())
}
