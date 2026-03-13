//! CLI argument contracts and enum adapters for `wendao`.

#[path = "types/cli.rs"]
mod cli;
#[path = "types/commands/mod.rs"]
mod commands;
#[path = "types/enums.rs"]
mod enums;

pub(crate) use cli::Cli;
pub(crate) use commands::{AgenticCommand, Command, HmasCommand, SaliencyCommand};
pub(crate) use enums::{LinkGraphScopeArg, OutputFormat, RelatedPprSubgraphModeArg};
