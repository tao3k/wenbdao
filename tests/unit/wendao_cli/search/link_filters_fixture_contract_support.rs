use crate::fixture_json_assertions::assert_json_fixture_eq;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub(crate) struct SearchLinkFiltersFixture {
    _temp_dir: TempDir,
    root: PathBuf,
}

impl SearchLinkFiltersFixture {
    pub(crate) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("wendao_cli")
            .join("search")
            .join("link_filters")
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

pub(crate) fn assert_search_link_filters_fixture(scenario: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("wendao_cli/search/link_filters/{scenario}/expected"),
        "result.json",
        actual,
    );
}

pub(crate) fn payload_snapshot(payload: &Value) -> Value {
    json!({
        "query": payload.get("query").and_then(Value::as_str),
        "limit": payload.get("limit").and_then(Value::as_u64),
        "sort_terms": payload
            .get("sort_terms")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().map(sort_term_snapshot).collect::<Vec<_>>()),
        "filters": filter_snapshot(payload.get("filters")),
        "results": payload
            .get("results")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().map(result_row_snapshot).collect::<Vec<_>>()),
    })
}

fn filter_snapshot(filters: Option<&Value>) -> Value {
    let related = filters.and_then(|row| row.get("related"));
    let related_ppr = related.and_then(|row| row.get("ppr"));
    json!({
        "link_to_seed_count": filters
            .and_then(|row| row.get("link_to"))
            .and_then(|row| row.get("seeds"))
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
        "related": related.map(|row| json!({
            "seed_count": row.get("seeds").and_then(Value::as_array).map(std::vec::Vec::len),
            "max_distance": row.get("max_distance").and_then(Value::as_u64),
            "ppr": related_ppr.map(|ppr| json!({
                "alpha": ppr.get("alpha").and_then(Value::as_f64),
                "max_iter": ppr.get("max_iter").and_then(Value::as_u64),
                "tol": ppr.get("tol").and_then(Value::as_f64),
                "subgraph_mode": ppr.get("subgraph_mode").and_then(Value::as_str),
            })),
        })),
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
