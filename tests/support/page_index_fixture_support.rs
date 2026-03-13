use serde_json::{Value, json};
use xiuxian_wendao::{
    LinkGraphIndex,
    link_graph::{LinkGraphSemanticDocument, LinkGraphSemanticDocumentKind, PageIndexNode},
};

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::fixture_read::read_fixture;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

/// Map legacy scenario names to new scenario directory names
fn map_scenario_name(scenario: &str) -> String {
    match scenario {
        "hierarchy" => "001_page_index_hierarchy".to_string(),
        "headingless" => "002_page_index_headingless".to_string(),
        "thinning" => "003_page_index_thinning".to_string(),
        "refresh" => "004_page_index_refresh".to_string(),
        "semantic_documents" => "005_page_index_semantic_docs".to_string(),
        other => other.to_string(),
    }
}

pub(super) struct PageIndexFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl PageIndexFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mapped = map_scenario_name(scenario);
        let temp_dir = materialize_link_graph_fixture(&format!("scenarios/{mapped}/input"))?;
        let root = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_dir: temp_dir,
            root,
        })
    }

    pub(super) fn build_index(&self) -> Result<LinkGraphIndex, Box<dyn std::error::Error>> {
        LinkGraphIndex::build(self.root.as_path()).map_err(|error| error.clone().into())
    }

    pub(super) fn path(&self, relative: &str) -> std::path::PathBuf {
        self.root.join(relative)
    }
}

pub(super) fn read_page_index_fixture(scenario: &str, relative: &str) -> String {
    let mapped = map_scenario_name(scenario);
    read_fixture(&format!("scenarios/{mapped}"), relative)
}

pub(super) fn assert_page_index_fixture(scenario: &str, relative: &str, actual: &Value) {
    let mapped = map_scenario_name(scenario);
    assert_json_fixture_eq(&format!("scenarios/{mapped}/expected"), relative, actual);
}

pub(super) fn page_index_tree_snapshot(nodes: &[PageIndexNode]) -> Value {
    json!({
        "nodes": nodes.iter().map(snapshot_page_index_node).collect::<Vec<_>>(),
    })
}

pub(super) fn semantic_documents_snapshot(documents: &[LinkGraphSemanticDocument]) -> Value {
    let mut ordered = documents
        .iter()
        .map(snapshot_semantic_document)
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        let left_key = (
            left["kind"].as_str().unwrap_or_default(),
            left["anchor_id"].as_str().unwrap_or_default(),
        );
        let right_key = (
            right["kind"].as_str().unwrap_or_default(),
            right["anchor_id"].as_str().unwrap_or_default(),
        );
        left_key.cmp(&right_key)
    });

    json!({ "documents": ordered })
}

fn snapshot_page_index_node(node: &PageIndexNode) -> Value {
    json!({
        "node_id": node.node_id,
        "title": node.title,
        "level": node.level,
        "text": node.text.as_ref(),
        "summary": node.summary,
        "line_range": [node.metadata.line_range.0, node.metadata.line_range.1],
        "token_count": node.metadata.token_count,
        "is_thinned": node.metadata.is_thinned,
        "children": node.children.iter().map(snapshot_page_index_node).collect::<Vec<_>>(),
    })
}

fn snapshot_semantic_document(document: &LinkGraphSemanticDocument) -> Value {
    json!({
        "anchor_id": document.anchor_id,
        "doc_id": document.doc_id,
        "path": document.path,
        "kind": semantic_document_kind_label(document.kind),
        "semantic_path": document.semantic_path,
        "content": document.content.as_ref(),
        "line_range": document.line_range.map(|(start, end)| vec![start, end]),
    })
}

fn semantic_document_kind_label(kind: LinkGraphSemanticDocumentKind) -> &'static str {
    match kind {
        LinkGraphSemanticDocumentKind::Summary => "summary",
        LinkGraphSemanticDocumentKind::Section => "section",
    }
}
