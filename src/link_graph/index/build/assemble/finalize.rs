use std::collections::HashMap;

use crate::link_graph::index::LinkGraphIndex;
use crate::link_graph::index::build::assemble::types::{BuildInputs, EdgeTables, NoteTables};

pub(crate) fn finalize_index(
    inputs: BuildInputs,
    note_tables: NoteTables,
    edge_tables: EdgeTables,
    virtual_nodes: HashMap<String, crate::link_graph::index::build::VirtualNode>,
) -> LinkGraphIndex {
    let rank_by_id = LinkGraphIndex::compute_rank_by_id(
        &note_tables.docs_by_id,
        &edge_tables.incoming,
        &edge_tables.outgoing,
    );

    let mut index = LinkGraphIndex {
        root: inputs.root,
        include_dirs: inputs.normalized_include_dirs,
        excluded_dirs: inputs.normalized_excluded_dirs,
        docs_by_id: note_tables.docs_by_id,
        sections_by_doc: note_tables.sections_by_doc,
        passages_by_id: HashMap::new(),
        attachments_by_doc: note_tables.attachments_by_doc,
        trees_by_doc: HashMap::new(),
        node_parent_map: HashMap::new(),
        explicit_id_registry: HashMap::new(),
        alias_to_doc_id: note_tables.alias_to_doc_id,
        outgoing: edge_tables.outgoing,
        incoming: edge_tables.incoming,
        rank_by_id,
        edge_count: edge_tables.edge_count,
        virtual_nodes,
        symbol_to_docs: HashMap::new(),
    };
    index.rebuild_all_page_indices();
    index
}
