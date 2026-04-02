use crate::analyzers::{
    RegisteredRepository, RepoIntelligenceError, RepositoryPluginConfig, RepositoryRef,
    RepositoryRefreshPolicy,
};
use crate::gateway::studio::pathing;

use super::state::StudioState;

/// Returns the configured repository by ID.
///
/// # Errors
///
/// Returns an error when no configured repository matches `repo_id`.
pub fn configured_repository(
    studio: &StudioState,
    repo_id: &str,
) -> Result<RegisteredRepository, RepoIntelligenceError> {
    let repositories = configured_repositories(studio);
    repositories
        .iter()
        .find(|repository| repository.id == repo_id)
        .cloned()
        .or_else(|| {
            repositories
                .into_iter()
                .find(|repository| repository.id.eq_ignore_ascii_case(repo_id))
        })
        .ok_or_else(|| RepoIntelligenceError::UnknownRepository {
            repo_id: repo_id.to_string(),
        })
}

/// Returns all configured repositories.
#[must_use]
pub fn configured_repositories(studio: &StudioState) -> Vec<RegisteredRepository> {
    studio
        .configured_repo_projects()
        .into_iter()
        .filter_map(|project| {
            if project.plugins.is_empty() {
                return None;
            }
            let path = project
                .root
                .as_deref()
                .and_then(|root| pathing::resolve_path_like(studio.config_root.as_path(), root));
            let url = project.url.map(|value| value.trim().to_string());
            if path.is_none() && url.is_none() {
                return None;
            }
            Some(RegisteredRepository {
                id: project.id,
                path,
                url,
                git_ref: project.git_ref.map(RepositoryRef::Branch),
                refresh: parse_refresh_policy(project.refresh.as_deref()),
                plugins: project
                    .plugins
                    .into_iter()
                    .map(RepositoryPluginConfig::Id)
                    .collect(),
            })
        })
        .collect()
}

fn parse_refresh_policy(refresh: Option<&str>) -> RepositoryRefreshPolicy {
    match refresh
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("fetch")
    {
        "manual" => RepositoryRefreshPolicy::Manual,
        _ => RepositoryRefreshPolicy::Fetch,
    }
}
