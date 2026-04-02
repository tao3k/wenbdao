//! `LinkGraph` note enhancement engine.
//!
//! Secondary analysis for `LinkGraph` query results.
//! - Parse YAML frontmatter into structured metadata
//! - Infer typed relations from note structure
//! - Batch enhance notes (frontmatter + entities + relations)
//!
//! The `LinkGraph` backend remains the primary engine for scanning, building the link
//! graph, and querying. This module enriches results with deeper
//! structural analysis at Rust-native speed.

mod frontmatter;
pub mod markdown_config;
mod pipeline;
mod relations;
mod resource_registry;
mod resource_semantics;
mod types;

pub use frontmatter::parse_frontmatter;
pub use pipeline::{enhance_note, enhance_notes_batch};
pub use relations::infer_relations;
pub use resource_registry::types::{WendaoResourceLinkTarget, WendaoResourceRegistry};
pub use resource_semantics::classify_skill_reference;
pub use types::{
    EnhancedNote, EntityRefData, InferredRelation, NoteFrontmatter, NoteInput, RefStatsData,
};

#[cfg(test)]
mod tests;
