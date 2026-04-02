//! Section creation logic for `create_if_missing` in `semantic_edit`.
//!
//! Implements path traversal and section insertion for creating new heading
//! hierarchies in markdown documents.

pub(crate) mod building;
pub(crate) mod insertion;
pub(crate) mod types;

#[cfg(test)]
#[path = "../../../../tests/unit/zhenfa_router/native/section_create.rs"]
mod tests;

pub(crate) use building::{build_new_sections_content_with_options, compute_content_hash};
pub(crate) use insertion::find_insertion_point;
pub(crate) use types::{BuildSectionOptions, InsertionInfo};
