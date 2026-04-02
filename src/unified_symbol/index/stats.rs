use crate::unified_symbol::{UnifiedIndexStats, UnifiedSymbolIndex};

impl UnifiedSymbolIndex {
    /// Return index statistics.
    #[must_use]
    pub fn stats(&self) -> UnifiedIndexStats {
        let mut project_symbols = 0;
        let mut external_symbols = 0;
        for symbol in &self.symbols {
            if symbol.is_project() {
                project_symbols += 1;
            } else {
                external_symbols += 1;
            }
        }

        UnifiedIndexStats {
            total_symbols: self.symbols.len(),
            project_symbols,
            external_symbols,
            external_crates: self.external_usage.len(),
            project_files_with_externals: self.project_files.len(),
        }
    }
}
