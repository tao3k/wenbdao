use std::fs;
use std::path::{Path, PathBuf};

use crate::analyzers::errors::RepoIntelligenceError;

use super::parse::{
    normalize_path, parse_refresh_policy, parse_repository_plugins, parse_repository_ref,
};
use super::toml::WendaoTomlConfig;
use super::types::{RegisteredRepository, RepoIntelligenceConfig};

/// Load the repo intelligence configuration from the project.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when configuration cannot be loaded.
pub fn load_repo_intelligence_config(
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoIntelligenceConfig, RepoIntelligenceError> {
    let config_path = config_path.map_or_else(|| cwd.join("wendao.toml"), Path::to_path_buf);
    let contents =
        fs::read_to_string(&config_path).map_err(|error| RepoIntelligenceError::ConfigLoad {
            message: format!("failed to read `{}`: {error}", config_path.display()),
        })?;
    let parsed: WendaoTomlConfig =
        toml::from_str(&contents).map_err(|error| RepoIntelligenceError::ConfigLoad {
            message: format!("failed to parse `{}`: {error}", config_path.display()),
        })?;

    let config_root = config_path
        .parent()
        .map_or_else(|| cwd.to_path_buf(), Path::to_path_buf);

    let repos = parsed
        .link_graph
        .projects
        .into_iter()
        .map(|(id, project)| {
            let plugins = parse_repository_plugins(project.plugins, &id, &config_path)?;
            if plugins.is_empty() {
                return Ok(None);
            }

            let path = project
                .root
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .map(|path| {
                    if path.is_absolute() {
                        normalize_path(path.as_path())
                    } else {
                        normalize_path(config_root.join(path).as_path())
                    }
                });
            let url = project
                .url
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            if path.is_none() && url.is_none() {
                return Ok(None);
            }

            Ok(Some(RegisteredRepository {
                id,
                path,
                url,
                git_ref: project.git_ref.as_deref().and_then(parse_repository_ref),
                refresh: parse_refresh_policy(project.refresh.as_deref()),
                plugins,
            }))
        })
        .collect::<Result<Vec<_>, RepoIntelligenceError>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(RepoIntelligenceConfig { repos })
}
