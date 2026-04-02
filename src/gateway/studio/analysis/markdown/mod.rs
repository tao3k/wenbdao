//! Markdown analysis engine for Studio.

mod compile;
mod text;

pub(crate) use self::compile::{CompiledDocument, compile_markdown_ir};
