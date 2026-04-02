use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

static LINK_GRAPH_CONFIG_HOME_OVERRIDE: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static LINK_GRAPH_WENDAO_CONFIG_OVERRIDE: OnceLock<RwLock<Option<String>>> = OnceLock::new();

fn config_home_override_store() -> &'static RwLock<Option<String>> {
    LINK_GRAPH_CONFIG_HOME_OVERRIDE.get_or_init(|| RwLock::new(None))
}

fn wendao_config_override_store() -> &'static RwLock<Option<String>> {
    LINK_GRAPH_WENDAO_CONFIG_OVERRIDE.get_or_init(|| RwLock::new(None))
}

/// Override the global wendao configuration home directory.
pub fn set_link_graph_config_home_override(path: &str) {
    let mut guard = match config_home_override_store().write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(path.trim().to_string());
}

/// Override the global wendao configuration file path.
pub fn set_link_graph_wendao_config_override(path: &str) {
    let mut guard = match wendao_config_override_store().write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(path.trim().to_string());
}

#[must_use]
pub fn wendao_config_file_override() -> Option<PathBuf> {
    let guard = match wendao_config_override_store().read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.clone().map(PathBuf::from)
}
