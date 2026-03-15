use super::attachments::attachments_for_parsed_note;
use super::cluster_finder::find_dense_clusters;
use super::collapse::collapse_clusters;
use super::constants::DEFAULT_EXCLUDED_DIR_NAMES;
use super::filters::{merge_excluded_dirs, normalize_include_dir, should_skip_entry};
use super::graphmem::sync_graphmem_state_best_effort;
use super::saliency_snapshot::SaliencySnapshot;
use crate::link_graph::index::{IndexedSection, LinkGraphIndex, doc_sort_key};
use crate::link_graph::models::{LinkGraphAttachment, LinkGraphDocument};
use crate::link_graph::parser::{ParsedNote, is_supported_note, normalize_alias, parse_note};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

impl LinkGraphIndex {
    /// Build index with excluded directory names (e.g. ".cache", ".git").
    pub fn build_with_excluded_dirs(
        root_dir: &Path,
        excluded_dirs: &[String],
    ) -> Result<Self, String> {
        let index = Self::build_with_filters(root_dir, &[], excluded_dirs)?;
        sync_graphmem_state_best_effort(&index);
        Ok(index)
    }

    /// Build index with include/exclude directory filters relative to notebook root.
    pub fn build_with_filters(
        root_dir: &Path,
        include_dirs: &[String],
        excluded_dirs: &[String],
    ) -> Result<Self, String> {
        let root = root_dir
            .canonicalize()
            .map_err(|e| format!("invalid notebook root '{}': {e}", root_dir.display()))?;
        if !root.is_dir() {
            return Err(format!(
                "notebook root is not a directory: {}",
                root.display()
            ));
        }

        let normalized_include_dirs: Vec<String> = include_dirs
            .iter()
            .filter_map(|path| normalize_include_dir(path))
            .collect();
        let normalized_excluded_dirs: Vec<String> =
            merge_excluded_dirs(excluded_dirs, DEFAULT_EXCLUDED_DIR_NAMES);
        let included: HashSet<String> = normalized_include_dirs.iter().cloned().collect();
        let excluded: HashSet<String> = normalized_excluded_dirs.iter().cloned().collect();

        let mut candidate_paths: Vec<PathBuf> = Vec::new();
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                !should_skip_entry(
                    entry.path(),
                    entry.file_type().is_dir(),
                    &root,
                    &included,
                    &excluded,
                )
            })
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !entry.file_type().is_file() || !is_supported_note(path) {
                continue;
            }
            candidate_paths.push(path.to_path_buf());
        }

        let mut parsed_notes: Vec<ParsedNote> = candidate_paths
            .into_par_iter()
            .filter_map(|path| {
                let content = std::fs::read_to_string(&path).ok()?;
                parse_note(&path, &root, &content)
            })
            .collect();

        parsed_notes.sort_by(|left, right| doc_sort_key(&left.doc).cmp(&doc_sort_key(&right.doc)));

        let mut docs_by_id: HashMap<String, LinkGraphDocument> = HashMap::new();
        let mut sections_by_doc: HashMap<String, Vec<IndexedSection>> = HashMap::new();
        let mut attachments_by_doc: HashMap<String, Vec<LinkGraphAttachment>> = HashMap::new();
        let mut alias_to_doc_id: HashMap<String, String> = HashMap::new();
        for parsed in &parsed_notes {
            let doc = &parsed.doc;
            docs_by_id.insert(doc.id.clone(), doc.clone());
            let indexed_sections = parsed
                .sections
                .iter()
                .map(IndexedSection::from_parsed)
                .collect::<Vec<IndexedSection>>();
            sections_by_doc.insert(doc.id.clone(), indexed_sections);
            attachments_by_doc.insert(doc.id.clone(), attachments_for_parsed_note(parsed));
            for alias in [&doc.id, &doc.path, &doc.stem] {
                let key = normalize_alias(alias);
                if key.is_empty() {
                    continue;
                }
                alias_to_doc_id.entry(key).or_insert_with(|| doc.id.clone());
            }
        }

        let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
        let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();
        let mut edge_count = 0usize;

        for parsed in &parsed_notes {
            let from_id = &parsed.doc.id;

            // Extract structural edges from wiki-links
            for raw_target in &parsed.link_targets {
                let normalized = normalize_alias(raw_target);
                if normalized.is_empty() {
                    continue;
                }
                let Some(to_id) = alias_to_doc_id.get(&normalized).cloned() else {
                    continue;
                };
                if &to_id == from_id {
                    continue;
                }
                let inserted = outgoing
                    .entry(from_id.clone())
                    .or_default()
                    .insert(to_id.clone());
                if inserted {
                    incoming.entry(to_id).or_default().insert(from_id.clone());
                    edge_count += 1;
                }
            }

            // Extract property drawer edges from section attributes (Blueprint v2.0)
            for section in &parsed.sections {
                let source_node_id = if section.heading_path.is_empty() {
                    from_id.clone()
                } else {
                    format!("{}#{}", from_id, section.heading_path.replace(" / ", "/"))
                };

                let pd_edges = super::property_drawer_edges::extract_property_drawer_edges(
                    &source_node_id,
                    &section.attributes,
                );

                for edge in pd_edges {
                    let normalized_target = normalize_alias(&edge.to);
                    let Some(to_id) = alias_to_doc_id.get(&normalized_target).cloned() else {
                        continue;
                    };

                    if to_id == *from_id {
                        continue;
                    }

                    let inserted = outgoing
                        .entry(edge.from.clone())
                        .or_default()
                        .insert(to_id.clone());
                    if inserted {
                        incoming.entry(to_id).or_default().insert(edge.from.clone());
                        edge_count += 1;
                    }
                }
            }
        }

        // === Knowledge Distillation Pipeline ===
        // Find and collapse dense clusters if saliency data is available
        let virtual_nodes = if let Some(snapshot) = Self::fetch_saliency_snapshot(&docs_by_id) {
            let saliency_map: HashMap<String, f64> = snapshot
                .states
                .iter()
                .map(|(k, v)| (k.clone(), v.current_saliency))
                .collect();

            let clusters = find_dense_clusters(
                &snapshot.high_saliency_nodes,
                &outgoing,
                &incoming,
                &saliency_map,
            );

            collapse_clusters(clusters, &docs_by_id, &mut outgoing, &mut incoming)
                .into_iter()
                .map(|vn| (vn.id.clone(), vn))
                .collect()
        } else {
            HashMap::new()
        };

        let rank_by_id = Self::compute_rank_by_id(&docs_by_id, &incoming, &outgoing);

        let mut index = Self {
            root,
            include_dirs: normalized_include_dirs,
            excluded_dirs: normalized_excluded_dirs,
            docs_by_id,
            sections_by_doc,
            passages_by_id: HashMap::new(),
        attachments_by_doc,
        trees_by_doc: HashMap::new(),
        node_parent_map: HashMap::new(),
        explicit_id_registry: HashMap::new(),
        alias_to_doc_id,
        outgoing,
            incoming,
            rank_by_id,
            edge_count,
            virtual_nodes,
        };
        index.rebuild_all_page_indices();
        Ok(index)
    }

    /// Fetch saliency snapshot from Valkey (optional, returns None if unavailable).
    fn fetch_saliency_snapshot(
        _docs_by_id: &HashMap<String, LinkGraphDocument>,
    ) -> Option<SaliencySnapshot> {
        // TODO: Wire up Valkey connection via config/env
        // For now, returns None to gracefully skip distillation
        None
    }

    /// Build index with saliency snapshot for knowledge distillation.
    ///
    /// This is the full build path that enables cluster collapse.
    pub fn build_with_saliency(
        root_dir: &Path,
        include_dirs: &[String],
        excluded_dirs: &[String],
        saliency_snapshot: Option<SaliencySnapshot>,
    ) -> Result<Self, String> {
        let root = root_dir
            .canonicalize()
            .map_err(|e| format!("invalid notebook root '{}': {e}", root_dir.display()))?;
        if !root.is_dir() {
            return Err(format!(
                "notebook root is not a directory: {}",
                root.display()
            ));
        }

        let normalized_include_dirs: Vec<String> = include_dirs
            .iter()
            .filter_map(|path| normalize_include_dir(path))
            .collect();
        let normalized_excluded_dirs: Vec<String> =
            merge_excluded_dirs(excluded_dirs, DEFAULT_EXCLUDED_DIR_NAMES);
        let included: HashSet<String> = normalized_include_dirs.iter().cloned().collect();
        let excluded: HashSet<String> = normalized_excluded_dirs.iter().cloned().collect();

        let mut candidate_paths: Vec<PathBuf> = Vec::new();
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                !should_skip_entry(
                    entry.path(),
                    entry.file_type().is_dir(),
                    &root,
                    &included,
                    &excluded,
                )
            })
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !entry.file_type().is_file() || !is_supported_note(path) {
                continue;
            }
            candidate_paths.push(path.to_path_buf());
        }

        let mut parsed_notes: Vec<ParsedNote> = candidate_paths
            .into_par_iter()
            .filter_map(|path| {
                let content = std::fs::read_to_string(&path).ok()?;
                parse_note(&path, &root, &content)
            })
            .collect();

        parsed_notes.sort_by(|left, right| doc_sort_key(&left.doc).cmp(&doc_sort_key(&right.doc)));

        let mut docs_by_id: HashMap<String, LinkGraphDocument> = HashMap::new();
        let mut sections_by_doc: HashMap<String, Vec<IndexedSection>> = HashMap::new();
        let mut attachments_by_doc: HashMap<String, Vec<LinkGraphAttachment>> = HashMap::new();
        let mut alias_to_doc_id: HashMap<String, String> = HashMap::new();
        for parsed in &parsed_notes {
            let doc = &parsed.doc;
            docs_by_id.insert(doc.id.clone(), doc.clone());
            let indexed_sections = parsed
                .sections
                .iter()
                .map(IndexedSection::from_parsed)
                .collect::<Vec<IndexedSection>>();
            sections_by_doc.insert(doc.id.clone(), indexed_sections);
            attachments_by_doc.insert(doc.id.clone(), attachments_for_parsed_note(parsed));
            for alias in [&doc.id, &doc.path, &doc.stem] {
                let key = normalize_alias(alias);
                if key.is_empty() {
                    continue;
                }
                alias_to_doc_id.entry(key).or_insert_with(|| doc.id.clone());
            }
        }

        let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
        let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();
        let mut edge_count = 0usize;

        for parsed in &parsed_notes {
            let from_id = &parsed.doc.id;

            // Extract structural edges from wiki-links
            for raw_target in &parsed.link_targets {
                let normalized = normalize_alias(raw_target);
                if normalized.is_empty() {
                    continue;
                }
                let Some(to_id) = alias_to_doc_id.get(&normalized).cloned() else {
                    continue;
                };
                if &to_id == from_id {
                    continue;
                }
                let inserted = outgoing
                    .entry(from_id.clone())
                    .or_default()
                    .insert(to_id.clone());
                if inserted {
                    incoming.entry(to_id).or_default().insert(from_id.clone());
                    edge_count += 1;
                }
            }

            // Extract property drawer edges from section attributes (Blueprint v2.0)
            for section in &parsed.sections {
                let source_node_id = if section.heading_path.is_empty() {
                    from_id.clone()
                } else {
                    format!("{}#{}", from_id, section.heading_path.replace(" / ", "/"))
                };

                let pd_edges = super::property_drawer_edges::extract_property_drawer_edges(
                    &source_node_id,
                    &section.attributes,
                );

                for edge in pd_edges {
                    let normalized_target = normalize_alias(&edge.to);
                    let Some(to_id) = alias_to_doc_id.get(&normalized_target).cloned() else {
                        continue;
                    };

                    if to_id == *from_id {
                        continue;
                    }

                    let inserted = outgoing
                        .entry(edge.from.clone())
                        .or_default()
                        .insert(to_id.clone());
                    if inserted {
                        incoming.entry(to_id).or_default().insert(edge.from.clone());
                        edge_count += 1;
                    }
                }
            }
        }

        // === Knowledge Distillation Pipeline ===
        let virtual_nodes = if let Some(snapshot) = saliency_snapshot {
            let saliency_map: HashMap<String, f64> = snapshot
                .states
                .iter()
                .map(|(k, v)| (k.clone(), v.current_saliency))
                .collect();

            let clusters = find_dense_clusters(
                &snapshot.high_saliency_nodes,
                &outgoing,
                &incoming,
                &saliency_map,
            );

            collapse_clusters(clusters, &docs_by_id, &mut outgoing, &mut incoming)
                .into_iter()
                .map(|vn| (vn.id.clone(), vn))
                .collect()
        } else {
            HashMap::new()
        };

        let rank_by_id = Self::compute_rank_by_id(&docs_by_id, &incoming, &outgoing);

        let mut index = Self {
            root,
            include_dirs: normalized_include_dirs,
            excluded_dirs: normalized_excluded_dirs,
            docs_by_id,
            sections_by_doc,
            passages_by_id: HashMap::new(),
            attachments_by_doc,
            trees_by_doc: HashMap::new(),
            node_parent_map: HashMap::new(),
            explicit_id_registry: HashMap::new(),
            alias_to_doc_id,
            outgoing,
            incoming,
            rank_by_id,
            edge_count,
            virtual_nodes,
        };
        index.rebuild_all_page_indices();
        sync_graphmem_state_best_effort(&index);
        Ok(index)
    }
}
