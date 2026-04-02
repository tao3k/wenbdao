//! `PyO3` bindings for `LinkGraph` note enhancement.
//!
//! Exposes Rust-native `enhance_note` and `enhance_notes_batch` to Python.

mod py_functions;
mod py_types;

pub use py_functions::{
    link_graph_enhance_note, link_graph_enhance_notes_batch, link_graph_parse_frontmatter,
};
pub use py_types::{PyEnhancedNote, PyInferredRelation, PyNoteFrontmatter};
