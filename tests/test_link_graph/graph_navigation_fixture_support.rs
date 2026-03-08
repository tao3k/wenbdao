use serde_json::{Value, json};
use xiuxian_wendao::link_graph::{
    LinkGraphDirection, LinkGraphDocument, LinkGraphMetadata, LinkGraphNeighbor,
    LinkGraphPprSubgraphMode, LinkGraphRelatedPprDiagnostics,
};

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

const FLOAT_PRECISION: f64 = 1_000_000_000_000.0;

pub(super) struct NavigationFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl NavigationFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = materialize_link_graph_fixture(&format!(
            "link_graph/graph_navigation/{scenario}/input"
        ))?;
        let root = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_dir: temp_dir,
            root,
        })
    }

    pub(super) fn path(&self, relative: &str) -> std::path::PathBuf {
        self.root.join(relative)
    }
}

pub(super) fn assert_graph_navigation_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/graph_navigation/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(super) fn navigation_surface_snapshot(
    neighbors: &[LinkGraphNeighbor],
    related: &[LinkGraphNeighbor],
    metadata: &LinkGraphMetadata,
    toc: &[LinkGraphDocument],
) -> Value {
    let mut ordered_neighbors = neighbors.iter().map(snapshot_neighbor).collect::<Vec<_>>();
    ordered_neighbors.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });

    let mut ordered_related = related.iter().map(snapshot_neighbor).collect::<Vec<_>>();
    ordered_related.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });

    let mut ordered_toc = toc.iter().map(snapshot_document).collect::<Vec<_>>();
    ordered_toc.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });

    json!({
        "neighbors": ordered_neighbors,
        "related": ordered_related,
        "metadata": snapshot_metadata(metadata),
        "toc": ordered_toc,
    })
}

pub(super) fn related_diagnostics_snapshot(
    rows: &[LinkGraphNeighbor],
    diagnostics: Option<LinkGraphRelatedPprDiagnostics>,
) -> Value {
    let mut ordered_rows = rows.iter().map(snapshot_neighbor).collect::<Vec<_>>();
    ordered_rows.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });

    json!({
        "rows": ordered_rows,
        "diagnostics": diagnostics.map(snapshot_diagnostics),
    })
}

fn snapshot_neighbor(row: &LinkGraphNeighbor) -> Value {
    json!({
        "stem": row.stem,
        "direction": direction_label(row.direction),
        "distance": row.distance,
        "title": row.title,
        "path": row.path,
    })
}

fn snapshot_metadata(metadata: &LinkGraphMetadata) -> Value {
    let mut tags = metadata.tags.clone();
    tags.sort();

    json!({
        "stem": metadata.stem,
        "title": metadata.title,
        "path": metadata.path,
        "tags": tags,
    })
}

fn snapshot_document(document: &LinkGraphDocument) -> Value {
    let mut tags = document.tags.clone();
    tags.sort();

    json!({
        "id": document.id,
        "stem": document.stem,
        "title": document.title,
        "path": document.path,
        "tags": tags,
        "lead": document.lead,
        "doc_type": document.doc_type,
        "word_count": document.word_count,
    })
}

fn snapshot_diagnostics(metrics: LinkGraphRelatedPprDiagnostics) -> Value {
    json!({
        "alpha": round_float(metrics.alpha),
        "max_iter": metrics.max_iter,
        "tol": round_float(metrics.tol),
        "candidate_count": metrics.candidate_count,
        "candidate_cap": metrics.candidate_cap,
        "candidate_cap_ge_candidate_count": metrics.candidate_cap >= metrics.candidate_count,
        "candidate_capped": metrics.candidate_capped,
        "graph_node_count": metrics.graph_node_count,
        "subgraph_count": metrics.subgraph_count,
        "partition_max_node_count": metrics.partition_max_node_count,
        "partition_min_node_count": metrics.partition_min_node_count,
        "partition_avg_node_count": round_float(metrics.partition_avg_node_count),
        "subgraph_mode": subgraph_mode_label(metrics.subgraph_mode),
        "horizon_restricted": metrics.horizon_restricted,
        "timed_out": metrics.timed_out,
        "iteration_count_ge_one": metrics.iteration_count >= 1,
        "final_residual_non_negative": metrics.final_residual >= 0.0,
        "total_duration_non_negative": metrics.total_duration_ms >= 0.0,
        "partition_duration_non_negative": metrics.partition_duration_ms >= 0.0,
        "kernel_duration_non_negative": metrics.kernel_duration_ms >= 0.0,
        "fusion_duration_non_negative": metrics.fusion_duration_ms >= 0.0,
        "time_budget_positive": metrics.time_budget_ms > 0.0,
    })
}

fn direction_label(direction: LinkGraphDirection) -> &'static str {
    match direction {
        LinkGraphDirection::Incoming => "incoming",
        LinkGraphDirection::Outgoing => "outgoing",
        LinkGraphDirection::Both => "both",
    }
}

fn subgraph_mode_label(mode: LinkGraphPprSubgraphMode) -> &'static str {
    match mode {
        LinkGraphPprSubgraphMode::Auto => "auto",
        LinkGraphPprSubgraphMode::Disabled => "disabled",
        LinkGraphPprSubgraphMode::Force => "force",
    }
}

fn round_float(value: f64) -> f64 {
    (value * FLOAT_PRECISION).round() / FLOAT_PRECISION
}
