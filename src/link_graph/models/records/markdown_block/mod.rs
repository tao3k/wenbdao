//! Block-level granularity for semantic addressing.

mod address;
mod block;
mod hash;
mod kind;

pub use address::{BlockAddress, BlockKindSpecifier};
pub use block::MarkdownBlock;
pub use kind::MarkdownBlockKind;

pub(super) use hash::compute_block_hash;

#[cfg(test)]
#[path = "../../../../../tests/unit/link_graph/models/records/markdown_block.rs"]
mod tests;
