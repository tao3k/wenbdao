//! Code observation parsing for Blueprint v2.7 (Internal AST Integration).
//!
//! This module provides parsing support for the `:OBSERVE:` property drawer attribute,
//! enabling documentation to observe code patterns via `xiuxian-ast` structural queries.
//!
//! ## Format
//!
//! The `:OBSERVE:` attribute uses the following syntax:
//! ```markdown
//! :OBSERVE: lang:<language> "<sgrep-pattern>"
//! :OBSERVE: lang:<language> scope:"<path-filter>" "<sgrep-pattern>"
//! ```
//!
//! ## Scope Filter
//!
//! The optional `scope:` attribute restricts pattern matching to specific file paths.
//! This prevents false positives when the same symbol exists in multiple packages.
//!
//! ```markdown
//! ## API Handler
//! :OBSERVE: lang:rust scope:"src/api/**" "fn $NAME($$$) -> Result<$$$>"
//! ```
//!
//! ## Example
//!
//! ```markdown
//! ## Storage Module
//! :OBSERVE: lang:rust "fn $NAME($$$ARGS) -> Result<$$$RET, $$$ERR>"
//! ```

mod extract;
mod format;
mod glob;
mod types;

pub use extract::extract_observations;
pub use glob::path_matches_scope;
pub use types::CodeObservation;

#[cfg(test)]
#[path = "../../../../tests/unit/link_graph/parser/code_observation.rs"]
mod tests;
