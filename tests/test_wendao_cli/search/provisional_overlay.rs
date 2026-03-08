use super::*;

#[path = "provisional_overlay_fixture_contract_support.rs"]
mod provisional_overlay_fixture_contract_support;

use provisional_overlay_fixture_contract_support::{
    SearchProvisionalFixture, assert_search_provisional_fixture, payload_snapshot, write_config,
};

#[test]
fn test_wendao_search_can_include_provisional_suggestions() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = SearchProvisionalFixture::build("include_provisional_cli_flag")?;

    let prefix = unique_agentic_prefix();
    if clear_valkey_prefix(&prefix).is_err() {
        return Ok(());
    }
    let config_path = fixture.config_path();
    write_config(config_path.as_path(), &prefix, false)?;

    let log_output = wendao_cmd()
        .arg("--conf")
        .arg(&config_path)
        .arg("agentic")
        .arg("log")
        .arg("docs/a.md")
        .arg("docs/b.md")
        .arg("related_to")
        .arg("--confidence")
        .arg("0.9")
        .arg("--evidence")
        .arg("alpha bridge")
        .arg("--agent-id")
        .arg("qianhuan-architect")
        .output()?;
    assert!(
        log_output.status.success(),
        "wendao agentic log failed: {}",
        String::from_utf8_lossy(&log_output.stderr)
    );

    let search_output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("--conf")
        .arg(&config_path)
        .arg("search")
        .arg("alpha")
        .arg("--limit")
        .arg("5")
        .arg("--include-provisional")
        .arg("--provisional-limit")
        .arg("10")
        .output()?;
    assert!(
        search_output.status.success(),
        "wendao search with provisional failed: {}",
        String::from_utf8_lossy(&search_output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(search_output.stdout)?)?;
    let actual = payload_snapshot(&payload);
    assert_search_provisional_fixture("include_provisional_cli_flag", &actual);

    clear_valkey_prefix(&prefix)?;
    Ok(())
}

#[test]
fn test_wendao_search_uses_engine_default_for_provisional_injection()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchProvisionalFixture::build("include_provisional_engine_default")?;

    let prefix = unique_agentic_prefix();
    if clear_valkey_prefix(&prefix).is_err() {
        return Ok(());
    }
    let config_path = fixture.config_path();
    write_config(config_path.as_path(), &prefix, true)?;

    let log_output = wendao_cmd()
        .arg("--conf")
        .arg(&config_path)
        .arg("agentic")
        .arg("log")
        .arg("docs/a.md")
        .arg("docs/b.md")
        .arg("related_to")
        .arg("--evidence")
        .arg("bridge")
        .arg("--agent-id")
        .arg("qianhuan-architect")
        .output()?;
    assert!(
        log_output.status.success(),
        "wendao agentic log failed: {}",
        String::from_utf8_lossy(&log_output.stderr)
    );

    let search_output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("--conf")
        .arg(&config_path)
        .arg("search")
        .arg("alpha")
        .arg("--limit")
        .arg("5")
        .output()?;
    assert!(
        search_output.status.success(),
        "wendao search with engine default provisional failed: {}",
        String::from_utf8_lossy(&search_output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(search_output.stdout)?)?;
    let actual = payload_snapshot(&payload);
    assert_search_provisional_fixture("include_provisional_engine_default", &actual);

    clear_valkey_prefix(&prefix)?;
    Ok(())
}
