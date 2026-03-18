//! Fuzzy path matching for human discovery.
//!
//! The `TopologyIndex` provides structural path indexing and fuzzy matching capabilities
//! for discovering nodes when the exact path or title is not known.

use std::collections::HashMap;

use crate::link_graph::PageIndexNode;

/// A path entry representing a node's position in the document structure.
#[derive(Debug, Clone)]
pub struct PathEntry {
    /// Full structural path (e.g., `["Architecture", "Storage", "Configuration"]`).
    pub path: Vec<String>,
    /// Stable node ID for anchoring after discovery.
    pub node_id: String,
    /// Display title (may differ from path if title was renamed).
    pub title: String,
    /// Heading level (1-6).
    pub level: usize,
    /// Document ID containing this node.
    pub doc_id: String,
    /// Content hash for self-healing lookups.
    pub content_hash: Option<String>,
}

/// A match result from fuzzy path resolution.
#[derive(Debug, Clone)]
pub struct PathMatch {
    /// Document ID containing the match.
    pub doc_id: String,
    /// Full structural path to the matched node.
    pub path: Vec<String>,
    /// Similarity score (0.0-1.0, where 1.0 is exact match).
    pub similarity_score: f32,
    /// The matched path entry.
    pub entry: PathEntry,
    /// How the match was found.
    pub match_type: MatchType,
}

/// Classification of how a path match was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    /// Exact path match.
    Exact,
    /// Path suffix match (partial path from the end).
    Suffix,
    /// Title substring match.
    TitleSubstring,
    /// Content hash match (self-healing).
    HashFallback,
    /// Case-insensitive match.
    CaseInsensitive,
}

/// Fuzzy path matching index for human discovery.
///
/// This index enables discovery of nodes when the exact path is not known,
/// supporting partial matches, title searches, and content hash fallbacks.
///
/// # Example
///
/// ```ignore
/// let topology = TopologyIndex::build_from_graph(&link_graph);
///
/// // Exact path lookup
/// if let Some(entry) = topology.exact_path("doc.md", &["Architecture", "Storage"]) {
///     println!("Found: {}", entry.title);
/// }
///
/// // Fuzzy discovery
/// let matches = topology.fuzzy_resolve("storage", 5);
/// for m in matches {
///     println!("{} (score: {})", m.path.join("/"), m.similarity_score);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct TopologyIndex {
    /// Structural paths for each document: `doc_id` → path entries.
    by_doc: HashMap<String, Vec<PathEntry>>,
    /// Lowercase title → possible matches (for fuzzy search).
    title_index: HashMap<String, Vec<PathMatch>>,
    /// Content hash → path entries (for self-healing).
    hash_index: HashMap<String, PathEntry>,
}

