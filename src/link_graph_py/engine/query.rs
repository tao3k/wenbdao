use super::PyLinkGraphEngine;
use pyo3::PyResult;

use crate::link_graph::LinkGraphDirection;

impl PyLinkGraphEngine {
    pub(super) fn neighbors_impl(
        &self,
        stem: &str,
        direction: LinkGraphDirection,
        hops: usize,
        limit: usize,
    ) -> PyResult<String> {
        let rows = self
            .inner
            .neighbors(stem, direction, hops.max(1), limit.max(1));
        serde_json::to_string(&rows)
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
    }

    pub(super) fn related_impl(
        &self,
        stem: &str,
        max_distance: usize,
        limit: usize,
    ) -> PyResult<String> {
        serde_json::to_string(&self.inner.related(stem, max_distance.max(1), limit.max(1)))
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
    }

    pub(super) fn metadata_impl(&self, stem: &str) -> PyResult<String> {
        serde_json::to_string(&self.inner.metadata(stem))
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
    }

    pub(super) fn toc_impl(&self, limit: usize) -> PyResult<String> {
        serde_json::to_string(&self.inner.toc(limit.max(1)))
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
    }

    pub(super) fn stats_impl(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.stats())
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
    }

    pub(super) fn cache_schema_info_impl(&self) -> PyResult<String> {
        let payload = serde_json::json!({
            "backend": self.cache_backend,
            "cache_status": self.cache_status,
            "cache_miss_reason": self.cache_miss_reason,
            "schema_version": self.cache_schema_version,
            "schema_fingerprint": self.cache_schema_fingerprint,
        });
        serde_json::to_string(&payload)
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
    }

    pub(super) fn narrate_hits_json_impl(hits_json: &str) -> PyResult<String> {
        let hits: Vec<crate::link_graph::LinkGraphHit> =
            serde_json::from_str(hits_json).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid hits JSON: {e}"))
            })?;
        Ok(crate::link_graph::narrate_subgraph(&hits))
    }
}
