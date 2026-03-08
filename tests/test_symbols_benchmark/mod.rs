//! Benchmark tests for symbols extraction performance.
//!
//! These tests measure the performance of symbol extraction from Rust and Python
//! source files. They are designed to be run with `cargo test` and validate
//! that symbol extraction completes within acceptable time limits.

mod mixed_symbol_extraction_performance;
mod python_symbol_extraction_performance;
mod rust_symbol_extraction_performance;
mod support;
mod symbol_index_search_performance;
