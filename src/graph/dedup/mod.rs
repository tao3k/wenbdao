//! Entity deduplication, normalization, and similarity scoring.

mod operations;
mod similarity;

/// Result of deduplication operation.
#[derive(Debug, Clone, Default)]
pub struct DeduplicationResult {
    /// Number of duplicate groups found
    pub duplicate_groups_found: usize,
    /// Number of entities merged (removed)
    pub entities_merged: usize,
}
