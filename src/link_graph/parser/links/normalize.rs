use crate::link_graph::index::LinkGraphIndex;
use crate::link_graph::models::PageIndexNode;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Hierarchical hit record for stable lineage tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalHit {
    /// Resolved anchor id.
    pub anchor_id: String,
    /// Resolved doc id.
    pub doc_id: String,
    /// Relative path.
    pub path: String,
    /// Semantic lineage path (headings).
    pub semantic_path: Vec<String>,
}

impl LinkGraphIndex {
    /// Resolve one anchor into a stable hierarchical trace record.
    #[must_use]
    pub fn hierarchical_hit(&self, anchor_id: &str) -> Option<HierarchicalHit> {
        let anchor_id = self.canonical_anchor_id(anchor_id)?;
        let doc_id = canonical_doc_id(anchor_id.as_str()).to_string();
        let doc = self.get_doc(doc_id.as_str())?;
        let semantic_path = self.extract_lineage(anchor_id.as_str())?;

        Some(HierarchicalHit {
            anchor_id,
            doc_id,
            path: doc.path.clone(),
            semantic_path,
        })
    }

    pub(crate) fn extract_lineage(&self, anchor_id: &str) -> Option<Vec<String>> {
        let anchor_id = self.canonical_anchor_id(anchor_id)?;
        let doc_id = canonical_doc_id(anchor_id.as_str());

        if anchor_id == doc_id {
            return self.root_lineage(doc_id);
        }

        let roots = self.get_tree(doc_id)?;
        let node_ids = collect_lineage_node_ids(self.get_node_parent_map(), anchor_id.as_str())?;
        node_ids
            .into_iter()
            .map(|node_id| find_node_title(roots, node_id.as_str()))
            .collect()
    }

    fn canonical_anchor_id(&self, anchor_id: &str) -> Option<String> {
        let trimmed = anchor_id.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some((doc_ref, suffix)) = trimmed.split_once('#') {
            let doc_id = if self.has_doc(doc_ref) {
                doc_ref.to_string()
            } else {
                self.resolve_doc_id_internal(doc_ref)?.to_string()
            };
            let suffix = suffix.trim_matches(|ch: char| ch == '#' || ch.is_whitespace());
            return if suffix.is_empty() {
                Some(doc_id)
            } else {
                Some(format!("{doc_id}#{suffix}"))
            };
        }

        if self.has_doc(trimmed) {
            return Some(trimmed.to_string());
        }

        self.resolve_doc_id_internal(trimmed).map(str::to_string)
    }

    fn root_lineage(&self, doc_id: &str) -> Option<Vec<String>> {
        if let Some(roots) = self.get_tree(doc_id)
            && let Some(root) = roots.first()
        {
            return Some(vec![root.title.clone()]);
        }

        self.get_doc(doc_id).map(|doc| vec![doc.title.clone()])
    }

    /// Internal helper for anchor resolution.
    fn resolve_doc_id_internal(&self, stem_or_id: &str) -> Option<&str> {
        // This effectively delegates to the private resolve_doc_id
        // which we can't call directly but we can implement the same logic
        // or just use a helper if we had one.
        // For now, let's assume we can add a pub(crate) version.
        self.resolve_doc_id_pub(stem_or_id)
    }
}

fn canonical_doc_id(anchor_id: &str) -> &str {
    anchor_id
        .split_once('#')
        .map_or(anchor_id, |(doc_id, _)| doc_id)
}

fn collect_lineage_node_ids(
    node_parent_map: &HashMap<String, Option<String>>,
    target_node_id: &str,
) -> Option<Vec<String>> {
    let mut node_ids = Vec::new();
    let mut current = Some(target_node_id.to_string());
    let mut visited = HashSet::new();

    while let Some(node_id) = current {
        if !visited.insert(node_id.clone()) {
            return None;
        }
        let parent_id = node_parent_map.get(node_id.as_str())?.clone();
        node_ids.push(node_id);
        current = parent_id;
    }

    node_ids.reverse();
    Some(node_ids)
}

fn find_node_title(nodes: &[PageIndexNode], target_node_id: &str) -> Option<String> {
    for node in nodes {
        if node.node_id == target_node_id {
            return Some(node.title.clone());
        }
        if let Some(title) = find_node_title(&node.children, target_node_id) {
            return Some(title);
        }
    }
    None
}

pub(super) fn strip_target_decorations(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(
        trimmed
            .trim_matches(|c: char| c == '[' || c == ']' || c == '(' || c == ')')
            .to_string(),
    )
}

pub(super) fn has_external_scheme(lower: &str) -> bool {
    lower.starts_with("http:") || lower.starts_with("https:") || lower.contains("://")
}

pub(super) fn strip_fragment_and_query(raw: &str) -> &str {
    raw.split_once('#')
        .map_or(raw, |(base, _)| base)
        .split_once('?')
        .map_or(raw, |(base, _)| base)
}

pub(super) fn has_supported_note_extension(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
}

pub(super) fn normalize_markdown_note_target(
    target: &str,
    _source_path: &std::path::Path,
    _root: &std::path::Path,
) -> Option<String> {
    let stem = target.split_once('.').map_or(target, |(s, _)| s);
    (!stem.is_empty()).then(|| crate::link_graph::parser::normalize_alias(stem))
}

pub(super) fn normalize_attachment_target(
    target: &str,
    _source_path: &std::path::Path,
    _root: &std::path::Path,
) -> Option<String> {
    (!target.is_empty()).then(|| target.to_string())
}

pub(super) fn normalize_wikilink_note_target(raw: &str) -> Option<String> {
    let stem = raw.split_once('|').map_or(raw, |(s, _)| s);
    let stem = stem.split_once('#').map_or(stem, |(s, _)| s);
    (!stem.is_empty()).then(|| crate::link_graph::parser::normalize_alias(stem))
}
