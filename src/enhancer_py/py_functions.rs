use pyo3::prelude::*;

use crate::enhancer::{self, NoteInput};

use super::py_types::{PyEnhancedNote, PyNoteFrontmatter};

/// Enhance a single note (Rust-accelerated).
///
/// Args:
///     path: Note file path.
///     title: Note title.
///     content: Full note content.
///
/// Returns:
///     `PyEnhancedNote` with frontmatter, entities, relations.
#[pyfunction]
#[pyo3(signature = (path, title, content))]
#[must_use]
pub fn link_graph_enhance_note(path: &str, title: &str, content: &str) -> PyEnhancedNote {
    let input = NoteInput {
        path: path.to_string(),
        title: title.to_string(),
        content: content.to_string(),
    };
    PyEnhancedNote {
        inner: enhancer::enhance_note(&input),
    }
}

/// Batch enhance notes (Rust-accelerated, parallelized with Rayon).
///
/// Args:
///     notes: List of (path, title, content) tuples.
///
/// Returns:
///     List of `PyEnhancedNote`.
#[pyfunction]
#[pyo3(signature = (notes))]
#[must_use]
pub fn link_graph_enhance_notes_batch(notes: Vec<(String, String, String)>) -> Vec<PyEnhancedNote> {
    let inputs: Vec<NoteInput> = notes
        .into_iter()
        .map(|(path, title, content)| NoteInput {
            path,
            title,
            content,
        })
        .collect();

    enhancer::enhance_notes_batch(&inputs)
        .into_iter()
        .map(|inner| PyEnhancedNote { inner })
        .collect()
}

/// Parse frontmatter from markdown content (Rust-accelerated).
///
/// Args:
///     content: Full markdown content.
///
/// Returns:
///     `PyNoteFrontmatter`.
#[pyfunction]
#[pyo3(signature = (content))]
#[must_use]
pub fn link_graph_parse_frontmatter(content: &str) -> PyNoteFrontmatter {
    PyNoteFrontmatter {
        inner: enhancer::parse_frontmatter(content),
    }
}
