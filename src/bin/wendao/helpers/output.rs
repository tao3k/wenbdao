use super::super::types::OutputFormat;
use anyhow::{Context, Result};
use serde::Serialize;

pub(crate) fn emit<T: Serialize>(value: &T, output: OutputFormat) -> Result<()> {
    let rendered = match output {
        OutputFormat::Json => serde_json::to_string(value),
        OutputFormat::Pretty => serde_json::to_string_pretty(value),
    }
    .context("failed to serialize CLI output as JSON")?;
    println!("{rendered}");
    Ok(())
}
