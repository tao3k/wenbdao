use crate::helpers::emit;
use crate::types::Cli;
use anyhow::Result;
use xiuxian_wendao::{
    LinkGraphSuggestedLinkDecisionRequest, LinkGraphSuggestedLinkRequest,
    LinkGraphSuggestedLinkState, valkey_suggested_link_decide,
    valkey_suggested_link_decisions_recent, valkey_suggested_link_log,
    valkey_suggested_link_recent, valkey_suggested_link_recent_latest,
};

pub(super) fn handle_log(
    cli: &Cli,
    source_id: &str,
    target_id: &str,
    relation: &str,
    confidence: f64,
    evidence: &str,
    agent_id: &str,
    created_at_unix: Option<f64>,
) -> Result<()> {
    let row = valkey_suggested_link_log(LinkGraphSuggestedLinkRequest {
        source_id: source_id.to_string(),
        target_id: target_id.to_string(),
        relation: relation.to_string(),
        confidence,
        evidence: evidence.to_string(),
        agent_id: agent_id.to_string(),
        created_at_unix,
    })
    .map_err(anyhow::Error::msg)?;
    emit(&row, cli.output)
}

pub(super) fn handle_recent(
    cli: &Cli,
    limit: usize,
    latest: bool,
    state: Option<LinkGraphSuggestedLinkState>,
) -> Result<()> {
    let state_filter = state.map(Into::into);
    let rows = if latest {
        valkey_suggested_link_recent_latest(limit, state_filter)
    } else {
        valkey_suggested_link_recent(limit)
    }
    .map_err(anyhow::Error::msg)?;
    let filtered = if latest || state_filter.is_none() {
        rows
    } else {
        rows.into_iter()
            .filter(|row| Some(row.promotion_state) == state_filter)
            .collect()
    };
    emit(&filtered, cli.output)
}

pub(super) fn handle_decide(
    cli: &Cli,
    suggestion_id: &str,
    target_state: LinkGraphSuggestedLinkState,
    decided_by: &str,
    reason: &str,
    decided_at_unix: Option<f64>,
) -> Result<()> {
    let result = valkey_suggested_link_decide(LinkGraphSuggestedLinkDecisionRequest {
        suggestion_id: suggestion_id.to_string(),
        target_state,
        decided_by: decided_by.to_string(),
        reason: reason.to_string(),
        decided_at_unix,
    })
    .map_err(anyhow::Error::msg)?;
    emit(&result, cli.output)
}

pub(super) fn handle_decisions(cli: &Cli, limit: usize) -> Result<()> {
    let rows = valkey_suggested_link_decisions_recent(limit).map_err(anyhow::Error::msg)?;
    emit(&rows, cli.output)
}
