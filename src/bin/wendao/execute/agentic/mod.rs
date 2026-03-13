//! Agentic command execution.

use crate::helpers::emit;
use crate::types::{AgenticCommand, Cli, Command};
use anyhow::{Context, Result};
use xiuxian_wendao::link_graph::LinkGraphAgenticExecutionConfig;
use xiuxian_wendao::{
    LinkGraphIndex, LinkGraphSuggestedLinkDecisionRequest, LinkGraphSuggestedLinkRequest,
    valkey_suggested_link_decide, valkey_suggested_link_decisions_recent,
    valkey_suggested_link_log, valkey_suggested_link_recent, valkey_suggested_link_recent_latest,
};

pub(super) fn handle(cli: &Cli, index: Option<&LinkGraphIndex>) -> Result<()> {
    let Command::Agentic { command } = &cli.command else {
        unreachable!("agentic handler must be called with agentic command");
    };

    match command {
        AgenticCommand::Log {
            source_id,
            target_id,
            relation,
            confidence,
            evidence,
            agent_id,
            created_at_unix,
        } => {
            let row = valkey_suggested_link_log(LinkGraphSuggestedLinkRequest {
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                relation: relation.clone(),
                confidence: *confidence,
                evidence: evidence.clone(),
                agent_id: agent_id.clone(),
                created_at_unix: *created_at_unix,
            })
            .map_err(anyhow::Error::msg)?;
            emit(&row, cli.output)
        }
        AgenticCommand::Recent {
            limit,
            latest,
            state,
        } => {
            let state_filter = state.map(Into::into);
            let rows = if *latest {
                valkey_suggested_link_recent_latest((*limit).max(1), state_filter)
            } else {
                valkey_suggested_link_recent((*limit).max(1))
            }
            .map_err(anyhow::Error::msg)?;
            let filtered = if *latest || state_filter.is_none() {
                rows
            } else {
                rows.into_iter()
                    .filter(|row| Some(row.promotion_state) == state_filter)
                    .collect()
            };
            emit(&filtered, cli.output)
        }
        AgenticCommand::Decide {
            suggestion_id,
            target_state,
            decided_by,
            reason,
            decided_at_unix,
        } => {
            let result = valkey_suggested_link_decide(LinkGraphSuggestedLinkDecisionRequest {
                suggestion_id: suggestion_id.clone(),
                target_state: (*target_state).into(),
                decided_by: decided_by.clone(),
                reason: reason.clone(),
                decided_at_unix: *decided_at_unix,
            })
            .map_err(anyhow::Error::msg)?;
            emit(&result, cli.output)
        }
        AgenticCommand::Decisions { limit } => {
            let rows = valkey_suggested_link_decisions_recent((*limit).max(1))
                .map_err(anyhow::Error::msg)?;
            emit(&rows, cli.output)
        }
        AgenticCommand::Plan {
            query,
            max_workers,
            max_candidates,
            max_pairs_per_worker,
            time_budget_ms,
        } => {
            let index = index.context("link_graph index is required for agentic plan command")?;
            let mut config = index.resolve_agentic_expansion_config();
            if let Some(value) = max_workers {
                config.max_workers = (*value).max(1);
            }
            if let Some(value) = max_candidates {
                config.max_candidates = (*value).max(1);
            }
            if let Some(value) = max_pairs_per_worker {
                config.max_pairs_per_worker = (*value).max(1);
            }
            if let Some(value) = time_budget_ms {
                config.time_budget_ms = if value.is_finite() && *value > 0.0 {
                    *value
                } else {
                    config.time_budget_ms
                };
            }
            let plan = index.agentic_expansion_plan_with_config(query.as_deref(), config);
            emit(&plan, cli.output)
        }
        AgenticCommand::Run {
            query,
            max_workers,
            max_candidates,
            max_pairs_per_worker,
            time_budget_ms,
            worker_time_budget_ms,
            persist,
            persist_retry_attempts,
            idempotency_scan_limit,
            relation,
            agent_id,
            evidence_prefix,
            created_at_unix,
            verbose,
        } => {
            let index = index.context("link_graph index is required for agentic run command")?;
            let mut config: LinkGraphAgenticExecutionConfig =
                index.resolve_agentic_execution_config();
            if let Some(value) = max_workers {
                config.expansion.max_workers = (*value).max(1);
            }
            if let Some(value) = max_candidates {
                config.expansion.max_candidates = (*value).max(1);
            }
            if let Some(value) = max_pairs_per_worker {
                config.expansion.max_pairs_per_worker = (*value).max(1);
            }
            if let Some(value) = time_budget_ms {
                config.expansion.time_budget_ms = if value.is_finite() && *value > 0.0 {
                    *value
                } else {
                    config.expansion.time_budget_ms
                };
            }
            if let Some(value) = worker_time_budget_ms {
                config.worker_time_budget_ms = if value.is_finite() && *value > 0.0 {
                    *value
                } else {
                    config.worker_time_budget_ms
                };
            }
            if let Some(value) = persist {
                config.persist_suggestions = *value;
            }
            if let Some(value) = persist_retry_attempts {
                config.persist_retry_attempts = (*value).max(1);
            }
            if let Some(value) = idempotency_scan_limit {
                config.idempotency_scan_limit = (*value).max(1);
            }
            if let Some(value) = relation {
                config.relation = value.clone();
            }
            if let Some(value) = agent_id {
                config.agent_id = value.clone();
            }
            if let Some(value) = evidence_prefix {
                config.evidence_prefix = value.clone();
            }
            config.created_at_unix = *created_at_unix;
            let result = index.agentic_expansion_execute_with_config(query.as_deref(), config);
            if *verbose {
                // Handle verbose output if needed
            }
            emit(&result, cli.output)
        }
    }
}
