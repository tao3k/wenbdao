use std::path::{Path, PathBuf};
use std::time::Duration;

pub(crate) const REAL_WORKSPACE_ROOT_ENV: &str = "XIUXIAN_WENDAO_GATEWAY_PERF_WORKSPACE_ROOT";
pub(crate) const REAL_WORKSPACE_READY_TIMEOUT_ENV: &str =
    "XIUXIAN_WENDAO_GATEWAY_PERF_READY_TIMEOUT_SECS";
pub(crate) const DEFAULT_REAL_WORKSPACE_ROOT: &str = ".data/wendao-frontend";
pub(crate) const DEFAULT_REAL_WORKSPACE_READY_TIMEOUT_SECS: u64 = 900;

#[derive(Debug, Clone)]
pub(crate) enum GatewayPerfRoot {
    Owned(PathBuf),
    External(PathBuf),
}

pub(crate) fn create_perf_root() -> anyhow::Result<PathBuf> {
    let root = std::env::temp_dir().join(format!(
        "xiuxian-wendao-gateway-perf-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

pub(crate) fn resolve_real_workspace_root() -> Option<PathBuf> {
    let project_root = xiuxian_io::PrjDirs::project_root();
    resolve_real_workspace_root_with_lookup(project_root.as_path(), &|key| std::env::var(key).ok())
}

pub(crate) fn resolve_real_workspace_root_with_lookup(
    project_root: &Path,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<PathBuf> {
    if let Some(path) = lookup(REAL_WORKSPACE_ROOT_ENV)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        let path = PathBuf::from(path);
        let resolved = if path.is_absolute() {
            path
        } else {
            project_root.join(path)
        };
        return Some(resolved);
    }

    let fallback = project_root.join(DEFAULT_REAL_WORKSPACE_ROOT);
    fallback.exists().then_some(fallback)
}

pub(crate) fn real_workspace_ready_timeout() -> Duration {
    let parsed = std::env::var(REAL_WORKSPACE_READY_TIMEOUT_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0);
    Duration::from_secs(parsed.unwrap_or(DEFAULT_REAL_WORKSPACE_READY_TIMEOUT_SECS))
}
