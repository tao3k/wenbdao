//! Fuzzy path matching for human discovery.
//!
//! The `TopologyIndex` provides structural path indexing and fuzzy matching capabilities
//! for discovering nodes when the exact path or title is not known.

mod build;
mod helpers;
mod search;
mod types;

pub use types::{MatchType, PathEntry, PathMatch, TopologyIndex};

#[cfg(test)]
#[path = "../../../../tests/unit/link_graph/addressing/topology.rs"]
mod tests;
