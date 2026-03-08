use pyo3::prelude::*;

use super::{PyUnifiedIndexStats, PyUnifiedSymbol, PyUnifiedSymbolIndex};

/// Register unified symbol module with Python.
///
/// # Errors
///
/// Returns an error if any class cannot be added to the Python module.
pub fn register_unified_symbol_module(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<PyUnifiedSymbol>()?;
    m.add_class::<PyUnifiedSymbolIndex>()?;
    m.add_class::<PyUnifiedIndexStats>()?;
    Ok(())
}
