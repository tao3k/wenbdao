use super::SymbolIndex;
use crate::dependency_indexer::symbols::ExternalSymbol;

impl SymbolIndex {
    /// Search for symbols matching a pattern.
    #[must_use]
    pub fn search(&self, pattern: &str, limit: usize) -> Vec<ExternalSymbol> {
        let pattern = pattern.to_lowercase();
        let mut results: Vec<&ExternalSymbol> = self
            .by_crate
            .iter()
            .flat_map(|crate_row| crate_row.symbols.iter())
            .filter(|symbol| symbol.name.to_lowercase().contains(&pattern))
            .collect();

        results.sort_by_key(|item| item.name.len());
        results.truncate(limit);

        results.into_iter().cloned().collect()
    }

    /// Search within a specific crate.
    #[must_use]
    pub fn search_crate(
        &self,
        crate_name: &str,
        pattern: &str,
        limit: usize,
    ) -> Vec<ExternalSymbol> {
        let pattern = pattern.to_lowercase();

        if let Some(&idx) = self.crate_map.get(crate_name) {
            let symbols = &self.by_crate[idx].symbols;
            let mut results: Vec<&ExternalSymbol> = symbols
                .iter()
                .filter(|symbol| symbol.name.to_lowercase().contains(&pattern))
                .collect();

            results.truncate(limit);
            return results.into_iter().cloned().collect();
        }

        Vec::new()
    }

    /// Get all indexed crate/package names.
    #[must_use]
    pub fn get_crates(&self) -> Vec<&str> {
        self.by_crate
            .iter()
            .map(|crate_row| crate_row.name.as_str())
            .collect()
    }
}
