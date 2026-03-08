//! Metadata lookup and alias resolution command handlers.

use crate::helpers::emit;
use crate::types::Cli;
use anyhow::{Context, Result};
use serde_json::json;
use xiuxian_wendao::{LinkGraphIndex, LinkGraphMetadata};

pub(super) fn handle_metadata(cli: &Cli, index: Option<&LinkGraphIndex>, stem: &str) -> Result<()> {
    let index = index.context("link_graph index is required for metadata command")?;
    let candidates = index.resolve_metadata_candidates(stem);
    match candidates.len() {
        0 => emit(&Option::<LinkGraphMetadata>::None, cli.output),
        1 => {
            let one = candidates.into_iter().next();
            emit(&one, cli.output)
        }
        _ => {
            let payload = json!({
                "error": "ambiguous_stem",
                "query": stem,
                "count": candidates.len(),
                "message": "multiple documents matched this stem/id/path; use full id or path",
                "candidates": candidates,
            });
            emit(&payload, cli.output)
        }
    }
}

pub(super) fn handle_resolve(
    cli: &Cli,
    index: Option<&LinkGraphIndex>,
    alias: &str,
    limit: usize,
) -> Result<()> {
    let index = index.context("link_graph index is required for resolve command")?;
    let mut candidates = index.resolve_metadata_candidates(alias);
    let total_count = candidates.len();
    candidates.truncate(limit.max(1));
    let payload = json!({
        "query": alias,
        "count": total_count,
        "returned_count": candidates.len(),
        "candidates": candidates,
    });
    emit(&payload, cli.output)
}
