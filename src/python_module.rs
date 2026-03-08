//! Python module registration helpers for `_xiuxian_wendao`.

use pyo3::prelude::*;

pub(crate) fn register(py: Python<'_>, m: &Bound<PyModule>) -> PyResult<()> {
    // Knowledge types
    m.add_class::<crate::knowledge_py::PyKnowledgeCategory>()?;
    m.add_class::<crate::knowledge_py::PyKnowledgeEntry>()?;
    m.add_function(wrap_pyfunction!(
        crate::knowledge_py::create_knowledge_entry,
        py
    )?)?;

    // Storage
    m.add_class::<crate::storage_py::PyKnowledgeStorage>()?;

    // Sync
    m.add_class::<crate::sync_py::PySyncEngine>()?;
    m.add_class::<crate::sync_py::PySyncResult>()?;
    m.add_function(wrap_pyfunction!(crate::sync_py::compute_hash, py)?)?;

    // Knowledge graph
    m.add_class::<crate::graph_py::PyEntity>()?;
    m.add_class::<crate::graph_py::PyRelation>()?;
    m.add_class::<crate::graph_py::PyKnowledgeGraph>()?;
    m.add_class::<crate::graph_py::PySkillDoc>()?;
    m.add_class::<crate::graph_py::PyQueryIntent>()?;
    m.add_function(wrap_pyfunction!(crate::graph_py::extract_query_intent, py)?)?;
    m.add_function(wrap_pyfunction!(crate::graph_py::invalidate_kg_cache, py)?)?;
    m.add_function(wrap_pyfunction!(
        crate::graph_py::load_kg_from_valkey_cached,
        py
    )?)?;
    m.add_class::<crate::link_graph_py::PyLinkGraphEngine>()?;
    m.add_function(wrap_pyfunction!(
        crate::link_graph_py::link_graph_stats_cache_get,
        py
    )?)?;
    m.add_function(wrap_pyfunction!(
        crate::link_graph_py::link_graph_stats_cache_set,
        py
    )?)?;
    m.add_function(wrap_pyfunction!(
        crate::link_graph_py::link_graph_stats_cache_del,
        py
    )?)?;

    // LinkGraph entity references
    m.add_class::<crate::PyLinkGraphEntityRef>()?;
    m.add_class::<crate::PyLinkGraphRefStats>()?;
    m.add_function(wrap_pyfunction!(crate::link_graph_extract_entity_refs, py)?)?;
    m.add_function(wrap_pyfunction!(crate::link_graph_get_ref_stats, py)?)?;
    m.add_function(wrap_pyfunction!(crate::link_graph_parse_entity_ref, py)?)?;
    m.add_function(wrap_pyfunction!(crate::link_graph_is_valid_ref, py)?)?;
    m.add_function(wrap_pyfunction!(crate::link_graph_count_refs, py)?)?;
    m.add_function(wrap_pyfunction!(
        crate::link_graph_find_referencing_notes,
        py
    )?)?;

    // Enhancer
    m.add_class::<crate::PyEnhancedNote>()?;
    m.add_class::<crate::PyNoteFrontmatter>()?;
    m.add_class::<crate::PyInferredRelation>()?;
    m.add_function(wrap_pyfunction!(crate::link_graph_enhance_note, py)?)?;
    m.add_function(wrap_pyfunction!(crate::link_graph_enhance_notes_batch, py)?)?;
    m.add_function(wrap_pyfunction!(crate::link_graph_parse_frontmatter, py)?)?;

    // Fusion recall boost (LinkGraph proximity)
    m.add_function(wrap_pyfunction!(
        crate::fusion_py::apply_link_graph_proximity_boost_py,
        py
    )?)?;

    // Unified symbol index
    crate::unified_symbol_py::register_unified_symbol_module(m)?;

    // Schemas
    crate::schema_py::register_schema_module(m)?;

    Ok(())
}
