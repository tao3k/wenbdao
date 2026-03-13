//! Command dispatch implementation for `wendao` CLI.

#[path = "execute/agentic/mod.rs"]
mod agentic;
#[path = "execute/attachments.rs"]
mod attachments;
#[path = "execute/graph.rs"]
mod graph;
#[path = "execute/hmas.rs"]
mod hmas;
#[path = "execute/saliency.rs"]
mod saliency;
#[path = "execute/search.rs"]
mod search;

use crate::types::{Cli, Command};
use anyhow::Result;
use xiuxian_wendao::LinkGraphIndex;

pub(crate) fn execute(cli: &Cli, index: Option<&LinkGraphIndex>) -> Result<()> {
    match &cli.command {
        Command::Search(_) => search::handle(cli, index),
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
    }
}
