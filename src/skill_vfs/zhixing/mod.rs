//! Zhixing domain specific indexing and address constants.

/// Metadata attribute for journal carryover state.
pub const ATTR_JOURNAL_CARRYOVER: &str = "journal:carryover";
/// Metadata attribute for scheduled timer timestamp.
pub const ATTR_TIMER_SCHEDULED: &str = "timer:scheduled";
/// Metadata attribute for reminded timer state.
pub const ATTR_TIMER_REMINDED: &str = "timer:reminded";

mod indexer;
mod resources;

pub use indexer::{ZhixingIndexSummary, ZhixingWendaoIndexer};

/// Specialized error type for Zhixing domain indexing.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Internal indexing failure.
    #[error("{0}")]
    Internal(String),
}

/// Result type for Zhixing domain indexing.
pub type Result<T> = std::result::Result<T, Error>;

pub use resources::{
    ZHIXING_SKILL_DOC_PATH, build_embedded_wendao_registry, embedded_discover_canonical_uris,
    embedded_resource_text, embedded_resource_text_from_wendao_uri, embedded_skill_links_for_id,
    embedded_skill_links_for_reference_type, embedded_skill_links_index, embedded_skill_markdown,
};

pub(crate) use resources::{
    ZHIXING_EMBEDDED_CRATE_ID, embedded_resource_dir, embedded_semantic_reference_mounts,
};
