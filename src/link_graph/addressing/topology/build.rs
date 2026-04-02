use std::collections::HashMap;

use crate::link_graph::PageIndexNode;
use crate::link_graph::addressing::topology::{MatchType, PathEntry, PathMatch, TopologyIndex};

impl TopologyIndex {
    /// Build a topology index from page index trees.
    #[must_use]
    pub fn build_from_trees(trees: &HashMap<String, Vec<PageIndexNode>>) -> Self {
        let mut by_doc: HashMap<String, Vec<PathEntry>> = HashMap::new();
        let mut title_index: HashMap<String, Vec<PathMatch>> = HashMap::new();
        let mut hash_index: HashMap<String, PathEntry> = HashMap::new();

        for (doc_id, nodes) in trees {
            let mut entries = Vec::new();
            Self::collect_entries(
                nodes,
                doc_id,
                &mut entries,
                &mut title_index,
                &mut hash_index,
            );
            by_doc.insert(doc_id.clone(), entries);
        }

        Self {
            by_doc,
            title_index,
            hash_index,
        }
    }

    /// Recursively collect path entries from nodes.
    fn collect_entries(
        nodes: &[PageIndexNode],
        doc_id: &str,
        entries: &mut Vec<PathEntry>,
        title_index: &mut HashMap<String, Vec<PathMatch>>,
        hash_index: &mut HashMap<String, PathEntry>,
    ) {
        for node in nodes {
            let entry = PathEntry {
                path: node.metadata.structural_path.clone(),
                node_id: node.node_id.clone(),
                title: node.title.clone(),
                level: node.level,
                doc_id: doc_id.to_string(),
                content_hash: node.metadata.content_hash.clone(),
            };

            // Add to document entries
            entries.push(entry.clone());

            // Index by lowercase title for fuzzy search
            let title_lower = node.title.to_lowercase();
            let path_match = PathMatch {
                doc_id: doc_id.to_string(),
                path: node.metadata.structural_path.clone(),
                similarity_score: 1.0,
                entry: entry.clone(),
                match_type: MatchType::Exact,
            };
            title_index.entry(title_lower).or_default().push(path_match);

            // Index by content hash for self-healing
            if let Some(ref hash) = node.metadata.content_hash {
                hash_index.insert(hash.clone(), entry);
            }

            // Recurse into children
            Self::collect_entries(&node.children, doc_id, entries, title_index, hash_index);
        }
    }
}
