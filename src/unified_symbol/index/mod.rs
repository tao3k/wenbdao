use std::collections::HashMap;

use super::{UnifiedIndexStats, UnifiedSymbol, symbol::SymbolSource};

mod add;
mod query;
mod stats;

/// Unified Symbol Index - combines project and external symbols.
#[derive(Debug, Default, Clone)]
pub struct UnifiedSymbolIndex {
    /// All symbols indexed by lowercase name
    by_name: HashMap<String, Vec<usize>>,
    /// All symbols stored in a vector
    symbols: Vec<UnifiedSymbol>,
    /// External crate usage in project (`crate_name` -> project locations)
    external_usage: HashMap<String, Vec<String>>,
    /// Project files that use external crates
    project_files: HashMap<String, Vec<String>>, // file -> [symbol names]
}

impl UnifiedSymbolIndex {
    /// Create an empty unified index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_name: HashMap::new(),
            symbols: Vec::new(),
            external_usage: HashMap::new(),
            project_files: HashMap::new(),
        }
    }
}
