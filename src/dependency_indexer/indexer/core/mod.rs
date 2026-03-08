use std::collections::HashMap;
use std::path::PathBuf;

use super::{
    DependencyBuildConfig, DependencyIndexResult, DependencyStats, ExternalSymbol, SymbolIndex,
};

mod build;
mod process;
mod query;

/// Dependency indexer that coordinates parsing, fetching, and symbol extraction.
#[derive(Debug, Clone)]
pub struct DependencyIndexer {
    /// Symbol index for fast lookup
    pub symbol_index: SymbolIndex,
    /// Crate name to version mapping
    crate_versions: HashMap<String, String>,
    /// Project root directory
    project_root: PathBuf,
    /// Config path
    config_path: Option<PathBuf>,
}

impl DependencyIndexer {
    /// Create a new dependency indexer.
    pub fn new(project_root: &str, config_path: Option<&str>) -> Self {
        Self {
            symbol_index: SymbolIndex::new(),
            crate_versions: HashMap::new(),
            project_root: PathBuf::from(project_root),
            config_path: config_path.map(PathBuf::from),
        }
    }
}
