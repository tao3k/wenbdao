//! Command-line interface entrypoint for xiuxian-wendao link-graph operations.

use anyhow::{Result, anyhow};
use clap::Parser;

#[path = "wendao/execute.rs"]
mod execute;
#[path = "wendao/helpers/mod.rs"]
mod helpers;
#[path = "wendao/types.rs"]
mod types;

use execute::execute;
use helpers::build_index;
use types::{AgenticCommand, Cli, Command};
use xiuxian_logging::init_from_cli;
use xiuxian_wendao::set_link_graph_wendao_config_override;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_from_cli("xiuxian_wendao", &cli.logging).map_err(|err| anyhow!(err))?;

    let mut config_path = cli.config_file.clone();
    if config_path.is_none() {
        let local_toml = std::path::Path::new("wendao.toml");
        if local_toml.exists() {
            config_path = Some(local_toml.to_path_buf());
        }
    }

    if let Some(conf) = &config_path
        && let Some(path_str) = conf.to_str()
    {
        set_link_graph_wendao_config_override(path_str);
    }

    let needs_index = matches!(
        &cli.command,
        Command::Search(_)
            | Command::Audit(_)
            | Command::Attachments(_)
            | Command::Stats
            | Command::Toc(_)
            | Command::Neighbors(_)
            | Command::Related(_)
            | Command::Metadata(_)
            | Command::Resolve(_)
            | Command::Fix(_)
            | Command::Sentinel(_)
            | Command::Agentic {
                command: AgenticCommand::Plan { .. } | AgenticCommand::Run { .. },
            }
    );
    if needs_index {
        let index = build_index(&cli)?;
        execute(&cli, Some(&index)).await
    } else {
        execute(&cli, None).await
    }
}
