use super::*;

use fixture_contract_support::{
    SearchDirectivesFixture, assert_search_directives_fixture, legacy_sort_error_snapshot,
};

#[test]
fn test_wendao_search_rejects_legacy_sort_flag() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchDirectivesFixture::build("rejects_legacy_sort_flag")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("search")
        .arg("a")
        .arg("--sort")
        .arg("path_asc")
        .output()?;

    let actual = legacy_sort_error_snapshot(&output);
    assert_search_directives_fixture("rejects_legacy_sort_flag", "result.json", &actual);
    Ok(())
}
