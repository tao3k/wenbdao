//! Repo Intelligence command execution handler.

use std::env;

use anyhow::Result;

use crate::helpers::emit;
use crate::types::{Cli, Command, RepoCommand, RepoSyncModeArg};
use xiuxian_wendao::{
    DocCoverageQuery, ExampleSearchQuery, ModuleSearchQuery, RepoOverviewQuery, RepoSyncMode,
    RepoSyncQuery, SymbolSearchQuery, doc_coverage_from_config, example_search_from_config,
    module_search_from_config, repo_overview_from_config, repo_sync_from_config,
    symbol_search_from_config,
};

pub(super) fn handle(cli: &Cli) -> Result<()> {
    let Command::Repo { command } = &cli.command else {
        unreachable!("repo handler called with non-repo command");
    };

    match command {
        RepoCommand::Sync(args) => {
            let cwd = env::current_dir()?;
            let query = RepoSyncQuery {
                repo_id: args.repo.clone(),
                mode: match args.mode {
                    RepoSyncModeArg::Ensure => RepoSyncMode::Ensure,
                    RepoSyncModeArg::Refresh => RepoSyncMode::Refresh,
                    RepoSyncModeArg::Status => RepoSyncMode::Status,
                },
            };
            let result = repo_sync_from_config(&query, cli.config_file.as_deref(), &cwd)?;
            emit(&result, cli.output)
        }
        RepoCommand::Overview(args) => {
            let cwd = env::current_dir()?;
            let query = RepoOverviewQuery {
                repo_id: args.repo.clone(),
            };
            let result = repo_overview_from_config(&query, cli.config_file.as_deref(), &cwd)?;
            emit(&result, cli.output)
        }
        RepoCommand::ModuleSearch(args) => {
            let cwd = env::current_dir()?;
            let query = ModuleSearchQuery {
                repo_id: args.repo.clone(),
                query: args.query.clone(),
                limit: args.limit,
            };
            let result = module_search_from_config(&query, cli.config_file.as_deref(), &cwd)?;
            emit(&result, cli.output)
        }
        RepoCommand::SymbolSearch(args) => {
            let cwd = env::current_dir()?;
            let query = SymbolSearchQuery {
                repo_id: args.repo.clone(),
                query: args.query.clone(),
                limit: args.limit,
            };
            let result = symbol_search_from_config(&query, cli.config_file.as_deref(), &cwd)?;
            emit(&result, cli.output)
        }
        RepoCommand::ExampleSearch(args) => {
            let cwd = env::current_dir()?;
            let query = ExampleSearchQuery {
                repo_id: args.repo.clone(),
                query: args.query.clone(),
                limit: args.limit,
            };
            let result = example_search_from_config(&query, cli.config_file.as_deref(), &cwd)?;
            emit(&result, cli.output)
        }
        RepoCommand::DocCoverage(args) => {
            let cwd = env::current_dir()?;
            let query = DocCoverageQuery {
                repo_id: args.repo.clone(),
                module_id: args.module.clone(),
            };
            let result = doc_coverage_from_config(&query, cli.config_file.as_deref(), &cwd)?;
            emit(&result, cli.output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_overview_query_preserves_repo_id() {
        let query = RepoOverviewQuery {
            repo_id: "sciml".to_string(),
        };
        assert_eq!(query.repo_id, "sciml");
    }

    #[test]
    fn doc_coverage_query_preserves_optional_module_scope() {
        let query = DocCoverageQuery {
            repo_id: "sciml".to_string(),
            module_id: Some("BaseModelica".to_string()),
        };
        assert_eq!(query.repo_id, "sciml");
        assert_eq!(query.module_id.as_deref(), Some("BaseModelica"));
    }
}
