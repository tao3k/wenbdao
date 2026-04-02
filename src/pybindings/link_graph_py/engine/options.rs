use super::PyLinkGraphEngine;
use pyo3::PyResult;

use crate::link_graph::LinkGraphSearchOptions;

impl PyLinkGraphEngine {
    pub(super) fn parse_search_options(
        options_json: Option<&str>,
    ) -> PyResult<LinkGraphSearchOptions> {
        let Some(raw) = options_json
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(LinkGraphSearchOptions::default());
        };
        let options = serde_json::from_str::<LinkGraphSearchOptions>(raw)
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))?;
        options
            .validate()
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(options)
    }

    pub(super) fn run_search_planned_impl(
        &self,
        query: &str,
        limit: usize,
        options_json: Option<&str>,
    ) -> PyResult<String> {
        let options = Self::parse_search_options(options_json)?;
        let payload = self
            .inner
            .search_planned_payload(query, limit.max(1), options);
        serde_json::to_string(&payload)
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
    }
}
