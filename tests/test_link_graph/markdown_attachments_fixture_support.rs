use serde_json::{Value, json};
use xiuxian_wendao::link_graph::{
    LinkGraphAttachmentHit, LinkGraphAttachmentKind, LinkGraphNeighbor, LinkGraphStats,
};

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

pub(super) struct AttachmentFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl AttachmentFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = materialize_link_graph_fixture(&format!(
            "link_graph/markdown_attachments/{scenario}/input"
        ))?;
        let root = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_dir: temp_dir,
            root,
        })
    }

    pub(super) fn build_index(
        &self,
    ) -> Result<xiuxian_wendao::LinkGraphIndex, Box<dyn std::error::Error>> {
        xiuxian_wendao::LinkGraphIndex::build(self.root.as_path())
            .map_err(|error| error.clone().into())
    }
}

pub(super) fn assert_markdown_attachment_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/markdown_attachments/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(super) fn stats_and_neighbors_snapshot(
    stats: LinkGraphStats,
    neighbors: &[LinkGraphNeighbor],
) -> Value {
    let mut ordered = neighbors.iter().map(snapshot_neighbor).collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
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
        "neighbors": ordered,
    })
}

pub(super) fn attachment_hits_snapshot(hits: &[LinkGraphAttachmentHit]) -> Value {
    let mut ordered = hits.iter().map(snapshot_attachment_hit).collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        let left_key = (
            left["source_path"].as_str().unwrap_or_default(),
            left["attachment_path"].as_str().unwrap_or_default(),
        );
        let right_key = (
            right["source_path"].as_str().unwrap_or_default(),
            right["attachment_path"].as_str().unwrap_or_default(),
        );
        left_key.cmp(&right_key)
    });

    json!({
        "hit_count": ordered.len(),
        "hits": ordered,
    })
}

fn snapshot_neighbor(row: &LinkGraphNeighbor) -> Value {
    json!({
        "stem": row.stem,
        "direction": match row.direction {
            xiuxian_wendao::link_graph::LinkGraphDirection::Incoming => "incoming",
            xiuxian_wendao::link_graph::LinkGraphDirection::Outgoing => "outgoing",
            xiuxian_wendao::link_graph::LinkGraphDirection::Both => "both",
        },
        "distance": row.distance,
        "title": row.title,
        "path": row.path,
    })
}

fn snapshot_attachment_hit(hit: &LinkGraphAttachmentHit) -> Value {
    json!({
        "source_stem": hit.source_stem,
        "source_title": hit.source_title,
        "source_path": hit.source_path,
        "attachment_path": hit.attachment_path,
        "attachment_name": hit.attachment_name,
        "attachment_ext": hit.attachment_ext,
        "kind": attachment_kind_label(hit.kind),
    })
}

fn attachment_kind_label(kind: LinkGraphAttachmentKind) -> &'static str {
    match kind {
        LinkGraphAttachmentKind::Image => "image",
        LinkGraphAttachmentKind::Pdf => "pdf",
        LinkGraphAttachmentKind::Gpg => "gpg",
        LinkGraphAttachmentKind::Document => "document",
        LinkGraphAttachmentKind::Archive => "archive",
        LinkGraphAttachmentKind::Audio => "audio",
        LinkGraphAttachmentKind::Video => "video",
        LinkGraphAttachmentKind::Other => "other",
    }
}
