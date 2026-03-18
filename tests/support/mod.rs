//! Test support module for xiuxian-wendao.
//!
//! Provides wendao-specific scenario runners for the unified test framework.

pub mod runners;

pub use runners::{GraphRunner, PageIndexRunner, SearchRunner, SemanticCheckRunner};
