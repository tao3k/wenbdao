use super::ExternalSymbol;

mod add;
mod query;
mod serde;
mod stats;

/// Symbol index for fast lookup.
#[derive(Debug, Default, Clone)]
pub struct SymbolIndex {
    /// Symbols grouped by crate/package
    by_crate: Vec<CrateSymbols>,
    /// Map `crate_name` -> index
    crate_map: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone)]
struct CrateSymbols {
    name: String,
    symbols: Vec<ExternalSymbol>,
}

impl SymbolIndex {
    /// Create an empty symbol index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_crate: Vec::new(),
            crate_map: std::collections::HashMap::new(),
        }
    }
}
