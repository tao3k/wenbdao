//! Shared types for docs governance.

/// Issue type for document identity protocol violations.
pub const DOC_IDENTITY_PROTOCOL_ISSUE_TYPE: &str = "DOC_IDENTITY_PROTOCOL";

/// Issue type for missing package docs index.
pub const MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE: &str = "MISSING_PACKAGE_DOCS_INDEX";

/// Issue type for missing package docs index footer block.
pub const MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE: &str =
    "MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK";

/// Issue type for incomplete package docs index footer block.
pub const INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE: &str =
    "INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK";

/// Issue type for stale package docs index footer standards.
pub const STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE: &str =
    "STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS";

/// Issue type for missing package docs index relations block.
pub const MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE: &str =
    "MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK";

/// Issue type for missing package docs index relation link.
pub const MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE: &str =
    "MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK";

/// Issue type for stale package docs index relation link.
pub const STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE: &str =
    "STALE_PACKAGE_DOCS_INDEX_RELATION_LINK";

/// Issue type for missing package docs index section link.
pub const MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE: &str =
    "MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK";

/// Issue type for missing package docs section landing page.
pub const MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE: &str =
    "MISSING_PACKAGE_DOCS_SECTION_LANDING";

/// Issue type for missing package docs tree.
pub const MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE: &str = "MISSING_PACKAGE_DOCS_TREE";

/// A slice of a line in a document.
#[derive(Debug, Clone, Copy)]
pub struct LineSlice<'a> {
    /// 1-based source line number.
    pub line_number: usize,
    /// Byte offset where this line starts.
    pub start_offset: usize,
    /// Byte offset where this line ends.
    pub end_offset: usize,
    /// Trimmed line contents without surrounding whitespace.
    pub trimmed: &'a str,
    /// Original line contents without trailing newline bytes.
    pub without_newline: &'a str,
    /// Trailing newline sequence captured for this line.
    pub newline: &'a str,
}

/// Metadata about a documentation section.
#[derive(Debug, Clone)]
pub struct SectionSpec {
    /// Canonical section directory name.
    pub section_name: &'static str,
    /// Relative markdown path for the section landing page.
    pub relative_path: String,
    /// Human-readable section title.
    pub title: String,
    /// Stable docs taxonomy label written into the generated page.
    pub doc_type: &'static str,
}

/// Parsed top properties drawer.
#[derive(Debug, Clone, Copy)]
pub struct TopPropertiesDrawer<'a> {
    /// 1-based line number where the drawer starts.
    pub properties_line: usize,
    /// Byte offset where a missing `:ID:` line should be inserted.
    pub insert_offset: usize,
    /// Newline sequence used by the surrounding document.
    pub newline: &'a str,
    /// Parsed `:ID:` line when one is already present.
    pub id_line: Option<IdLine<'a>>,
}

/// Parsed :ID: line in a properties drawer.
#[derive(Debug, Clone, Copy)]
pub struct IdLine<'a> {
    /// 1-based source line number.
    pub line: usize,
    /// Parsed `:ID:` value.
    pub value: &'a str,
    /// Byte offset where the value starts.
    pub value_start: usize,
    /// Byte offset where the value ends.
    pub value_end: usize,
}

/// Parsed :LINKS: line in a relations block.
#[derive(Debug, Clone, Copy)]
pub struct LinksLine<'a> {
    /// 1-based source line number.
    pub line: usize,
    /// Raw `:LINKS:` payload.
    pub value: &'a str,
    /// Byte offset where the payload starts.
    pub value_start: usize,
    /// Byte offset where the payload ends.
    pub value_end: usize,
}

/// Parsed :FOOTER: block.
#[derive(Debug, Clone, Copy)]
pub struct FooterBlock<'a> {
    /// 1-based source line number where the footer starts.
    pub line: usize,
    /// Byte offset where the footer block starts.
    pub start_offset: usize,
    /// Byte offset where the footer block ends.
    pub end_offset: usize,
    /// Parsed `:STANDARDS:` value, when present.
    pub standards_value: Option<&'a str>,
    /// Parsed `:LAST_SYNC:` value, when present.
    pub last_sync_value: Option<&'a str>,
}
