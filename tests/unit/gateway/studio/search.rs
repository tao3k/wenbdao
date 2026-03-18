use super::*;
use crate::gateway::studio::router::{GatewayState, StudioState};
use crate::gateway::studio::types::{UiConfig, UiProjectConfig};
use serde_json::json;
use tempfile::tempdir;

#[path = "support.rs"]
mod support;
use support::{assert_studio_json_snapshot, round_f64};

struct StudioStateFixture {
    state: Arc<GatewayState>,
    _temp_dir: tempfile::TempDir,
}

fn create_temp_dir() -> tempfile::TempDir {
    match tempdir() {
        Ok(temp_dir) => temp_dir,
        Err(err) => panic!("failed to create temp dir fixture: {err}"),
    }
}

fn write_doc(root: &std::path::Path, name: &str, content: &str) {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            panic!("failed to create fixture parent dirs for {name}: {err}");
        }
    }
    if let Err(err) = std::fs::write(path, content) {
        panic!("failed to write fixture doc {name}: {err}");
    }
}

fn make_state_with_docs(docs: Vec<(&str, &str)>) -> StudioStateFixture {
    let temp_dir = create_temp_dir();
    for (name, content) in docs {
        write_doc(temp_dir.path(), name, content);
    }

    let mut studio_state = StudioState::new();
    studio_state.project_root = temp_dir.path().to_path_buf();
    studio_state.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            paths: vec![
                "packages".to_string(),
                ".data".to_string(),
                "internal_skills".to_string(),
            ],
            watch_patterns: vec![
                "**/*.rs".to_string(),
                "**/*.py".to_string(),
                "**/*.md".to_string(),
                "**/SKILL.md".to_string(),
            ],
            include_dirs_auto: false,
            include_dirs_auto_candidates: Vec::new(),
        }],
    });

    StudioStateFixture {
        state: Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio_state),
        }),
        _temp_dir: temp_dir,
    }
}

#[test]
fn test_strip_option() {
    assert_eq!(strip_option(""), None);
    assert_eq!(strip_option("value"), Some("value".to_string()));
    assert_eq!(strip_option(" value "), Some("value".to_string()));
}

#[tokio::test]
async fn search_knowledge_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = search_knowledge(
        Query(SearchQuery {
            q: Some("   ".to_string()),
            limit: None,
        }),
        State(Arc::clone(&fixture.state)),
    )
    .await;

    let Err(error) = result else {
        panic!("expected missing-query request to fail");
    };

    assert_eq!(error.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "MISSING_QUERY");
}

