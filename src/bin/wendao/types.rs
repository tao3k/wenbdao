//! CLI argument contracts and enum adapters for `wendao`.

#[path = "types/cli.rs"]
mod cli;
#[path = "types/commands/mod.rs"]
mod commands;
#[path = "types/enums.rs"]
mod enums;

pub(crate) use cli::Cli;
pub(crate) use commands::{
    AgenticCommand, AuditArgs, Command, FixArgs, HmasCommand, SaliencyCommand, SentinelArgs,
    SentinelCommand, SentinelWatchArgs,
};
#[cfg(feature = "zhenfa-router")]
pub(crate) use commands::{GatewayArgs, GatewayCommand, GatewayStartArgs};
pub(crate) use commands::{RepoCommand, RepoSyncModeArg};
pub(crate) use enums::{LinkGraphScopeArg, OutputFormat, RelatedPprSubgraphModeArg};
