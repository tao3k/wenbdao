//! Centralized Python binding surface for `xiuxian-wendao`.
//!
//! This module owns the PyO3 boundary and the domain-specific binding modules.

/// Python bindings for dependency indexing.
pub mod dep_indexer_py;
/// Python bindings for note enhancement helpers.
pub mod enhancer_py;
/// Python bindings for fusion recall scoring.
pub mod fusion_py;
/// Python bindings for knowledge graph primitives.
pub mod graph_py;
/// Python bindings for knowledge categories and entries.
pub mod knowledge_py;
/// Python bindings for the LinkGraph engine surface.
pub mod link_graph_py;
/// Python bindings for LinkGraph entity reference helpers.
pub mod link_graph_refs_py;
mod python_module;
/// Python bindings for schema lookup helpers.
pub mod schema_py;
/// Python bindings for `KnowledgeStorage`.
pub mod storage_py;
/// Python bindings for incremental sync helpers.
pub mod sync_py;
/// Python bindings for unified symbol indexing.
pub mod unified_symbol_py;

pub use dep_indexer_py::{
    PyDependencyConfig, PyDependencyIndexResult, PyDependencyIndexer, PyDependencyStats,
    PyExternalDependency, PyExternalSymbol, PySymbolIndex,
};
pub use enhancer_py::{
    PyEnhancedNote, PyInferredRelation, PyNoteFrontmatter, link_graph_enhance_note,
    link_graph_enhance_notes_batch, link_graph_parse_frontmatter,
};
pub use graph_py::{
    PyEntity, PyEntityType, PyKnowledgeGraph, PyQueryIntent, PyRelation, PySkillDoc,
    extract_query_intent, invalidate_kg_cache, load_kg_from_valkey_cached,
};
pub use knowledge_py::{PyKnowledgeCategory, PyKnowledgeEntry, create_knowledge_entry};
pub use link_graph_py::{
    PyLinkGraphEngine, link_graph_stats_cache_del, link_graph_stats_cache_get,
    link_graph_stats_cache_set,
};
pub use link_graph_refs_py::{
    PyLinkGraphEntityRef, PyLinkGraphRefStats, link_graph_count_refs,
    link_graph_extract_entity_refs, link_graph_find_referencing_notes, link_graph_get_ref_stats,
    link_graph_is_valid_ref, link_graph_parse_entity_ref,
};
pub use storage_py::PyKnowledgeStorage;
pub use sync_py::{PySyncEngine, PySyncResult, compute_hash};
pub use unified_symbol_py::{PyUnifiedIndexStats, PyUnifiedSymbol, PyUnifiedSymbolIndex};
