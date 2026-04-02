//! Smoke tests for the optional `pybindings` feature.

#[cfg(not(feature = "pybindings"))]
#[test]
fn core_surface_builds_without_python_bindings() {
    let _ = std::mem::size_of::<xiuxian_wendao::KnowledgeEntry>();
    let _ = std::mem::size_of::<xiuxian_wendao::KnowledgeCategory>();
}

#[cfg(feature = "pybindings")]
#[test]
fn python_binding_surface_compiles_when_feature_is_enabled() {
    let _ = std::mem::size_of::<xiuxian_wendao::pybindings::PyKnowledgeCategory>();
    let _ = std::mem::size_of::<xiuxian_wendao::pybindings::PyKnowledgeEntry>();
    let _ = std::mem::size_of::<xiuxian_wendao::pybindings::PyKnowledgeGraph>();
    let _ = std::mem::size_of::<xiuxian_wendao::pybindings::PyKnowledgeStorage>();
    let _ = std::mem::size_of::<xiuxian_wendao::pybindings::PySyncEngine>();
    let _ = std::mem::size_of::<xiuxian_wendao::pybindings::PyUnifiedSymbolIndex>();
}
