//! Benchmark tests for pyproject.toml parsing performance.
//!
//! These tests measure the performance of parsing pyproject.toml files
//! for Python dependency extraction.

mod minimal_pyproject_parsing_performance;
mod mixed_pyproject_parsing_performance;
mod pyproject_extras_parsing_performance;
mod pyproject_parsing_performance;
mod regex_fallback_parsing_performance;
mod support;
