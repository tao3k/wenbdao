use super::{UnifiedIndexStats, UnifiedSymbolIndex};

impl UnifiedSymbolIndex {
    /// Get statistics.
    #[must_use]
    pub fn stats(&self) -> UnifiedIndexStats {
        let project_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.is_project())
            .count();
        let external_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.is_external())
            .count();

        UnifiedIndexStats {
            total_symbols: self.symbols.len(),
            project_symbols: project_count,
            external_symbols: external_count,
            external_crates: self.external_usage.len(),
            project_files_with_externals: self.project_files.len(),
        }
    }
}
