use super::{SymbolSource, UnifiedSymbol, UnifiedSymbolIndex};

impl UnifiedSymbolIndex {
    /// Search across both project and external symbols.
    #[must_use]
    pub fn search_unified(&self, pattern: &str, limit: usize) -> Vec<&UnifiedSymbol> {
        let pattern = pattern.to_lowercase();
        let mut results: Vec<&UnifiedSymbol> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.name.to_lowercase().contains(&pattern))
            .collect();

        results.truncate(limit);
        results
    }

    /// Search only project symbols.
    #[must_use]
    pub fn search_project(&self, pattern: &str, limit: usize) -> Vec<&UnifiedSymbol> {
        let pattern = pattern.to_lowercase();
        self.symbols
            .iter()
            .filter(|symbol| symbol.name.to_lowercase().contains(&pattern) && symbol.is_project())
            .take(limit)
            .collect()
    }

    /// Search only external symbols.
    #[must_use]
    pub fn search_external(&self, pattern: &str, limit: usize) -> Vec<&UnifiedSymbol> {
        let pattern = pattern.to_lowercase();
        self.symbols
            .iter()
            .filter(|symbol| symbol.name.to_lowercase().contains(&pattern) && symbol.is_external())
            .take(limit)
            .collect()
    }

    /// Search within a specific crate (project or external).
    #[must_use]
    pub fn search_crate(
        &self,
        crate_name: &str,
        pattern: &str,
        limit: usize,
    ) -> Vec<&UnifiedSymbol> {
        let pattern = pattern.to_lowercase();
        self.symbols
            .iter()
            .filter(|symbol| {
                symbol.crate_name == crate_name && symbol.name.to_lowercase().contains(&pattern)
            })
            .take(limit)
            .collect()
    }

    /// Find where an external crate's symbols are used in the project.
    #[must_use]
    pub fn find_external_usage(&self, crate_name: &str) -> Vec<&str> {
        self.external_usage
            .get(crate_name)
            .map(|rows| rows.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Find project files that use a specific external symbol.
    #[must_use]
    pub fn find_symbol_usage(&self, symbol_name: &str, crate_name: &str) -> Vec<&str> {
        self.symbols
            .iter()
            .filter(|symbol| {
                symbol.name == symbol_name
                    && matches!(symbol.source, SymbolSource::External(ref c) if c == crate_name)
            })
            .map(|symbol| symbol.location.as_str())
            .collect()
    }

    /// Get all external crates used in the project.
    #[must_use]
    pub fn get_external_crates(&self) -> Vec<&str> {
        self.external_usage.keys().map(String::as_str).collect()
    }

    /// Get all project crates.
    #[must_use]
    pub fn get_project_crates(&self) -> Vec<&str> {
        let mut crates: Vec<&str> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.is_project())
            .map(|symbol| symbol.crate_name.as_str())
            .collect();

        crates.sort_unstable();
        crates.dedup();
        crates
    }
}
