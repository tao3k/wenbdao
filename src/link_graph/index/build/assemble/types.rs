use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::link_graph::models::{LinkGraphAttachment, LinkGraphDocument};
use crate::link_graph::parser::ParsedNote;

pub(crate) struct BuildInputs {
    pub(crate) root: PathBuf,
    pub(crate) normalized_include_dirs: Vec<String>,
    pub(crate) normalized_excluded_dirs: Vec<String>,
    pub(crate) included: HashSet<String>,
    pub(crate) excluded: HashSet<String>,
}

pub(crate) struct NoteTables {
    pub(crate) parsed_notes: Vec<ParsedNote>,
    pub(crate) docs_by_id: HashMap<String, LinkGraphDocument>,
    pub(crate) sections_by_doc: HashMap<String, Vec<crate::link_graph::index::IndexedSection>>,
    pub(crate) attachments_by_doc: HashMap<String, Vec<LinkGraphAttachment>>,
    pub(crate) alias_to_doc_id: HashMap<String, String>,
}

pub(crate) struct EdgeTables {
    pub(crate) outgoing: HashMap<String, HashSet<String>>,
    pub(crate) incoming: HashMap<String, HashSet<String>>,
    pub(crate) edge_count: usize,
}
