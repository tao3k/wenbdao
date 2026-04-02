//! Logic for resolving the best semantic definition for a query.

mod filters;
pub(crate) mod resolve;
#[cfg(test)]
mod tests;

pub use resolve::{DefinitionResolveOptions, resolve_best_definition};
