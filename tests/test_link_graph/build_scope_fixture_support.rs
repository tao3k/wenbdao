use serde_json::{Value, json};
use xiuxian_wendao::link_graph::{LinkGraphDocument, LinkGraphStats};

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

pub(super) struct BuildScopeFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl BuildScopeFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir =
            materialize_link_graph_fixture(&format!("link_graph/build_scope/{scenario}/input"))?;
        let root = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_dir: temp_dir,
            root,
        })
    }

    pub(super) fn root(&self) -> &std::path::Path {
        self.root.as_path()
    }
}

pub(super) fn assert_build_scope_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/build_scope/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(super) fn stats_and_toc_snapshot(stats: LinkGraphStats, docs: &[LinkGraphDocument]) -> Value {
    let mut toc = docs.iter().map(snapshot_doc).collect::<Vec<_>>();
    toc.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });

    json!({
        "stats": {
            "total_notes": stats.total_notes,
            "orphans": stats.orphans,
            "links_in_graph": stats.links_in_graph,
            "nodes_in_graph": stats.nodes_in_graph,
        },
        "toc": toc,
    })
}

pub(super) fn docs_snapshot(docs: &[LinkGraphDocument]) -> Value {
    let mut ordered = docs.iter().map(snapshot_doc).collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });
    json!({ "documents": ordered })
}

fn snapshot_doc(document: &LinkGraphDocument) -> Value {
    let mut tags = document.tags.clone();
    tags.sort();

    json!({
        "id": document.id,
        "stem": document.stem,
        "path": document.path,
        "title": document.title,
        "tags": tags,
        "lead": document.lead,
        "doc_type": document.doc_type,
        "word_count": document.word_count,
    })
}
