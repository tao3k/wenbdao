use super::*;

use fixture_contract_support::{
    SearchBasicFixture, assert_search_basic_fixture, search_verbose_snapshot,
};

#[test]
fn test_wendao_search_verbose_includes_monitor_summary() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchBasicFixture::build("verbose_monitor_summary")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("search")
        .arg("beta")
        .arg("--limit")
        .arg("5")
        .arg("--verbose")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search --verbose failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
    let actual = search_verbose_snapshot(&payload)?;
    assert_search_basic_fixture("verbose_monitor_summary", "result.json", &actual);
    Ok(())
}
