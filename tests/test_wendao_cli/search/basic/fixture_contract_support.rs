use super::*;
use crate::fixture_json_assertions::assert_json_fixture_eq;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct SearchBasicFixture {
    _temp_dir: TempDir,
    root: PathBuf,
}

impl SearchBasicFixture {
    pub(crate) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("wendao_cli")
            .join("search")
            .join("basic")
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

pub(crate) fn assert_search_basic_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("wendao_cli/search/basic/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(crate) fn search_payload_snapshot(payload: &Value) -> Value {
    json!({
        "query": payload.get("query").and_then(Value::as_str),
        "limit": payload.get("limit").and_then(Value::as_u64),
        "match_strategy": payload.get("match_strategy").and_then(Value::as_str),
        "case_sensitive": payload.get("case_sensitive").and_then(Value::as_bool),
        "sort_terms": payload
            .get("sort_terms")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().map(sort_term_snapshot).collect::<Vec<_>>()),
        "filters": filter_presence_snapshot(payload.get("filters")),
        "results": payload
            .get("results")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().map(result_row_snapshot).collect::<Vec<_>>()),
    })
}

pub(crate) fn search_verbose_snapshot(
    payload: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let phases = payload
        .get("phases")
        .and_then(Value::as_array)
        .ok_or("missing phases")?;
    let mut phase_labels = phases
        .iter()
        .filter_map(|row| row.get("phase").and_then(Value::as_str).map(str::to_string))
        .collect::<Vec<_>>();
    phase_labels.sort();
    phase_labels.dedup();

    Ok(json!({
        "requested_mode": payload.get("requested_mode"),
        "selected_mode": payload.get("selected_mode"),
        "reason": payload.get("reason"),
        "graph_confidence_score": payload.get("graph_confidence_score"),
        "graph_confidence_level": payload.get("graph_confidence_level"),
        "results": payload
            .get("results")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().map(result_row_snapshot).collect::<Vec<_>>()),
        "phase_labels": phase_labels,
        "policy_reason_validated": phases.iter().any(|row| {
            row.get("phase").and_then(Value::as_str) == Some("link_graph.search.policy")
                && row
                    .get("extra")
                    .and_then(|extra| extra.get("reason_validated"))
                    .and_then(Value::as_bool)
                    == Some(true)
        }),
        "has_monitor_slowest_phase": payload
            .get("monitor")
            .and_then(|row| row.get("bottlenecks"))
            .and_then(|row| row.get("slowest_phase"))
            .is_some(),
        "retrieval_plan": retrieval_plan_snapshot(payload.get("retrieval_plan")),
    }))
}

fn filter_presence_snapshot(filters: Option<&Value>) -> Value {
    json!({
        "link_to_seed_count": filters
            .and_then(|row| row.get("link_to"))
            .and_then(|row| row.get("seeds"))
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
        "linked_by_seed_count": filters
            .and_then(|row| row.get("linked_by"))
            .and_then(|row| row.get("seeds"))
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
        "related_seed_count": filters
            .and_then(|row| row.get("related"))
            .and_then(|row| row.get("seeds"))
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
    })
}

fn retrieval_plan_snapshot(retrieval_plan: Option<&Value>) -> Value {
    json!({
        "has_semantic_policy": retrieval_plan
            .and_then(|row| row.get("semantic_policy"))
            .is_some(),
        "document_scope": retrieval_plan
            .and_then(|row| row.get("semantic_policy"))
            .and_then(|row| row.get("document_scope")),
    })
}

fn sort_term_snapshot(sort_term: &Value) -> Value {
    json!({
        "field": sort_term.get("field").and_then(Value::as_str),
        "order": sort_term.get("order").and_then(Value::as_str),
    })
}

fn result_row_snapshot(row: &Value) -> Value {
    json!({
        "stem": row.get("stem").and_then(Value::as_str),
        "title": row.get("title").and_then(Value::as_str),
        "path": row.get("path").and_then(Value::as_str),
        "best_section": row.get("best_section").and_then(Value::as_str),
        "match_reason": row.get("match_reason").and_then(Value::as_str),
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
