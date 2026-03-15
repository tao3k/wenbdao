use crate::test_wendao_cli::cli_commands::support::parse_success_json;
use crate::test_wendao_cli::support::{wendao_cmd, write_file};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn test_wendao_page_index_emits_hierarchical_roots() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(
        &tmp.path().join("docs/alpha.md"),
        concat!(
            "# Alpha\n",
            "alpha root section carries enough words to remain stable without thinning across the CLI page index output.\n\n",
            "## Beta\n",
            "beta child section also carries enough words to preserve its nested heading tree and keep gamma visible beneath it.\n\n",
            "### Gamma\n",
            "Gamma body.\n\n",
            "## Delta\n",
            "Delta body.\n"
        ),
    )?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("page-index")
        .arg("alpha")
        .output()?;
    let payload = parse_success_json(output, "wendao page-index failed")?;

    assert_eq!(payload.get("query").and_then(Value::as_str), Some("alpha"));
    assert_eq!(payload.get("root_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        payload
            .get("resolved")
            .and_then(|value| value.get("path"))
            .and_then(Value::as_str),
        Some("docs/alpha.md")
    );

    let roots = payload
        .get("roots")
        .and_then(Value::as_array)
        .ok_or("missing roots array")?;
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].get("title").and_then(Value::as_str), Some("Alpha"));
    let children = roots[0]
        .get("children")
        .and_then(Value::as_array)
        .ok_or("missing child roots")?;
    assert_eq!(children.len(), 2);
    assert_eq!(
        children[0].get("title").and_then(Value::as_str),
        Some("Beta")
    );
    assert_eq!(
        children[1].get("title").and_then(Value::as_str),
        Some("Delta")
    );

    let gamma_children = children[0]
        .get("children")
        .and_then(Value::as_array)
        .ok_or("missing nested gamma children")?;
    assert_eq!(gamma_children.len(), 1);
    assert_eq!(
        gamma_children[0].get("title").and_then(Value::as_str),
        Some("Gamma")
    );

    Ok(())
}

#[test]
fn test_wendao_page_index_reports_ambiguous_aliases() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# Alpha\nBody.\n")?;
    write_file(&tmp.path().join("notes/a.md"), "# Alternate Alpha\nBody.\n")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("page-index")
        .arg("a")
        .output()?;
    let payload = parse_success_json(output, "wendao page-index ambiguity failed")?;

    assert_eq!(
        payload.get("error").and_then(Value::as_str),
        Some("ambiguous_stem")
    );
    assert_eq!(payload.get("query").and_then(Value::as_str), Some("a"));
    assert_eq!(payload.get("count").and_then(Value::as_u64), Some(2));

    let candidates = payload
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or("missing ambiguity candidates")?;
    assert_eq!(candidates.len(), 2);

    Ok(())
}
