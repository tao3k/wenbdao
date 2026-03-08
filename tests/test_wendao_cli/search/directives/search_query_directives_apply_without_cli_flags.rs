use super::*;

use fixture_contract_support::{
    SearchDirectivesFixture, assert_search_directives_fixture, search_payload_snapshot,
};

#[test]
fn test_wendao_search_query_directives_apply_without_cli_flags()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchDirectivesFixture::build("query_directives_without_cli_flags")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("search")
        .arg("to:b sort:path_asc .md")
        .arg("--limit")
        .arg("10")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search with query directives failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
    let actual = search_payload_snapshot(&payload);
    assert_search_directives_fixture("query_directives_without_cli_flags", "result.json", &actual);
    Ok(())
}
