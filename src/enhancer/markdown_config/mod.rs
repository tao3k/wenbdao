//! Markdown configuration extraction and registry helpers for embedded resources.

mod blocks;
mod index;
mod links;
mod types;

#[cfg(test)]
mod tests;

pub use index::MarkdownConfigMemoryIndex;
pub use types::MarkdownConfigBlock;

pub use blocks::extract_markdown_config_blocks;
pub use links::extract_markdown_config_link_targets_by_id;
