use std::path::{Path, PathBuf};

use xiuxian_io::PrjDirs;

/// Returns the path to `wendao.toml` for the given config root.
#[must_use]
pub fn studio_wendao_toml_path(config_root: &Path) -> PathBuf {
    config_root.join("wendao.toml")
}

/// Resolves the studio config root directory.
#[must_use]
pub fn resolve_studio_config_root(project_root: &Path) -> PathBuf {
    let candidate = PrjDirs::data_home().join("wendao-frontend");
    if candidate.exists() {
        candidate
    } else {
        project_root.to_path_buf()
    }
}