impl TopologyIndex {
    /// Create an empty topology index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_doc: HashMap::new(),
            title_index: HashMap::new(),
            hash_index: HashMap::new(),
        }
    }

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

    /// Find a node by exact structural path within a document.
    #[must_use]
    pub fn exact_path(&self, doc_id: &str, components: &[String]) -> Option<&PathEntry> {
        let entries = self.by_doc.get(doc_id)?;
        entries.iter().find(|e| e.path.as_slice() == components)
    }

    /// Find a node by exact or case-insensitive path.
    #[must_use]
    pub fn path_case_insensitive(&self, doc_id: &str, components: &[String]) -> Option<PathMatch> {
        // Try exact match first
        if let Some(entry) = self.exact_path(doc_id, components) {
            return Some(PathMatch {
                doc_id: doc_id.to_string(),
                path: entry.path.clone(),
                similarity_score: 1.0,
                entry: entry.clone(),
                match_type: MatchType::Exact,
            });
        }

        // Try case-insensitive match
        let entries = self.by_doc.get(doc_id)?;
        let lower_components: Vec<String> = components.iter().map(|c| c.to_lowercase()).collect();

        for entry in entries {
            let entry_lower: Vec<String> = entry.path.iter().map(|p| p.to_lowercase()).collect();
            if entry_lower == lower_components {
                return Some(PathMatch {
                    doc_id: doc_id.to_string(),
                    path: entry.path.clone(),
                    similarity_score: 0.95,
                    entry: entry.clone(),
                    match_type: MatchType::CaseInsensitive,
                });
            }
        }

        None
    }

    /// Find a node by content hash (self-healing).
    #[must_use]
    pub fn find_by_hash(&self, hash: &str) -> Option<&PathEntry> {
        self.hash_index.get(hash)
    }

    /// Fuzzy path matching with path drift tolerance.
    ///
    /// Returns matches sorted by similarity score (highest first).
    #[must_use]
    pub fn fuzzy_resolve(&self, query: &str, max_results: usize) -> Vec<PathMatch> {
        let query_lower = query.to_lowercase();
        let mut matches: Vec<PathMatch> = Vec::new();

        // 1. Exact title match
        if let Some(exact_matches) = self.title_index.get(&query_lower) {
            for m in exact_matches {
                let mut scored = m.clone();
                scored.similarity_score = 1.0;
                scored.match_type = MatchType::Exact;
                matches.push(scored);
            }
        }

        // 2. Partial path match (suffix)
        for entries in self.by_doc.values() {
            for entry in entries {
                // Check if query matches end of path
                let path_lower: Vec<String> = entry.path.iter().map(|p| p.to_lowercase()).collect();
                if path_match_suffix(&path_lower, &query_lower) {
                    let m = PathMatch {
                        doc_id: entry.doc_id.clone(),
                        path: entry.path.clone(),
                        similarity_score: 0.85,
                        entry: entry.clone(),
                        match_type: MatchType::Suffix,
                    };
                    if !matches.iter().any(|existing: &PathMatch| {
                        existing.entry.node_id == entry.node_id && existing.doc_id == entry.doc_id
                    }) {
                        matches.push(m);
                    }
                }
            }
        }

        // 3. Title substring match
        for (title, title_matches) in &self.title_index {
            if title.contains(&query_lower) && title != &query_lower {
                for m in title_matches {
                    let mut scored = m.clone();
                    // Score based on how much of the title the query covers
                    scored.similarity_score =
                        0.7 + similarity_ratio(query_lower.len(), title.len()).min(0.25);
                    scored.match_type = MatchType::TitleSubstring;

                    if !matches.iter().any(|existing: &PathMatch| {
                        existing.entry.node_id == m.entry.node_id && existing.doc_id == m.doc_id
                    }) {
                        matches.push(scored);
                    }
                }
            }
        }

        // Sort by similarity score (descending)
        matches.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        matches.truncate(max_results);
        matches
    }

    /// Get all path entries for a document.
    #[must_use]
    pub fn entries_for_doc(&self, doc_id: &str) -> Option<&Vec<PathEntry>> {
        self.by_doc.get(doc_id)
    }

    /// Get the total number of indexed entries across all documents.
    #[must_use]
    pub fn total_entries(&self) -> usize {
        self.by_doc.values().map(std::vec::Vec::len).sum()
    }

    /// Get all document IDs in the index.
    #[must_use]
    pub fn doc_ids(&self) -> Vec<&str> {
        self.by_doc
            .keys()
            .map(std::string::String::as_str)
            .collect()
    }

    /// Find a path entry by its `node_id` (Blueprint Section 2.2 skeleton validation).
    ///
    /// This is used for skeleton re-ranking to validate vector search results
    /// against the current AST structure.
    #[must_use]
    pub fn find_by_node_id(&self, node_id: &str) -> Option<&PathEntry> {
        self.hash_index
            .values()
            .find(|entry| entry.node_id == node_id)
    }
}

/// Check if the query matches the end of a path.
pub(super) fn path_match_suffix(path_lower: &[String], query_lower: &str) -> bool {
    // Try matching query against path suffixes
    let query_parts: Vec<&str> = query_lower.split('/').filter(|s| !s.is_empty()).collect();

    if query_parts.is_empty() {
        return false;
    }

    // Check if path ends with query parts
    if query_parts.len() > path_lower.len() {
        return false;
    }

    let suffix_start = path_lower.len() - query_parts.len();
    for (i, query_part) in query_parts.iter().enumerate() {
        if &path_lower[suffix_start + i] != query_part {
            return false;
        }
    }

    true
}

fn similarity_ratio(left: usize, right: usize) -> f32 {
    f32::from(u16::try_from(left).unwrap_or(u16::MAX))
        / f32::from(u16::try_from(right.max(1)).unwrap_or(u16::MAX))
}

#[cfg(test)]
#[path = "../../../tests/unit/link_graph/addressing/topology.rs"]
mod tests;
