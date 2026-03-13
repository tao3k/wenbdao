//! Test support module for xiuxian-wendao.
//!
//! This module provides wendao-specific test utilities.

pub mod runners;

// Re-export wendao-specific runners
pub use runners::{GraphRunner, PageIndexRunner, SearchRunner};
