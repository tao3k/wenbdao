//! Issue collection functions for docs governance.

mod footer;
mod package_docs;
mod relations;
mod workspace;

pub use footer::collect_stale_index_footer_standards;
pub use package_docs::collect_doc_governance_issues;
pub use relations::collect_stale_index_relation_links;
pub use workspace::collect_workspace_doc_governance_issues;
