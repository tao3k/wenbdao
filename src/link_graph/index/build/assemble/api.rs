use std::collections::HashMap;
use std::path::Path;

use crate::link_graph::index::LinkGraphIndex;
use crate::link_graph::index::build::assemble::edges::build_edge_tables;
use crate::link_graph::index::build::assemble::finalize::finalize_index;
use crate::link_graph::index::build::assemble::inputs::{
    collect_candidate_paths, prepare_build_inputs,
};
use crate::link_graph::index::build::assemble::notes::{build_note_tables, parse_notes};
use crate::link_graph::index::build::assemble::virtual_nodes::build_virtual_nodes;
use crate::link_graph::index::build::graphmem::sync_graphmem_state_best_effort;
use crate::link_graph::index::build::saliency_snapshot::SaliencySnapshot;
use crate::link_graph::models::LinkGraphDocument;

impl LinkGraphIndex {
    /// Build index with excluded directory names (e.g. ".cache", ".git").
    ///
    /// # Errors
    ///
    /// Returns an error when the notebook root is invalid or note parsing fails.
    pub fn build_with_excluded_dirs(
        root_dir: &Path,
        excluded_dirs: &[String],
    ) -> Result<Self, String> {
        let index = Self::build_with_filters(root_dir, &[], excluded_dirs)?;
        sync_graphmem_state_best_effort(&index);
        Ok(index)
    }

    /// Build index with include/exclude directory filters relative to notebook root.
    ///
    /// # Errors
    ///
    /// Returns an error when the notebook root is invalid or note parsing fails.
    pub fn build_with_filters(
        root_dir: &Path,
        include_dirs: &[String],
        excluded_dirs: &[String],
    ) -> Result<Self, String> {
        let inputs = prepare_build_inputs(root_dir, include_dirs, excluded_dirs)?;
        let candidate_paths = collect_candidate_paths(&inputs);
        let note_tables = build_note_tables(parse_notes(&inputs.root, candidate_paths));
        let mut edge_tables =
            build_edge_tables(&note_tables.parsed_notes, &note_tables.alias_to_doc_id);
        let virtual_nodes = build_virtual_nodes(
            &note_tables.docs_by_id,
            &mut edge_tables.outgoing,
            &mut edge_tables.incoming,
            Self::fetch_saliency_snapshot(&note_tables.docs_by_id),
        );
        Ok(finalize_index(
            inputs,
            note_tables,
            edge_tables,
            virtual_nodes,
        ))
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
    ///
    /// # Errors
    ///
    /// Returns an error when the notebook root is invalid or note parsing fails.
    pub fn build_with_saliency(
        root_dir: &Path,
        include_dirs: &[String],
        excluded_dirs: &[String],
        saliency_snapshot: Option<SaliencySnapshot>,
    ) -> Result<Self, String> {
        let inputs = prepare_build_inputs(root_dir, include_dirs, excluded_dirs)?;
        let candidate_paths = collect_candidate_paths(&inputs);
        let note_tables = build_note_tables(parse_notes(&inputs.root, candidate_paths));
        let mut edge_tables =
            build_edge_tables(&note_tables.parsed_notes, &note_tables.alias_to_doc_id);
        let virtual_nodes = build_virtual_nodes(
            &note_tables.docs_by_id,
            &mut edge_tables.outgoing,
            &mut edge_tables.incoming,
            saliency_snapshot,
        );
        let index = finalize_index(inputs, note_tables, edge_tables, virtual_nodes);
        sync_graphmem_state_best_effort(&index);
        Ok(index)
    }
}
