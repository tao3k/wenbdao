use crate::test_wendao_cli::cli_commands::support::parse_success_json;
use crate::test_wendao_cli::support::{wendao_cmd, write_file};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn test_wendao_stats_reports_note_counts() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\n[[b]]\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\n[[a]]\n")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("stats")
        .output()?;
    let payload = parse_success_json(output, "wendao stats failed")?;
    assert_eq!(payload.get("total_notes").and_then(Value::as_u64), Some(2));
    assert_eq!(
        payload.get("links_in_graph").and_then(Value::as_u64),
        Some(2)
    );
    Ok(())
}
