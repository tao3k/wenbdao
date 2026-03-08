//! Knowledge graph storage and operations.
//!
//! Modular design:
//! - `mod.rs`: module declarations and public re-exports
//! - `core.rs`: core `KnowledgeGraph` type and lock helpers
//! - `errors.rs`: graph error definitions
//! - `entity_ops.rs`: entity CRUD operations
//! - `relation_ops.rs`: relation CRUD operations
//! - `stats.rs`: aggregate graph statistics
//! - `query/`: search and traversal algorithms
//! - `intent.rs`: Lightweight query intent extractor (action/target/context)
//! - `persistence.rs`: JSON save/load, entity/relation parsing
//! - `valkey_persistence.rs`: Valkey save/load (runtime-native persistence)
//! - `dedup.rs`: Entity deduplication and normalization
//! - `skill_registry.rs`: Bulk skill entity registration (Bridge 4)

mod core;
mod dedup;
mod entity_ops;
mod errors;
mod intent;
mod persistence;
mod query;
mod relation_ops;
mod skill_registry;
mod stats;
pub(crate) mod valkey_persistence;

pub use core::KnowledgeGraph;
pub use errors::GraphError;

// Re-export sub-module public items
pub use dedup::DeduplicationResult;
pub use intent::{QueryIntent, extract_intent};
pub use persistence::{entity_from_dict, relation_from_dict};
pub use skill_registry::{SkillDoc, SkillRegistrationResult};
