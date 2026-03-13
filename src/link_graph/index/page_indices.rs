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
        self.get_tree(doc_id).map(Vec::as_slice)
    }

    /// Render the canonical traceability label for one anchor id.
    #[must_use]
    pub fn page_index_trace_label(&self, anchor_id: &str) -> Option<String> {
        self.extract_lineage(anchor_id)
            .map(|path| format!("[Path: {}]", path.join(" > ")))
    }

    /// Lookup the parent node id for a page-index node.
    ///
    /// Returns `Some(None)` for roots, `Some(Some(parent_id))` for child nodes,
    /// and `None` when the node id is unknown.
    #[must_use]
    pub fn page_index_parent_id(&self, node_id: &str) -> Option<Option<&str>> {
        self.node_parent_map
            .get(node_id)
            .map(|parent| parent.as_deref())
    }

    #[allow(dead_code)]
    pub(super) fn rebuild_all_page_indices(&mut self) {
        self.trees_by_doc.clear();
        self.node_parent_map.clear();
        let doc_ids = self.docs_by_id.keys().cloned().collect::<Vec<_>>();
        for doc_id in doc_ids {
            self.rebuild_page_index_for_doc(&doc_id);
        }
    }

    pub(in crate::link_graph::index) fn rebuild_page_index_for_doc(&mut self, doc_id: &str) {
        let Some(doc_title) = self.docs_by_id.get(doc_id).map(|doc| doc.title.clone()) else {
            self.remove_page_index_for_doc(doc_id);
            return;
        };
        let Some(sections) = self.sections_by_doc.get(doc_id).cloned() else {
            self.remove_page_index_for_doc(doc_id);
            return;
        };

        self.remove_page_index_for_doc(doc_id);
        let mut tree = build_page_index_tree(doc_id, &doc_title, &sections);
        thin_page_index_tree(&mut tree, DEFAULT_PAGE_INDEX_THINNING_TOKEN_THRESHOLD);
        if tree.is_empty() {
            self.remove_page_index_for_doc(doc_id);
        } else {
            self.index_page_index_nodes(&tree, None);
            self.trees_by_doc.insert(doc_id.to_string(), tree);
        }
    }

    pub(in crate::link_graph::index) fn remove_page_index_for_doc(&mut self, doc_id: &str) {
        self.trees_by_doc.remove(doc_id);
        let prefix = format!("{doc_id}#");
        self.node_parent_map
            .retain(|node_id, _| !node_id.starts_with(&prefix));
    }

    fn index_page_index_nodes(&mut self, nodes: &[PageIndexNode], parent_id: Option<&str>) {
        for node in nodes {
            self.node_parent_map
                .insert(node.node_id.clone(), parent_id.map(str::to_string));
            self.index_page_index_nodes(&node.children, Some(node.node_id.as_str()));
        }
    }
}
