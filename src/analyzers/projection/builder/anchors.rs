#[derive(Debug, Clone, Default)]
pub(super) struct TargetAnchors {
    pub(super) module_ids: Vec<String>,
    pub(super) symbol_ids: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct SourceAssociations {
    pub(super) doc_ids: Vec<String>,
    pub(super) example_ids: Vec<String>,
    pub(super) doc_paths: Vec<String>,
    pub(super) example_paths: Vec<String>,
    pub(super) format_hints: Vec<String>,
}

pub(super) fn attach_target(targets: &mut TargetAnchors, target_id: &str) {
    if target_id.contains(":module:") {
        push_unique(&mut targets.module_ids, target_id.to_string());
    } else if target_id.contains(":symbol:") {
        push_unique(&mut targets.symbol_ids, target_id.to_string());
    }
}

use super::helpers::push_unique;
