//! Repository search functions (overview, module, symbol, example, import, doc coverage).
mod artifacts;
mod contracts;
mod coverage;
mod documents;
mod example;
mod imports;
mod indexed_exact;
mod indexed_fuzzy;
mod legacy;
mod module;
mod overview;
mod ranking;
mod symbol;

#[cfg(test)]
mod tests;

pub use coverage::*;
pub use example::*;
pub use imports::*;
pub use module::*;
pub use overview::*;
pub use symbol::*;

pub(crate) use artifacts::repository_search_artifacts;
pub(crate) use contracts::{
    RepoAnalysisFallbackContract, canonical_import_query_text, example_fallback_contract,
    import_fallback_contract, module_fallback_contract, symbol_fallback_contract,
};
pub(crate) use documents::ExampleSearchMetadata;
