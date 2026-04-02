//! Markdown compilation pipeline for Studio.

mod compiler;
mod handlers;
mod retrieval;
mod types;
mod utils;

#[cfg(test)]
mod tests;

pub(crate) use self::compiler::compile_markdown_ir;
pub(crate) use self::types::CompiledDocument;
