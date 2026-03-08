use super::LinkGraphIndex;
use crate::link_graph::models::PageIndexNode;
use crate::link_graph::page_index::{
    DEFAULT_PAGE_INDEX_THINNING_TOKEN_THRESHOLD, build_page_index_tree, thin_page_index_tree,
};

impl LinkGraphIndex {
    /// Return the hierarchical `PageIndex` roots for a note.
    #[must_use]
    pub fn page_index(&self, stem_or_id: &str) -> Option<&[PageIndexNode]> {
        let doc_id = self.resolve_doc_id(stem_or_id)?;
        self.trees_by_doc.get(doc_id).map(Vec::as_slice)
    }

    /// Resolve an anchor id into its complete semantic `PageIndex` path.
    #[must_use]
    pub fn page_index_semantic_path(&self, anchor_id: &str) -> Option<Vec<String>> {
        let trimmed = anchor_id.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some((doc_id, node_id)) = trimmed.split_once('#') {
            let roots = self.trees_by_doc.get(doc_id)?;
            let mut path = Vec::new();
            if find_node_path(roots, trimmed, &mut path) {
                return Some(path);
            }
            if self.docs_by_id.contains_key(doc_id) {
                return self
                    .docs_by_id
                    .get(doc_id)
                    .map(|doc| vec![doc.title.clone()]);
            }
            let _ = node_id;
            return None;
        }

        let doc_id = self.resolve_doc_id(trimmed)?;
        self.docs_by_id
            .get(doc_id)
            .map(|doc| vec![doc.title.clone()])
    }

    /// Render the canonical traceability label for one anchor id.
    #[must_use]
    pub fn page_index_trace_label(&self, anchor_id: &str) -> Option<String> {
        self.page_index_semantic_path(anchor_id)
            .map(|path| format!("[Path: {}]", path.join(" > ")))
    }

    pub(super) fn rebuild_all_page_indices(&mut self) {
        self.trees_by_doc.clear();
        let doc_ids = self.docs_by_id.keys().cloned().collect::<Vec<_>>();
        for doc_id in doc_ids {
            self.rebuild_page_index_for_doc(&doc_id);
        }
    }

    pub(super) fn rebuild_page_index_for_doc(&mut self, doc_id: &str) {
        let Some(doc) = self.docs_by_id.get(doc_id) else {
            self.trees_by_doc.remove(doc_id);
            return;
        };
        let Some(sections) = self.sections_by_doc.get(doc_id) else {
            self.trees_by_doc.remove(doc_id);
            return;
        };

        let mut tree = build_page_index_tree(doc_id, &doc.title, sections);
        thin_page_index_tree(&mut tree, DEFAULT_PAGE_INDEX_THINNING_TOKEN_THRESHOLD);
        if tree.is_empty() {
            self.trees_by_doc.remove(doc_id);
        } else {
            self.trees_by_doc.insert(doc_id.to_string(), tree);
        }
    }

    pub(super) fn remove_page_index_for_doc(&mut self, doc_id: &str) {
        self.trees_by_doc.remove(doc_id);
    }
}

fn find_node_path(nodes: &[PageIndexNode], target_node_id: &str, path: &mut Vec<String>) -> bool {
    for node in nodes {
        path.push(node.title.clone());
        if node.node_id == target_node_id {
            return true;
        }
        if find_node_path(&node.children, target_node_id, path) {
            return true;
        }
        path.pop();
    }
    false
}
