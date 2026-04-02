use serde::{Deserialize, Serialize};

/// Canonical corpus partitions in the Studio search plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCorpusKind {
    /// Knowledge sections derived from the link graph.
    KnowledgeSection,
    /// Attachments associated with indexed knowledge documents.
    Attachment,
    /// Local workspace symbols derived from AST extraction.
    LocalSymbol,
    /// Reference occurrences materialized from source scanning.
    ReferenceOccurrence,
    /// Repository intelligence entities such as modules, symbols, and examples.
    RepoEntity,
    /// Repository content chunks used for fallback code search.
    RepoContentChunk,
}

impl SearchCorpusKind {
    /// Stable iteration order for all supported corpora.
    pub const ALL: [Self; 6] = [
        Self::KnowledgeSection,
        Self::Attachment,
        Self::LocalSymbol,
        Self::ReferenceOccurrence,
        Self::RepoEntity,
        Self::RepoContentChunk,
    ];

    /// Canonical storage and API identifier for the corpus.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KnowledgeSection => "knowledge_section",
            Self::Attachment => "attachment",
            Self::LocalSymbol => "local_symbol",
            Self::ReferenceOccurrence => "reference_occurrence",
            Self::RepoEntity => "repo_entity",
            Self::RepoContentChunk => "repo_content_chunk",
        }
    }

    /// Current schema version for the corpus table layout.
    #[must_use]
    pub const fn schema_version(self) -> u32 {
        match self {
            Self::LocalSymbol => 3,
            Self::KnowledgeSection
            | Self::Attachment
            | Self::ReferenceOccurrence
            | Self::RepoEntity
            | Self::RepoContentChunk => 1,
        }
    }

    /// Whether the corpus still needs legacy Lance secondary indices on the published table.
    ///
    /// The current search-plane read path is fully DataFusion/Parquet backed for all corpora,
    /// so these legacy Lance indices are now dead write-time cost and should stay disabled.
    #[must_use]
    pub const fn requires_legacy_lance_indices(self) -> bool {
        false
    }

    /// Whether the corpus is repo-backed instead of local-epoch backed.
    #[must_use]
    pub const fn is_repo_backed(self) -> bool {
        matches!(self, Self::RepoEntity | Self::RepoContentChunk)
    }
}

impl std::fmt::Display for SearchCorpusKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str((*self).as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::SearchCorpusKind;

    #[test]
    fn datafusion_cutover_corpora_no_longer_require_legacy_lance_indices() {
        for corpus in SearchCorpusKind::ALL {
            assert!(
                !corpus.requires_legacy_lance_indices(),
                "{corpus} should not build legacy Lance indices after the DataFusion cutover"
            );
        }
    }
}
