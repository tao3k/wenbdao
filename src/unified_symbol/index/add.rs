use super::{UnifiedSymbol, UnifiedSymbolIndex};

impl UnifiedSymbolIndex {
    /// Add a project symbol.
    pub fn add_project_symbol(&mut self, name: &str, kind: &str, location: &str, crate_name: &str) {
        let symbol = UnifiedSymbol::new_project(name, kind, location, crate_name);
        self.add_symbol(symbol);
    }

    /// Add an external dependency symbol.
    pub fn add_external_symbol(
        &mut self,
        name: &str,
        kind: &str,
        location: &str,
        crate_name: &str,
    ) {
        let symbol = UnifiedSymbol::new_external(name, kind, location, crate_name);
        self.add_symbol(symbol);
    }

    /// Record usage of an external symbol in a project file.
    pub fn record_external_usage(
        &mut self,
        crate_name: &str,
        symbol_name: &str,
        project_file: &str,
    ) {
        // Record in external_usage: which project locations use this external crate
        self.external_usage
            .entry(crate_name.to_string())
            .or_default()
            .push(project_file.to_string());

        // Record in project_files: which symbols are used in this file
        self.project_files
            .entry(project_file.to_string())
            .or_default()
            .push(symbol_name.to_string());
    }

    fn add_symbol(&mut self, symbol: UnifiedSymbol) {
        let idx = self.symbols.len();
        let key = symbol.name.to_lowercase();
        self.symbols.push(symbol);
        self.by_name.entry(key).or_default().push(idx);
    }

    /// Clear all symbols.
    pub fn clear(&mut self) {
        self.by_name.clear();
        self.symbols.clear();
        self.external_usage.clear();
        self.project_files.clear();
    }
}
