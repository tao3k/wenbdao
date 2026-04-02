use crate::link_graph::addressing::topology::helpers::{path_match_suffix, similarity_ratio};
use crate::link_graph::addressing::topology::{MatchType, PathEntry, PathMatch, TopologyIndex};
use crate::search::{FuzzyMatcher, FuzzySearchOptions, LexicalMatcher};

impl TopologyIndex {
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
        self.fuzzy_resolve_with_options(query, max_results, FuzzySearchOptions::path_search())
    }

    /// Fuzzy path matching with explicit fuzzy options.
    ///
    /// # Panics
    ///
    /// Panics if the lexical matcher unexpectedly returns an error. The current
    /// in-memory matcher implementation is designed to be infallible.
    #[must_use]
    pub fn fuzzy_resolve_with_options(
        &self,
        query: &str,
        max_results: usize,
        options: FuzzySearchOptions,
    ) -> Vec<PathMatch> {
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

        // 4. Lexical title fuzzy fallback
        if matches.is_empty() {
            fn path_entry_title(entry: &PathEntry) -> &str {
                entry.title.as_str()
            }

            let candidates = self
                .by_doc
                .values()
                .flat_map(|entries| entries.iter().cloned())
                .collect::<Vec<_>>();
            let lexical_matcher =
                LexicalMatcher::new(candidates.as_slice(), path_entry_title, options);
            let fuzzy_matches = lexical_matcher
                .search(query, max_results)
                .expect("lexical matcher is infallible");
            for fuzzy_match in fuzzy_matches {
                let entry = fuzzy_match.item;
                matches.push(PathMatch {
                    doc_id: entry.doc_id.clone(),
                    path: entry.path.clone(),
                    similarity_score: fuzzy_match.score,
                    entry,
                    match_type: MatchType::TitleFuzzy,
                });
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
