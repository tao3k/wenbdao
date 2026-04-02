//! Docs governance module for semantic checking.
//!
//! This module provides document governance validation for package-local
//! crate documentation, ensuring proper identity protocols, index structure,
//! and relation tracking.

pub mod collection;
pub mod parsing;
pub mod rendering;
mod scope;
pub mod types;

pub use collection::{collect_doc_governance_issues, collect_workspace_doc_governance_issues};
pub use parsing::is_package_local_crate_doc;
pub use types::{
    DOC_IDENTITY_PROTOCOL_ISSUE_TYPE, INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE, MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE, MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE,
    STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE,
    STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE,
};

#[cfg(test)]
mod tests;
