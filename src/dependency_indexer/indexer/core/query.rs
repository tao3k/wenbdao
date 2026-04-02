use super::DependencyIndexer;
use crate::dependency_indexer::indexer::{DependencyStats, ExternalSymbol};

impl DependencyIndexer {
    /// Search for symbols by name pattern.
    #[must_use]
    pub fn search(&self, pattern: &str, limit: usize) -> Vec<ExternalSymbol> {
        self.symbol_index.search(pattern, limit)
    }

    /// Search within a specific crate.
    #[must_use]
    pub fn search_crate(
        &self,
        crate_name: &str,
        pattern: &str,
        limit: usize,
    ) -> Vec<ExternalSymbol> {
        self.symbol_index.search_crate(crate_name, pattern, limit)
    }

    /// Get all indexed crate names.
    #[must_use]
    pub fn get_indexed(&self) -> Vec<String> {
        self.crate_versions.keys().cloned().collect()
    }

    /// Get statistics.
    #[must_use]
    pub fn stats(&self) -> DependencyStats {
        DependencyStats {
            total_crates: self.crate_versions.len(),
            total_symbols: self.symbol_index.symbol_count(),
        }
    }
}
