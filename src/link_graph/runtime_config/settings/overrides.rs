use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static LINK_GRAPH_CONFIG_HOME_OVERRIDE: OnceLock<String> = OnceLock::new();
static LINK_GRAPH_WENDAO_CONFIG_OVERRIDE: OnceLock<String> = OnceLock::new();

/// Override the global wendao configuration home directory.
pub fn set_link_graph_config_home_override(path: &str) {
    let _ = LINK_GRAPH_CONFIG_HOME_OVERRIDE.set(path.trim().to_string());
}

/// Override the global wendao configuration file path.
pub fn set_link_graph_wendao_config_override(path: &str) {
    let _ = LINK_GRAPH_WENDAO_CONFIG_OVERRIDE.set(path.trim().to_string());
}

#[must_use]
pub fn resolve_project_root() -> PathBuf {
    if let Some(overridden) = LINK_GRAPH_CONFIG_HOME_OVERRIDE.get() {
        return PathBuf::from(overridden);
    }
    std::env::var("XIUXIAN_WENDAO_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

#[must_use]
pub fn wendao_config_file_override() -> Option<PathBuf> {
    LINK_GRAPH_WENDAO_CONFIG_OVERRIDE.get().map(PathBuf::from)
}

#[must_use]
pub fn resolve_prj_config_home(root: &Path) -> PathBuf {
    root.join(".config")
}
