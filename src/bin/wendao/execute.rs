//! Command dispatch implementation for `wendao` CLI.
//!
//! This module dispatches CLI commands to their respective handler modules.
//!
//! Each handler module implements the logic for a specific command.

#[path = "execute/agentic/mod.rs"]
mod agentic;
#[path = "execute/attachments.rs"]
mod attachments;
#[path = "execute/audit.rs"]
mod audit;
#[path = "execute/fix.rs"]
mod fix;
#[cfg(feature = "zhenfa-router")]
#[path = "execute/gateway/mod.rs"]
mod gateway;
#[path = "execute/graph.rs"]
mod graph;
#[path = "execute/hmas.rs"]
mod hmas;
#[path = "execute/repo.rs"]
mod repo;
#[path = "execute/saliency.rs"]
mod saliency;
#[path = "execute/search.rs"]
mod search;
#[path = "execute/sentinel.rs"]
mod sentinel;

use crate::types::{Cli, Command};
use anyhow::Result;
use xiuxian_wendao::LinkGraphIndex;

/// Execute the CLI command.
///
/// This function dispatches the command into its respective handler.
/// All handlers take the CLI, an optional link graph index,
/// and return a Result indicating success or failure.
pub(crate) async fn execute(cli: &Cli, index: Option<&LinkGraphIndex>) -> Result<()> {
    match &cli.command {
        Command::Search(_) => search::handle(cli, index),
        Command::Audit(args) => audit::handle(cli, args, index),
        Command::Attachments(_) => attachments::handle(cli, index),
        Command::Stats
        | Command::Toc(_)
        | Command::Neighbors(_)
        | Command::Related(_)
        | Command::Metadata(_)
        | Command::Resolve(_) => graph::handle(cli, index),
        Command::Saliency { .. } => saliency::handle(cli),
        Command::Hmas { .. } => hmas::handle(cli),
        Command::Agentic { .. } => agentic::handle(cli, index),
        Command::Repo { .. } => repo::handle(cli),
        Command::Fix(args) => fix::handle(cli, args, index),
        #[cfg(feature = "zhenfa-router")]
        Command::Gateway(args) => gateway::handle(cli, args, index).await,
        Command::Sentinel(args) => sentinel::handle(cli, args, index).await,
    }
}
