//! Integration tests for the `KnowledgeGraph` module.
//!
//! Covers: CRUD, multi-hop search, persistence, skill registration,
//! query-time tool relevance, and export/import roundtrip.

mod entity_relation_crud;
mod entity_search_scoring;
mod graph_persistence;
mod graph_traversal;
mod skill_registration;
mod support;
mod tool_relevance;
mod valkey_persistence;
