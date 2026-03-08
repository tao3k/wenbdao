use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static PROJECT_ROOT_CACHE: OnceLock<PathBuf> = OnceLock::new();
static PRJ_CONFIG_HOME_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();
static WENDAO_CONFIG_FILE_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

/// CLI/runtime override for config home (`$PRJ_CONFIG_HOME` equivalent).
///
/// Used by callers that need Rust-side runtime and index scope to resolve
/// against a specific config directory during local experiments.
///
/// # Errors
///
/// Returns an error when a different override has already been set.
pub fn set_link_graph_config_home_override(path: PathBuf) -> Result<(), String> {
    let normalized = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };

    if let Some(existing) = PRJ_CONFIG_HOME_OVERRIDE.get() {
        if existing == &normalized {
            return Ok(());
        }
        return Err(format!(
            "link_graph config home override already set to '{}' (requested '{}')",
            existing.display(),
            normalized.display()
        ));
    }

    PRJ_CONFIG_HOME_OVERRIDE
        .set(normalized)
        .map_err(|_| "failed to set link_graph config home override".to_string())
}

/// CLI/runtime override for the exact wendao config file path.
///
/// Used by `wendao --conf <file>` for deterministic experiments where the
/// caller provides a concrete YAML file location.
///
/// # Errors
///
/// Returns an error when a different override has already been set.
pub fn set_link_graph_wendao_config_override(path: PathBuf) -> Result<(), String> {
    let normalized = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };

    if let Some(existing) = WENDAO_CONFIG_FILE_OVERRIDE.get() {
        if existing == &normalized {
            return Ok(());
        }
        return Err(format!(
            "wendao config override already set to '{}' (requested '{}')",
            existing.display(),
            normalized.display()
        ));
    }

    WENDAO_CONFIG_FILE_OVERRIDE
        .set(normalized)
        .map_err(|_| "failed to set wendao config override".to_string())
}

fn resolve_project_root_uncached() -> PathBuf {
    if let Ok(raw) = std::env::var("PRJ_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.is_absolute() {
                return path;
            }
            if let Ok(cwd) = std::env::current_dir() {
                return cwd.join(path);
            }
            return path;
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut cursor = cwd.clone();
    loop {
        let marker = cursor.join(".git");
        if marker.exists() {
            return cursor;
        }
        match cursor.parent() {
            Some(parent) => cursor = parent.to_path_buf(),
            None => return cwd,
        }
    }
}

pub(super) fn resolve_project_root() -> PathBuf {
    PROJECT_ROOT_CACHE
        .get_or_init(resolve_project_root_uncached)
        .clone()
}

pub(super) fn resolve_prj_config_home(project_root: &Path) -> PathBuf {
    if let Some(override_path) = PRJ_CONFIG_HOME_OVERRIDE.get() {
        return override_path.clone();
    }

    if let Ok(raw) = std::env::var("PRJ_CONFIG_HOME") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            return if path.is_absolute() {
                path
            } else {
                project_root.join(path)
            };
        }
    }
    project_root.join(".config")
}

pub(super) fn wendao_config_file_override() -> Option<PathBuf> {
    WENDAO_CONFIG_FILE_OVERRIDE.get().cloned()
}
