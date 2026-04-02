pub(super) use std::collections::HashMap;
pub(super) use std::fs;
pub(super) use std::path::Path;

pub(super) use tempfile::TempDir;
pub(super) use xiuxian_zhenfa::ZhenfaContext;

pub(super) use crate::link_graph::LinkGraphIndex;
pub(super) use crate::zhenfa_router::native::audit::fix::AtomicFixBatch;
pub(super) use crate::zhenfa_router::native::audit::generate_surgical_fixes;
pub(super) use crate::zhenfa_router::native::semantic_check::docs_governance::collect_workspace_doc_governance_issues;
pub(super) use crate::zhenfa_router::native::semantic_check::docs_governance::collection::collect_stale_index_footer_standards;
pub(super) use crate::zhenfa_router::native::semantic_check::docs_governance::parsing::derive_opaque_doc_id;
pub(super) use crate::zhenfa_router::native::semantic_check::docs_governance::{
    DOC_IDENTITY_PROTOCOL_ISSUE_TYPE, INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE, MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE, MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE,
    STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE,
    STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE, collect_doc_governance_issues,
};
pub(super) use crate::zhenfa_router::native::semantic_check::{
    CheckType, WendaoSemanticCheckArgs, run_audit_core,
};

pub(super) trait PanicExt<T> {
    fn or_panic(self, context: &str) -> T;
}

impl<T, E> PanicExt<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn or_panic(self, context: &str) -> T {
        self.unwrap_or_else(|error| panic!("{context}: {error}"))
    }
}

impl<T> PanicExt<T> for Option<T> {
    fn or_panic(self, context: &str) -> T {
        self.unwrap_or_else(|| panic!("{context}"))
    }
}
