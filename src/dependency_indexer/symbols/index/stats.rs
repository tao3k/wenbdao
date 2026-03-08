use super::SymbolIndex;

impl SymbolIndex {
    /// Get total symbol count.
    #[must_use]
    pub fn symbol_count(&self) -> usize {
        self.by_crate
            .iter()
            .map(|crate_row| crate_row.symbols.len())
            .sum()
    }

    /// Get crate count.
    #[must_use]
    pub fn crate_count(&self) -> usize {
        self.by_crate.len()
    }
}
