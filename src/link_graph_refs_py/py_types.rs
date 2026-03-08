use pyo3::prelude::*;
use serde_json::to_string;

use crate::link_graph_refs::{LinkGraphEntityRef, LinkGraphRefStats};

/// Python wrapper for `LinkGraphEntityRef`.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyLinkGraphEntityRef {
    pub(crate) inner: LinkGraphEntityRef,
}

#[pymethods]
impl PyLinkGraphEntityRef {
    #[new]
    fn new(name: String, entity_type: Option<String>, original: String) -> Self {
        Self {
            inner: LinkGraphEntityRef::new(name, entity_type, original),
        }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn entity_type(&self) -> Option<String> {
        self.inner.entity_type.clone()
    }

    #[getter]
    fn original(&self) -> String {
        self.inner.original.clone()
    }

    fn to_wikilink(&self) -> String {
        self.inner.to_wikilink()
    }

    fn to_tag(&self) -> String {
        self.inner.to_tag()
    }

    fn to_dict(&self) -> String {
        let value = serde_json::json!({
            "name": self.inner.name,
            "entity_type": self.inner.entity_type,
            "original": self.inner.original,
        });
        to_string(&value).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Python wrapper for `LinkGraphRefStats`.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyLinkGraphRefStats {
    pub(crate) inner: LinkGraphRefStats,
}

#[pymethods]
impl PyLinkGraphRefStats {
    #[new]
    fn new(total_refs: usize, unique_entities: usize, by_type: Vec<(String, usize)>) -> Self {
        Self {
            inner: LinkGraphRefStats {
                total_refs,
                unique_entities,
                by_type,
            },
        }
    }

    #[getter]
    fn total_refs(&self) -> usize {
        self.inner.total_refs
    }

    #[getter]
    fn unique_entities(&self) -> usize {
        self.inner.unique_entities
    }

    #[getter]
    fn by_type(&self) -> Vec<(String, usize)> {
        self.inner.by_type.clone()
    }

    fn to_dict(&self) -> String {
        let value = serde_json::json!({
            "total_refs": self.inner.total_refs,
            "unique_entities": self.inner.unique_entities,
            "by_type": self.inner.by_type,
        });
        to_string(&value).unwrap_or_else(|_| "{}".to_string())
    }
}
