use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::wendao_cli_search_gateway_contract_support::agentic_gateway_snapshot;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub(crate) struct SearchDirectivesFixture {
    _temp_dir: TempDir,
    root: PathBuf,
}

impl SearchDirectivesFixture {
    pub(crate) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (temp_dir, root) =
            crate::wendao_cli_fixture_tree_support::materialize_wendao_cli_fixture(&format!(
                "search/directives/{scenario}"
            ))?;
        Ok(Self {
            _temp_dir: temp_dir,
            root,
        })
    }

    pub(crate) fn root(&self) -> &Path {
        self.root.as_path()
    }
}

pub(crate) fn assert_search_directives_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("wendao_cli/search/directives/{scenario}/expected"),
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
        "created_after": payload.get("created_after").and_then(Value::as_i64),
        "created_before": payload.get("created_before").and_then(Value::as_i64),
        "sort_terms": payload
            .get("sort_terms")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().map(sort_term_snapshot).collect::<Vec<_>>()),
        "filters": filter_seed_snapshot(payload.get("filters")),
        "results": payload
            .get("results")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().map(result_row_snapshot).collect::<Vec<_>>()),
        "agentic_gateway": agentic_gateway_snapshot(payload.get("agentic_gateway")),
    })
}

pub(crate) fn legacy_sort_error_snapshot(output: &std::process::Output) -> Value {
    let stderr = String::from_utf8_lossy(&output.stderr);
    json!({
        "status_success": output.status.success(),
        "mentions_unexpected_sort_flag": stderr.contains("unexpected argument '--sort'"),
        "mentions_sort_term_hint": stderr.contains("--sort-term"),
    })
}

fn filter_seed_snapshot(filters: Option<&Value>) -> Value {
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
