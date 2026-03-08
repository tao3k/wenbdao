use super::super::fingerprint::LinkGraphFingerprint;
use super::schema::{LINK_GRAPH_VALKEY_CACHE_SCHEMA_VERSION, cache_schema_fingerprint};
use crate::link_graph::index::{IndexedSection, LinkGraphIndex};
use crate::link_graph::models::{
    LinkGraphAttachment, LinkGraphDocument, LinkGraphPassage, PageIndexMeta, PageIndexNode,
};
use crate::link_graph::saliency::{DEFAULT_DECAY_RATE, DEFAULT_SALIENCY_BASE};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

fn snapshot_default_saliency_base() -> f64 {
    DEFAULT_SALIENCY_BASE
}

fn snapshot_default_decay_rate() -> f64 {
    DEFAULT_DECAY_RATE
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotDocument {
    id: String,
    id_lower: String,
    stem: String,
    stem_lower: String,
    path: String,
    path_lower: String,
    title: String,
    title_lower: String,
    tags: Vec<String>,
    tags_lower: Vec<String>,
    lead: String,
    doc_type: Option<String>,
    word_count: usize,
    search_text: String,
    search_text_lower: String,
    #[serde(default = "snapshot_default_saliency_base")]
    saliency_base: f64,
    #[serde(default = "snapshot_default_decay_rate")]
    decay_rate: f64,
    created_ts: Option<i64>,
    modified_ts: Option<i64>,
}

impl From<&LinkGraphDocument> for SnapshotDocument {
    fn from(value: &LinkGraphDocument) -> Self {
        Self {
            id: value.id.clone(),
            id_lower: value.id_lower.clone(),
            stem: value.stem.clone(),
            stem_lower: value.stem_lower.clone(),
            path: value.path.clone(),
            path_lower: value.path_lower.clone(),
            title: value.title.clone(),
            title_lower: value.title_lower.clone(),
            tags: value.tags.clone(),
            tags_lower: value.tags_lower.clone(),
            lead: value.lead.clone(),
            doc_type: value.doc_type.clone(),
            word_count: value.word_count,
            search_text: value.search_text.clone(),
            search_text_lower: value.search_text_lower.clone(),
            saliency_base: value.saliency_base,
            decay_rate: value.decay_rate,
            created_ts: value.created_ts,
            modified_ts: value.modified_ts,
        }
    }
}

impl SnapshotDocument {
    fn into_document(self) -> LinkGraphDocument {
        LinkGraphDocument {
            id: self.id,
            id_lower: self.id_lower,
            stem: self.stem,
            stem_lower: self.stem_lower,
            path: self.path,
            path_lower: self.path_lower,
            title: self.title,
            title_lower: self.title_lower,
            tags: self.tags,
            tags_lower: self.tags_lower,
            lead: self.lead,
            doc_type: self.doc_type,
            word_count: self.word_count,
            search_text: self.search_text,
            search_text_lower: self.search_text_lower,
            saliency_base: self.saliency_base,
            decay_rate: self.decay_rate,
            created_ts: self.created_ts,
            modified_ts: self.modified_ts,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotPageIndexMeta {
    line_range: (usize, usize),
    token_count: usize,
    is_thinned: bool,
}

impl From<&PageIndexMeta> for SnapshotPageIndexMeta {
    fn from(value: &PageIndexMeta) -> Self {
        Self {
            line_range: value.line_range,
            token_count: value.token_count,
            is_thinned: value.is_thinned,
        }
    }
}

impl From<SnapshotPageIndexMeta> for PageIndexMeta {
    fn from(value: SnapshotPageIndexMeta) -> Self {
        Self {
            line_range: value.line_range,
            token_count: value.token_count,
            is_thinned: value.is_thinned,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotPageIndexNode {
    node_id: String,
    title: String,
    level: usize,
    text: String,
    summary: Option<String>,
    children: Vec<SnapshotPageIndexNode>,
    metadata: SnapshotPageIndexMeta,
}

impl From<&PageIndexNode> for SnapshotPageIndexNode {
    fn from(value: &PageIndexNode) -> Self {
        Self {
            node_id: value.node_id.clone(),
            title: value.title.clone(),
            level: value.level,
            text: value.text.to_string(),
            summary: value.summary.clone(),
            children: value.children.iter().map(Self::from).collect(),
            metadata: SnapshotPageIndexMeta::from(&value.metadata),
        }
    }
}

impl SnapshotPageIndexNode {
    fn into_node(self) -> PageIndexNode {
        PageIndexNode {
            node_id: self.node_id,
            title: self.title,
            level: self.level,
            text: Arc::<str>::from(self.text),
            summary: self.summary,
            children: self.children.into_iter().map(Self::into_node).collect(),
            metadata: self.metadata.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LinkGraphIndexSnapshot {
    schema_version: String,
    #[serde(default)]
    schema_fingerprint: Option<String>,
    root: PathBuf,
    include_dirs: Vec<String>,
    excluded_dirs: Vec<String>,
    fingerprint: LinkGraphFingerprint,
    docs_by_id: HashMap<String, SnapshotDocument>,
    #[serde(default)]
    passages_by_id: HashMap<String, LinkGraphPassage>,
    sections_by_doc: HashMap<String, Vec<IndexedSection>>,
    #[serde(default)]
    trees_by_doc: HashMap<String, Vec<SnapshotPageIndexNode>>,
    #[serde(default)]
    attachments_by_doc: HashMap<String, Vec<LinkGraphAttachment>>,
    alias_to_doc_id: HashMap<String, String>,
    outgoing: HashMap<String, HashSet<String>>,
    incoming: HashMap<String, HashSet<String>>,
    rank_by_id: HashMap<String, f64>,
    edge_count: usize,
}

impl LinkGraphIndexSnapshot {
    pub(super) fn from_index(index: &LinkGraphIndex, fingerprint: LinkGraphFingerprint) -> Self {
        let docs_by_id = index
            .docs_by_id
            .iter()
            .map(|(k, v)| (k.clone(), SnapshotDocument::from(v)))
            .collect();
        let trees_by_doc = index
            .trees_by_doc
            .iter()
            .map(|(doc_id, roots)| {
                (
                    doc_id.clone(),
                    roots.iter().map(SnapshotPageIndexNode::from).collect(),
                )
            })
            .collect();
        Self {
            schema_version: LINK_GRAPH_VALKEY_CACHE_SCHEMA_VERSION.to_string(),
            schema_fingerprint: Some(cache_schema_fingerprint().to_string()),
            root: index.root.clone(),
            include_dirs: index.include_dirs.clone(),
            excluded_dirs: index.excluded_dirs.clone(),
            fingerprint,
            docs_by_id,
            passages_by_id: index.passages_by_id.clone(),
            sections_by_doc: index.sections_by_doc.clone(),
            trees_by_doc,
            attachments_by_doc: index.attachments_by_doc.clone(),
            alias_to_doc_id: index.alias_to_doc_id.clone(),
            outgoing: index.outgoing.clone(),
            incoming: index.incoming.clone(),
            rank_by_id: index.rank_by_id.clone(),
            edge_count: index.edge_count,
        }
    }

    pub(super) fn into_index(self) -> LinkGraphIndex {
        let docs_by_id = self
            .docs_by_id
            .into_iter()
            .map(|(k, v)| (k, v.into_document()))
            .collect();
        let trees_by_doc = self
            .trees_by_doc
            .into_iter()
            .map(|(doc_id, roots)| {
                (
                    doc_id,
                    roots
                        .into_iter()
                        .map(SnapshotPageIndexNode::into_node)
                        .collect(),
                )
            })
            .collect();
        let mut index = LinkGraphIndex {
            root: self.root,
            include_dirs: self.include_dirs,
            excluded_dirs: self.excluded_dirs,
            docs_by_id,
            passages_by_id: self.passages_by_id,
            sections_by_doc: self.sections_by_doc,
            trees_by_doc,
            attachments_by_doc: self.attachments_by_doc,
            alias_to_doc_id: self.alias_to_doc_id,
            outgoing: self.outgoing,
            incoming: self.incoming,
            rank_by_id: self.rank_by_id,
            edge_count: self.edge_count,
        };
        if index.trees_by_doc.is_empty() {
            index.rebuild_all_page_indices();
        }
        index
    }

    pub(super) fn root(&self) -> &PathBuf {
        &self.root
    }

    pub(super) fn include_dirs(&self) -> &[String] {
        &self.include_dirs
    }

    pub(super) fn excluded_dirs(&self) -> &[String] {
        &self.excluded_dirs
    }

    pub(super) fn fingerprint(&self) -> &LinkGraphFingerprint {
        &self.fingerprint
    }

    pub(super) fn schema_version(&self) -> &str {
        &self.schema_version
    }

    pub(super) fn schema_fingerprint(&self) -> Option<&str> {
        self.schema_fingerprint.as_deref()
    }
}
