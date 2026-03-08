//! Lightweight query intent extractor.
//!
//! Decomposes a natural-language query into structured intent signals:
//! - **action**: the verb / desired operation (e.g. "commit", "search", "create")
//! - **target**: the object / domain the action applies to (e.g. "code", "file", "git")
//! - **context**: additional qualifiers / modifiers (e.g. "python", "async", "memory")
//! - **keywords**: all significant tokens extracted from the query

mod extract;
mod models;
mod vocabulary;

pub use extract::extract_intent;
pub use models::QueryIntent;
