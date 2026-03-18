use std::path::PathBuf;
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
pub fn wendao_config_file_override() -> Option<PathBuf> {
    LINK_GRAPH_WENDAO_CONFIG_OVERRIDE.get().map(PathBuf::from)
}
