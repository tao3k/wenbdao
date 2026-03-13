//! `PyO3` bindings for `KnowledgeStorage` (Valkey operations).

use pyo3::prelude::*;
use tokio::runtime::Runtime;

use crate::knowledge_py::PyKnowledgeEntry;
use crate::storage::KnowledgeStorage;

/// Knowledge storage Python wrapper.
#[pyclass]
#[derive(Debug)]
pub struct PyKnowledgeStorage {
    inner: KnowledgeStorage,
    runtime: Runtime,
}

#[pymethods]
impl PyKnowledgeStorage {
    /// Create a new storage instance.
    #[new]
    #[pyo3(signature = (path, table_name))]
    pub fn new(path: &str, table_name: &str) -> PyResult<Self> {
        let storage = KnowledgeStorage::new(path, table_name);
        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(Self {
            inner: storage,
            runtime,
        })
    }

    /// Add an entry to storage.
    pub fn add_entry(&self, entry: PyKnowledgeEntry) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner
                .upsert(&entry.inner)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }

    /// Get an entry by ID.
    pub fn get_entry(&self, entry_id: &str) -> PyResult<Option<PyKnowledgeEntry>> {
        self.runtime.block_on(async {
            let entry = self
                .inner
                .get_entry(entry_id)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(entry.map(|e| PyKnowledgeEntry { inner: e }))
        })
    }

    /// Search entries by text.
    pub fn text_search(&self, query: &str, limit: i32) -> PyResult<Vec<PyKnowledgeEntry>> {
        self.runtime.block_on(async {
            let entries =
                self.inner.search_text(query, limit).await.map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                })?;
            Ok(entries
                .into_iter()
                .map(|e| PyKnowledgeEntry { inner: e })
                .collect())
        })
    }

    /// Search entries by vector similarity.
    pub fn vector_search(
        &self,
        query_vector: Vec<f32>,
        limit: i32,
    ) -> PyResult<Vec<PyKnowledgeEntry>> {
        self.runtime.block_on(async {
            let entries =
                self.inner.search(&query_vector, limit).await.map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                })?;
            Ok(entries
                .into_iter()
                .map(|e| PyKnowledgeEntry { inner: e })
                .collect())
        })
    }

    /// Hybrid search (text + vector).
    pub fn search(&self, query: &str, limit: i32) -> PyResult<Vec<PyKnowledgeEntry>> {
        self.runtime.block_on(async {
            let entries =
                self.inner.search_text(query, limit).await.map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                })?;
            Ok(entries
                .into_iter()
                .map(|e| PyKnowledgeEntry { inner: e })
                .collect())
        })
    }
}
