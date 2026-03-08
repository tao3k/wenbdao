use super::*;

use fixture_contract_support::{
    RelatedCliFixture, assert_related_cli_fixture, related_rows_snapshot,
};

#[test]
fn test_wendao_related_command_accepts_ppr_flags() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = RelatedCliFixture::build("linear_chain")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("related")
        .arg("b")
        .arg("--max-distance")
        .arg("2")
        .arg("--limit")
        .arg("10")
        .arg("--ppr-alpha")
        .arg("0.9")
        .arg("--ppr-max-iter")
        .arg("64")
        .arg("--ppr-tol")
        .arg("1e-6")
        .arg("--ppr-subgraph-mode")
        .arg("force")
        .output()?;

    assert!(
        output.status.success(),
        "wendao related with ppr flags failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
    let actual = related_rows_snapshot(&payload)?;
    assert_related_cli_fixture("linear_chain", "related_with_ppr_flags.json", &actual);

    Ok(())
}
