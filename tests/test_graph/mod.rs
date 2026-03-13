#![allow(
    missing_docs,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
//! Integration tests for the KnowledgeGraph module.
//!
//! Covers: CRUD, multi-hop search, persistence, skill registration,
//! query-time tool relevance, and export/import roundtrip.

use tempfile::TempDir;
use xiuxian_wendao::graph::{KnowledgeGraph, SkillDoc, entity_from_dict};
use xiuxian_wendao::{Entity, EntityType, Relation, RelationType};

fn has_valkey() -> bool {
    std::env::var("VALKEY_URL")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

mod entity_relation_crud;
mod entity_search_scoring;
mod graph_persistence;
mod graph_traversal;
mod skill_registration;
mod tool_relevance;
mod valkey_persistence;
