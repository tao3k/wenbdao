//! Shared search primitives for Wendao.

/// Shared lexical fuzzy-search utilities.
pub mod fuzzy;
/// Shared Tantivy-backed search primitives.
pub mod tantivy;

pub use fuzzy::{
    FuzzyMatch, FuzzyMatcher, FuzzyScore, FuzzySearchOptions, LexicalMatcher, edit_distance,
    levenshtein_distance, normalized_score, passes_prefix_requirement, shared_prefix_len,
};
pub use tantivy::{
    SearchDocument, SearchDocumentFields, SearchDocumentHit, SearchDocumentIndex,
    SearchDocumentMatchField, TantivyDocumentMatch, TantivyMatcher,
};
