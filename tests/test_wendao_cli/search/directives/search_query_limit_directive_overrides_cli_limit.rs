use crate::test_wendao_cli::support::wendao_cmd;
use serde_json::Value;

use super::fixture_contract_support::{
    SearchDirectivesFixture, assert_search_directives_fixture, search_payload_snapshot,
};

#[test]
fn test_wendao_search_query_limit_directive_overrides_cli_limit()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchDirectivesFixture::build("query_limit_directive_overrides_cli_limit")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("search")
        .arg("query:shared keyword limit:1 sort:path_asc")
        .arg("--limit")
        .arg("10")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search with query limit directive failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
    let actual = search_payload_snapshot(&payload);
    assert_search_directives_fixture(
        "query_limit_directive_overrides_cli_limit",
        "result.json",
        &actual,
    );
    Ok(())
}
