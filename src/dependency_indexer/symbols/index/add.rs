use super::{CrateSymbols, SymbolIndex};
use crate::dependency_indexer::symbols::ExternalSymbol;

impl SymbolIndex {
    /// Add symbols from a source file.
    pub fn add_symbols(&mut self, crate_name: &str, symbols: &[ExternalSymbol]) {
        let idx = if let Some(&idx) = self.crate_map.get(crate_name) {
            idx
        } else {
            let idx = self.by_crate.len();
            self.crate_map.insert(crate_name.to_string(), idx);
            self.by_crate.push(CrateSymbols {
                name: crate_name.to_string(),
                symbols: Vec::new(),
            });
            idx
        };

        self.by_crate[idx].symbols.extend(symbols.to_vec());
    }

    /// Clear all symbols.
    pub fn clear(&mut self) {
        self.by_crate.clear();
        self.crate_map.clear();
    }
}
