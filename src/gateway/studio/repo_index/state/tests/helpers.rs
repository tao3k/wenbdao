use std::path::PathBuf;
use std::sync::Arc;

use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::{RegisteredRepository, RepositoryPluginConfig, RepositoryRefreshPolicy};
use crate::gateway::studio::repo_index::state::coordinator::RepoIndexCoordinator;

pub(crate) fn repo(id: &str, path: &str) -> RegisteredRepository {
    RegisteredRepository {
        id: id.to_string(),
        path: Some(PathBuf::from(path)),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    }
}

pub(crate) fn new_coordinator(
    search_plane: crate::search_plane::SearchPlaneService,
) -> RepoIndexCoordinator {
    RepoIndexCoordinator::new(
        PathBuf::from("."),
        Arc::new(PluginRegistry::new()),
        search_plane,
    )
}
