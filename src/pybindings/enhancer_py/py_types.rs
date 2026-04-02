use pyo3::prelude::*;

use crate::enhancer::{EnhancedNote, InferredRelation, NoteFrontmatter};

/// Python wrapper for `NoteFrontmatter`.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyNoteFrontmatter {
    pub(crate) inner: NoteFrontmatter,
}

#[pymethods]
impl PyNoteFrontmatter {
    /// Document title.
    #[getter]
    fn title(&self) -> Option<String> {
        self.inner.title.clone()
    }

    /// Description.
    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// Skill name.
    #[getter]
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    /// Category.
    #[getter]
    fn category(&self) -> Option<String> {
        self.inner.category.clone()
    }

    /// Tags.
    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    /// Routing keywords.
    #[getter]
    fn routing_keywords(&self) -> Vec<String> {
        self.inner.routing_keywords.clone()
    }

    /// Intents.
    #[getter]
    fn intents(&self) -> Vec<String> {
        self.inner.intents.clone()
    }

    /// Serialize to JSON string.
    fn to_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Python wrapper for `InferredRelation`.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyInferredRelation {
    pub(crate) inner: InferredRelation,
}

#[pymethods]
impl PyInferredRelation {
    /// Source entity.
    #[getter]
    fn source(&self) -> String {
        self.inner.source.clone()
    }

    /// Target entity.
    #[getter]
    fn target(&self) -> String {
        self.inner.target.clone()
    }

    /// Relation type string.
    #[getter]
    fn relation_type(&self) -> String {
        self.inner.relation_type.clone()
    }

    /// Description.
    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }
}

/// Python wrapper for `EnhancedNote`.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyEnhancedNote {
    pub(crate) inner: EnhancedNote,
}

#[pymethods]
impl PyEnhancedNote {
    /// Note path.
    #[getter]
    fn path(&self) -> String {
        self.inner.path.clone()
    }

    /// Note title.
    #[getter]
    fn title(&self) -> String {
        self.inner.title.clone()
    }

    /// Parsed frontmatter.
    #[getter]
    fn frontmatter(&self) -> PyNoteFrontmatter {
        PyNoteFrontmatter {
            inner: self.inner.frontmatter.clone(),
        }
    }

    /// Entity references.
    #[getter]
    fn entity_refs(&self) -> Vec<(String, Option<String>)> {
        self.inner
            .entity_refs
            .iter()
            .map(|row| (row.name.clone(), row.entity_type.clone()))
            .collect()
    }

    /// Total entity references count.
    #[getter]
    fn total_refs(&self) -> usize {
        self.inner.ref_stats.total_refs
    }

    /// Unique entities count.
    #[getter]
    fn unique_entities(&self) -> usize {
        self.inner.ref_stats.unique_entities
    }

    /// Inferred relations.
    #[getter]
    fn relations(&self) -> Vec<PyInferredRelation> {
        self.inner
            .inferred_relations
            .iter()
            .map(|row| PyInferredRelation { inner: row.clone() })
            .collect()
    }

    /// Serialize full result to JSON.
    fn to_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_else(|_| "{}".to_string())
    }
}
