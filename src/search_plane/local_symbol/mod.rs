mod build;
mod query;
mod schema;

pub(crate) use build::ensure_local_symbol_index_started;
#[cfg(test)]
pub(crate) use build::{LocalSymbolBuildError, publish_local_symbol_hits};
pub(crate) use query::{LocalSymbolSearchError, autocomplete_local_symbols, search_local_symbols};
