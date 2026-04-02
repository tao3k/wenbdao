//! PyO3 bindings for dependency indexer.
#![allow(clippy::doc_markdown)]

mod config;
mod helpers;
mod indexer;
mod registration;
mod symbols;

pub use config::{PyDependencyConfig, PyExternalDependency};
pub use indexer::{PyDependencyIndexResult, PyDependencyIndexer, PyDependencyStats};
pub use registration::register_dependency_indexer_module;
pub use symbols::{PyExternalSymbol, PySymbolIndex};

#[cfg(test)]
mod tests;
