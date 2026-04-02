//! Code-AST response builders and repository/path resolution helpers.

pub(crate) mod atoms;
pub(crate) mod blocks;
pub(crate) mod resolve;
pub(crate) mod response;

pub use resolve::resolve_code_ast_repository_and_path;
pub use response::build_code_ast_analysis_response;
