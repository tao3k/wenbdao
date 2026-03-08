use super::*;
use crate::fixture_json_assertions::assert_json_fixture_eq;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct SearchProvisionalFixture {
    _temp_dir: TempDir,
    root: PathBuf,
}

impl SearchProvisionalFixture {
    pub(crate) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("wendao_cli")
            .join("search")
            .join("provisional_overlay")
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

    pub(crate) fn config_path(&self) -> PathBuf {
        self.root.join("wendao.yaml")
    }
}

pub(crate) fn assert_search_provisional_fixture(scenario: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("wendao_cli/search/provisional_overlay/{scenario}/expected"),
        "result.json",
        actual,
    );
}

pub(crate) fn write_config(
    path: &Path,
    prefix: &str,
    include_default: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let search_block = if include_default {
        "    search:\n      include_provisional_default: true\n      provisional_limit: 10\n"
    } else {
        ""
    };
    fs::write(
        path,
        format!(
            "link_graph:\n  cache:\n    valkey_url: \"redis://127.0.0.1:6379/0\"\n    key_prefix: \"{prefix}\"\n  agentic:\n    suggested_link:\n      max_entries: 64\n      ttl_seconds: null\n{search_block}"
        ),
    )?;
    Ok(())
}

pub(crate) fn payload_snapshot(payload: &Value) -> Value {
    let results = payload
        .get("results")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().map(result_row_snapshot).collect::<Vec<_>>());
    let provisional = payload
        .get("provisional_suggestions")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(provisional_row_snapshot)
                .collect::<Vec<_>>()
        });
    let injected_paths = payload
        .get("results")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    row.get("match_reason")
                        .and_then(Value::as_str)
                        .is_some_and(|reason| reason.contains("agentic_provisional"))
                })
                .filter_map(|row| row.get("path").and_then(Value::as_str).map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    json!({
        "provisional_error": payload.get("provisional_error").and_then(Value::as_str),
        "results": results,
        "provisional_suggestions": provisional,
        "injected_paths": injected_paths,
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

fn provisional_row_snapshot(row: &Value) -> Value {
    json!({
        "source_id": row.get("source_id").and_then(Value::as_str),
        "target_id": row.get("target_id").and_then(Value::as_str),
        "relation": row.get("relation").and_then(Value::as_str),
        "state": row.get("state").and_then(Value::as_str),
        "agent_id": row.get("agent_id").and_then(Value::as_str),
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
