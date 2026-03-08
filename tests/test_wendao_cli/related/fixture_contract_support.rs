use crate::fixture_json_assertions::assert_json_fixture_eq;
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub(crate) struct RelatedCliFixture {
    _temp_dir: TempDir,
    root: PathBuf,
}

impl RelatedCliFixture {
    pub(crate) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("wendao_cli")
            .join("related")
            .join(scenario)
            .join("input");
        copy_tree(fixture_root.as_path(), temp_dir.path())?;
        let root = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_dir: temp_dir,
            root,
        })
    }

    pub(crate) fn root(&self) -> &Path {
        self.root.as_path()
    }
}

pub(crate) fn assert_related_cli_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("wendao_cli/related/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(crate) fn related_rows_snapshot(payload: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let rows = payload
        .as_array()
        .ok_or("expected related output to be a json array")?;
    Ok(rows_snapshot(rows))
}

pub(crate) fn related_verbose_snapshot(
    payload: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let results = payload
        .get("results")
        .and_then(Value::as_array)
        .ok_or("expected verbose results array")?;
    let diagnostics = payload
        .get("diagnostics")
        .ok_or("missing diagnostics payload")?;

    Ok(json!({
        "stem": payload.get("stem").and_then(Value::as_str),
        "max_distance": payload.get("max_distance").and_then(Value::as_u64),
        "limit": payload.get("limit").and_then(Value::as_u64),
        "total": payload.get("total").and_then(Value::as_u64),
        "ppr": ppr_snapshot(payload),
        "results": rows_snapshot(results),
        "diagnostics": diagnostics_snapshot(diagnostics),
        "monitor": monitor_snapshot(payload)?,
        "promoted_overlay": promoted_overlay_snapshot(payload),
    }))
}

fn rows_snapshot(rows: &[Value]) -> Value {
    let rows = sorted_rows(rows);
    let stems = rows
        .iter()
        .filter_map(|row| row["stem"].as_str().map(str::to_string))
        .collect::<Vec<_>>();

    json!({
        "row_count": rows.len(),
        "stems": stems,
        "rows": rows,
    })
}

fn sorted_rows(rows: &[Value]) -> Vec<Value> {
    let mut rows = rows.iter().map(snapshot_row).collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left["stem"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["stem"].as_str().unwrap_or_default())
            .then_with(|| {
                left["path"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(right["path"].as_str().unwrap_or_default())
            })
    });
    rows
}

fn ppr_snapshot(payload: &Value) -> Value {
    json!({
        "alpha": payload.get("ppr").and_then(|row| row.get("alpha")).and_then(Value::as_f64),
        "max_iter": payload.get("ppr").and_then(|row| row.get("max_iter")).and_then(Value::as_u64),
        "tol": payload.get("ppr").and_then(|row| row.get("tol")).and_then(Value::as_f64),
        "subgraph_mode": payload.get("ppr").and_then(|row| row.get("subgraph_mode")).and_then(Value::as_str),
    })
}

fn diagnostics_snapshot(diagnostics: &Value) -> Value {
    json!({
        "alpha": diagnostics.get("alpha").and_then(Value::as_f64),
        "max_iter": diagnostics.get("max_iter").and_then(Value::as_u64),
        "tol": diagnostics.get("tol").and_then(Value::as_f64),
        "subgraph_count": diagnostics.get("subgraph_count").and_then(Value::as_u64),
        "partition_max_node_count": diagnostics.get("partition_max_node_count").and_then(Value::as_u64),
        "partition_min_node_count": diagnostics.get("partition_min_node_count").and_then(Value::as_u64),
        "partition_avg_node_count": diagnostics.get("partition_avg_node_count").and_then(Value::as_f64),
        "subgraph_mode": diagnostics.get("subgraph_mode").and_then(Value::as_str),
        "horizon_restricted": diagnostics.get("horizon_restricted").and_then(Value::as_bool),
        "has_iteration_count": diagnostics.get("iteration_count").and_then(Value::as_u64).is_some(),
        "has_candidate_count": diagnostics.get("candidate_count").and_then(Value::as_u64).is_some(),
        "has_candidate_cap": diagnostics.get("candidate_cap").and_then(Value::as_u64).is_some(),
        "has_graph_node_count": diagnostics.get("graph_node_count").and_then(Value::as_u64).is_some(),
        "has_final_residual": diagnostics.get("final_residual").and_then(Value::as_f64).is_some(),
        "total_duration_ms_non_negative": non_negative_float(diagnostics, "total_duration_ms"),
        "partition_duration_ms_non_negative": non_negative_float(diagnostics, "partition_duration_ms"),
        "kernel_duration_ms_non_negative": non_negative_float(diagnostics, "kernel_duration_ms"),
        "fusion_duration_ms_non_negative": non_negative_float(diagnostics, "fusion_duration_ms"),
        "time_budget_ms_non_negative": non_negative_float(diagnostics, "time_budget_ms"),
        "candidate_capped_is_bool": diagnostics.get("candidate_capped").and_then(Value::as_bool).is_some(),
        "timed_out_is_bool": diagnostics.get("timed_out").and_then(Value::as_bool).is_some(),
    })
}

fn monitor_snapshot(payload: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let mut phase_labels = payload
        .get("phases")
        .and_then(Value::as_array)
        .ok_or("missing monitor phases")?
        .iter()
        .filter_map(|row| row.get("phase").and_then(Value::as_str).map(str::to_string))
        .collect::<Vec<_>>();
    phase_labels.sort();
    phase_labels.dedup();

    Ok(json!({
        "phase_labels": phase_labels,
        "has_slowest_phase": payload
            .get("monitor")
            .and_then(|row| row.get("bottlenecks"))
            .and_then(|row| row.get("slowest_phase"))
            .is_some(),
    }))
}

fn promoted_overlay_snapshot(payload: &Value) -> Value {
    json!({
        "applied_is_bool": payload
            .get("promoted_overlay")
            .and_then(|row| row.get("applied"))
            .and_then(Value::as_bool)
            .is_some(),
        "source": payload
            .get("promoted_overlay")
            .and_then(|row| row.get("source"))
            .and_then(Value::as_str),
    })
}

fn non_negative_float(payload: &Value, key: &str) -> bool {
    payload
        .get(key)
        .and_then(Value::as_f64)
        .is_some_and(|value| value >= 0.0)
}

fn snapshot_row(row: &Value) -> Value {
    json!({
        "stem": row.get("stem").and_then(Value::as_str),
        "path": row.get("path").and_then(Value::as_str),
        "distance": row.get("distance").and_then(Value::as_u64),
        "title": row.get("title").and_then(Value::as_str),
    })
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target_path)?;
            copy_tree(source_path.as_path(), target_path.as_path())?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source_path.as_path(), target_path.as_path())?;
        }
    }

    Ok(())
}