#[tokio::test]
async fn search_knowledge_returns_payload() {
    let fixture = make_state_with_docs(vec![
        (
            "alpha.md",
            "# Alpha\n\nThis note contains search target keyword: wendao.\n",
        ),
        (
            "beta.md",
            "# Beta\n\nAnother note mentions wendao in text.\n",
        ),
    ]);

    let result = search_knowledge(
        Query(SearchQuery {
            q: Some("wendao".to_string()),
            limit: Some(5),
        }),
        State(fixture.state),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_payload",
        json!({
            "query": response.0.query,
            "hitCount": response.0.hit_count,
            "selectedMode": response.0.selected_mode,
            "graphConfidenceScore": response.0.graph_confidence_score.map(round_f64),
            "hits": response.0.hits.into_iter().map(|hit| {
                json!({
                    "stem": hit.stem,
                    "title": hit.title,
                    "path": hit.path,
                    "docType": hit.doc_type,
                    "tags": hit.tags,
                    "score": round_f64(hit.score),
                    "bestSection": hit.best_section,
                    "matchReason": hit.match_reason,
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_knowledge_uses_studio_display_paths() {
    let fixture = make_state_with_docs(vec![(
        "alpha.md",
        "# Alpha\n\nThis note contains search target keyword: wendao.\n",
    )]);

    let result = search_knowledge(
        Query(SearchQuery {
            q: Some("wendao".to_string()),
            limit: Some(5),
        }),
        State(Arc::clone(&fixture.state)),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected search request to succeed");
    };
    let hit_paths = response
        .0
        .hits
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<Vec<_>>();

    assert!(
        hit_paths.iter().any(|path| path == "knowledge/alpha.md"),
        "unexpected hit paths: {hit_paths:?}",
    );
    assert!(
        hit_paths
            .iter()
            .all(|path| !path.ends_with("alpha.md") || path == "knowledge/alpha.md"),
        "unexpected hit paths: {hit_paths:?}",
    );
}

#[tokio::test]
async fn autocomplete_limits_and_filters_prefix() {
    let fixture = make_state_with_docs(vec![
        (
            "doc.md",
            "# Search Design\n\nThis doc starts with Search and discusses Search.\n",
        ),
        ("note.md", "# Search Notes\n\nTaggable text.\n"),
    ]);

    let result = search_autocomplete(
        Query(AutocompleteQuery {
            prefix: Some("se".to_string()),
            limit: Some(2),
        }),
        State(fixture.state),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected autocomplete request to succeed");
    };

    assert_studio_json_snapshot(
        "search_autocomplete_payload",
        json!({
            "prefix": response.0.prefix,
            "suggestions": response.0.suggestions.into_iter().map(|suggestion| {
                json!({
                    "text": suggestion.text,
                    "suggestionType": suggestion.suggestion_type,
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_ast_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = search_ast(
        Query(AstSearchQuery {
            q: Some("   ".to_string()),
            limit: None,
        }),
        State(Arc::clone(&fixture.state)),
    )
    .await;

    let Err(error) = result else {
        panic!("expected missing-query AST search to fail");
    };

    assert_eq!(error.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "MISSING_QUERY");
}

#[tokio::test]
async fn search_ast_returns_payload() {
    let fixture = make_state_with_docs(vec![
        (
            "packages/rust/crates/demo/src/lib.rs",
            "pub struct AlphaService {\n    ready: bool,\n}\n\npub fn alpha_handler() {}\n",
        ),
        (
            "packages/python/demo/tool.py",
            "class AlphaClient:\n    pass\n\ndef alpha_helper():\n    return None\n",
        ),
        (
            "notes/ignored.md",
            "# alpha\n\nThis markdown file should not affect AST search.\n",
        ),
    ]);

    let result = search_ast(
        Query(AstSearchQuery {
            q: Some("alpha".to_string()),
            limit: Some(10),
        }),
        State(fixture.state),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected AST search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_ast_payload",
        json!({
            "query": response.0.query,
            "hitCount": response.0.hit_count,
            "selectedScope": response.0.selected_scope,
            "hits": response.0.hits.into_iter().map(|hit| {
                json!({
                    "name": hit.name,
                    "signature": hit.signature,
                    "path": hit.path,
                    "language": hit.language,
                    "crateName": hit.crate_name,
                    "projectName": hit.project_name,
                    "rootLabel": hit.root_label,
                    "lineStart": hit.line_start,
                    "lineEnd": hit.line_end,
                    "score": round_f64(hit.score),
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_definition_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = search_definition(
        Query(DefinitionResolveQuery {
            q: Some("   ".to_string()),
            path: None,
            line: None,
        }),
        State(Arc::clone(&fixture.state)),
    )
    .await;

    let Err(error) = result else {
        panic!("expected missing-query definition resolve to fail");
    };

    assert_eq!(error.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "MISSING_QUERY");
}

#[tokio::test]
async fn search_definition_returns_best_payload() {
    let fixture = make_state_with_docs(vec![
        (
            "packages/rust/crates/demo/src/lib.rs",
            "pub fn build_service() {\n    let _service = AlphaService::new();\n}\n",
        ),
        (
            "packages/rust/crates/demo/src/service.rs",
            "pub struct AlphaService {\n    ready: bool,\n}\n",
        ),
        (
            "packages/rust/crates/other/src/service.rs",
            "pub struct AlphaService;\n",
        ),
    ]);

    let result = search_definition(
        Query(DefinitionResolveQuery {
            q: Some("AlphaService".to_string()),
            path: Some("packages/rust/crates/demo/src/lib.rs".to_string()),
            line: Some(2),
        }),
        State(fixture.state),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected definition resolve request to succeed");
    };

    assert_studio_json_snapshot(
        "search_definition_payload",
        json!({
            "query": response.0.query,
            "sourcePath": response.0.source_path,
            "sourceLine": response.0.source_line,
            "candidateCount": response.0.candidate_count,
            "selectedScope": response.0.selected_scope,
            "definition": {
                "name": response.0.definition.name,
                "signature": response.0.definition.signature,
                "path": response.0.definition.path,
                "language": response.0.definition.language,
                "crateName": response.0.definition.crate_name,
                "projectName": response.0.definition.project_name,
                "rootLabel": response.0.definition.root_label,
                "lineStart": response.0.definition.line_start,
                "lineEnd": response.0.definition.line_end,
                "score": round_f64(response.0.definition.score),
            },
        }),
    );
}

#[tokio::test]
async fn search_definition_accepts_absolute_source_paths() {
    let fixture = make_state_with_docs(vec![
        (
            "packages/rust/crates/demo/src/lib.rs",
            "pub fn build_service() {\n    let _service = AlphaService::new();\n}\n",
        ),
        (
            "packages/rust/crates/demo/src/service.rs",
            "pub struct AlphaService {\n    ready: bool,\n}\n",
        ),
        (
            "packages/rust/crates/other/src/service.rs",
            "pub struct AlphaService;\n",
        ),
    ]);
    let absolute_source_path = fixture
        .state
        .studio
        .project_root
        .join("packages/rust/crates/demo/src/lib.rs")
        .to_string_lossy()
        .to_string();

    let result = search_definition(
        Query(DefinitionResolveQuery {
            q: Some("AlphaService".to_string()),
            path: Some(absolute_source_path),
            line: Some(2),
        }),
        State(Arc::clone(&fixture.state)),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected definition resolve request to succeed");
    };

    assert_eq!(
        response.0.definition.path,
        "packages/rust/crates/demo/src/service.rs"
    );
}

#[tokio::test]
async fn search_references_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = search_references(
        Query(ReferenceSearchQuery {
            q: Some("   ".to_string()),
            limit: None,
        }),
        State(Arc::clone(&fixture.state)),
    )
    .await;

    let Err(error) = result else {
        panic!("expected missing-query reference search to fail");
    };

    assert_eq!(error.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "MISSING_QUERY");
}

#[tokio::test]
async fn search_references_returns_payload() {
    let fixture = make_state_with_docs(vec![
        (
            "packages/rust/crates/demo/src/lib.rs",
            "pub struct AlphaService {\n    ready: bool,\n}\n\npub fn alpha_handler() {\n    let _service = AlphaService { ready: true };\n}\n",
        ),
        (
            "packages/python/demo/tool.py",
            "class AlphaClient:\n    pass\n\ndef alpha_helper(client: AlphaClient):\n    return client\n",
        ),
    ]);

    let result = search_references(
        Query(ReferenceSearchQuery {
            q: Some("AlphaService".to_string()),
            limit: Some(10),
        }),
        State(fixture.state),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected reference search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_references_payload",
        json!({
            "query": response.0.query,
            "hitCount": response.0.hit_count,
            "selectedScope": response.0.selected_scope,
            "hits": response.0.hits.into_iter().map(|hit| {
                json!({
                    "name": hit.name,
                    "path": hit.path,
                    "language": hit.language,
                    "crateName": hit.crate_name,
                    "projectName": hit.project_name,
                    "rootLabel": hit.root_label,
                    "line": hit.line,
                    "column": hit.column,
                    "lineText": hit.line_text,
                    "score": round_f64(hit.score),
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_symbols_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = search_symbols(
        Query(SymbolSearchQuery {
            q: Some("   ".to_string()),
            limit: None,
        }),
        State(Arc::clone(&fixture.state)),
    )
    .await;

    let Err(error) = result else {
        panic!("expected missing-query symbol search to fail");
    };

    assert_eq!(error.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "MISSING_QUERY");
}

#[tokio::test]
async fn search_symbols_returns_payload() {
    let fixture = make_state_with_docs(vec![
        (
            "packages/rust/crates/demo/src/lib.rs",
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
        ),
        (
            "packages/python/demo/tool.py",
            "class AlphaClient:\n    pass\n\ndef alpha_helper():\n    return None\n",
        ),
        (
            "notes/ignored.md",
            "# alpha\n\nThis markdown file should not affect symbol search.\n",
        ),
    ]);

    let result = search_symbols(
        Query(SymbolSearchQuery {
            q: Some("alpha".to_string()),
            limit: Some(10),
        }),
        State(fixture.state),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected symbol search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_symbols_payload",
        json!({
            "query": response.0.query,
            "hitCount": response.0.hit_count,
            "selectedScope": response.0.selected_scope,
            "hits": response.0.hits.into_iter().map(|hit| {
                json!({
                    "name": hit.name,
                    "kind": hit.kind,
                    "path": hit.path,
                    "line": hit.line,
                    "location": hit.location,
                    "language": hit.language,
                    "crateName": hit.crate_name,
                    "projectName": hit.project_name,
                    "rootLabel": hit.root_label,
                    "source": hit.source,
                    "score": round_f64(hit.score),
                })
            }).collect::<Vec<_>>(),
        }),
    );
}
