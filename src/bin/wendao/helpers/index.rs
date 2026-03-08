use super::super::types::Cli;
use anyhow::Result;
use std::path::PathBuf;
use xiuxian_wendao::{LinkGraphIndex, resolve_link_graph_index_runtime};

pub(crate) fn build_index(cli: &Cli) -> Result<LinkGraphIndex> {
    let (include_dirs, exclude_dirs) = if cli.config_file.is_some() {
        let root_for_scope = if cli.root.is_absolute() {
            cli.root.clone()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(&cli.root)
        };
        let runtime_scope = resolve_link_graph_index_runtime(&root_for_scope);
        let include = if cli.include_dirs.is_empty() {
            runtime_scope.include_dirs
        } else {
            cli.include_dirs.clone()
        };
        let exclude = if cli.exclude_dirs.is_empty() {
            runtime_scope.exclude_dirs
        } else {
            cli.exclude_dirs.clone()
        };
        (include, exclude)
    } else {
        (cli.include_dirs.clone(), cli.exclude_dirs.clone())
    };

    LinkGraphIndex::build_with_cache(&cli.root, &include_dirs, &exclude_dirs)
        .map_err(anyhow::Error::msg)
}
