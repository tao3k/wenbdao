use serde_json::Value;

pub(crate) fn parse_success_json(
    output: std::process::Output,
    context: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{context}: {stderr}");
    let stdout = String::from_utf8(output.stdout)?;
    Ok(serde_json::from_str(&stdout)?)
}
