//! Parse dependencies from Cargo.toml - Root workspace priority.

mod parse;
mod regex;
mod types;

pub use parse::parse_cargo_dependencies;
pub use types::CargoDependency;
