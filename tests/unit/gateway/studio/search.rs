use super::test_prelude::*;
use super::*;
use crate::gateway::studio::build_ast_index;
use crate::gateway::studio::repo_index::{RepoIndexEntryStatus, RepoIndexPhase, RepoIndexSnapshot};
use crate::gateway::studio::router::{GatewayState, StudioState};
use crate::gateway::studio::search::support::strip_option;
use crate::gateway::studio::test_support::{assert_studio_json_snapshot, round_f64};
use crate::gateway::studio::types::{UiConfig, UiProjectConfig, UiRepoProjectConfig};
use crate::search_plane::SearchPlaneService;
use serde_json::json;
use tempfile::tempdir;

struct StudioStateFixture {
    state: Arc<GatewayState>,
    temp_dir: tempfile::TempDir,
}

fn create_temp_dir() -> tempfile::TempDir {
    match tempdir() {
        Ok(temp_dir) => temp_dir,
        Err(err) => panic!("failed to create temp dir fixture: {err}"),
    }
}

fn ok_or_panic<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn write_doc(root: &std::path::Path, name: &str, content: &str) {
    let path = root.join(name);
    if let Some(parent) = path.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        panic!("failed to create fixture parent dirs for {name}: {err}");
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

    let mut studio_state = StudioState::new_with_bootstrap_ui_config(Arc::new(
        crate::analyzers::bootstrap_builtin_registry()
            .unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
    ));
    studio_state.project_root = temp_dir.path().to_path_buf();
    studio_state.config_root = temp_dir.path().to_path_buf();
    studio_state.search_plane = SearchPlaneService::new(temp_dir.path().to_path_buf());
    studio_state.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec![
                ".".to_string(),
                "packages".to_string(),
                ".data".to_string(),
                "internal_skills".to_string(),
            ],
        }],
        repo_projects: Vec::new(),
    });

    StudioStateFixture {
        state: Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio_state),
        }),
        temp_dir,
    }
}

async fn publish_local_symbol_index(state: &Arc<GatewayState>) {
    let projects = state.studio.configured_projects();
    let hits = build_ast_index(
        state.studio.project_root.as_path(),
        state.studio.config_root.as_path(),
        &projects,
    );
    let fingerprint = format!(
        "test:{}",
        blake3::hash(
            format!(
                "{}:{}:{}",
                state.studio.project_root.display(),
                state.studio.config_root.display(),
                hits.len()
            )
            .as_bytes()
        )
        .to_hex()
    );
    ok_or_panic(
        state
            .studio
            .search_plane
            .publish_local_symbol_hits(fingerprint.as_str(), &hits)
            .await,
        "publish local symbol epoch",
    );
}

async fn publish_reference_occurrence_index(state: &Arc<GatewayState>) {
    let projects = state.studio.configured_projects();
    let fingerprint = format!(
        "test:reference:{}",
        blake3::hash(
            format!(
                "{}:{}:{}",
                state.studio.project_root.display(),
                state.studio.config_root.display(),
                projects.len()
            )
            .as_bytes()
        )
        .to_hex()
    );
    ok_or_panic(
        state
            .studio
            .search_plane
            .publish_reference_occurrences_from_projects(
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                &projects,
                fingerprint.as_str(),
            )
            .await,
        "publish reference occurrence epoch",
    );
}

async fn publish_attachment_index(state: &Arc<GatewayState>) {
    let projects = state.studio.configured_projects();
    let fingerprint = format!(
        "test:attachment:{}",
        blake3::hash(
            format!(
                "{}:{}:{}",
                state.studio.project_root.display(),
                state.studio.config_root.display(),
                projects.len()
            )
            .as_bytes()
        )
        .to_hex()
    );
    ok_or_panic(
        state
            .studio
            .search_plane
            .publish_attachments_from_projects(
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                &projects,
                fingerprint.as_str(),
            )
            .await,
        "publish attachment epoch",
    );
}

async fn publish_knowledge_section_index(state: &Arc<GatewayState>) {
    let projects = state.studio.configured_projects();
    let fingerprint = format!(
        "test:knowledge:{}",
        blake3::hash(
            format!(
                "{}:{}:{}",
                state.studio.project_root.display(),
                state.studio.config_root.display(),
                projects.len()
            )
            .as_bytes()
        )
        .to_hex()
    );
    ok_or_panic(
        state
            .studio
            .search_plane
            .publish_knowledge_sections_from_projects(
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                &projects,
                fingerprint.as_str(),
            )
            .await,
        "publish knowledge section epoch",
    );
}

async fn publish_repo_content_chunk_index(
    state: &Arc<GatewayState>,
    repo_id: &str,
    documents: Vec<crate::gateway::studio::repo_index::RepoCodeDocument>,
) {
    ok_or_panic(
        state
            .studio
            .search_plane
            .publish_repo_content_chunks_with_revision(repo_id, &documents, None)
            .await,
        "publish repo content chunks",
    );
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

    let result = build_knowledge_search_response(
        fixture.state.studio.as_ref(),
        "   ",
        10,
        Some("semantic_lookup".to_string()),
    )
    .await;

    let Err(error) = result else {
        panic!("expected missing-query request to fail");
    };

    assert_eq!(error.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "MISSING_QUERY");
}

#[tokio::test]
async fn search_intent_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = load_intent_search_response_with_metadata(
        fixture.state.studio.as_ref(),
        SearchQuery {
            q: Some("   ".to_string()),
            intent: Some("debug_lookup".to_string()),
            limit: None,
            repo: None,
        },
    )
    .await;

    let Err(error) = result else {
        panic!("expected missing-query intent request to fail");
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
    publish_knowledge_section_index(&fixture.state).await;

    let result = build_knowledge_search_response(
        fixture.state.studio.as_ref(),
        "wendao",
        5,
        Some("semantic_lookup".to_string()),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_payload",
        json!({
            "query": response.query,
            "hitCount": response.hit_count,
            "selectedMode": response.selected_mode,
            "searchMode": response.search_mode,
            "intent": response.intent,
            "intentConfidence": response.intent_confidence.map(round_f64),
            "graphConfidenceScore": response.graph_confidence_score.map(round_f64),
            "hits": response.hits.into_iter().map(|hit| {
                json!({
                    "stem": hit.stem,
                    "title": hit.title,
                    "path": hit.path,
                    "docType": hit.doc_type,
                    "tags": hit.tags,
                    "score": round_f64(hit.score),
                    "bestSection": hit.best_section,
                    "matchReason": hit.match_reason,
                    "hierarchicalUri": hit.hierarchical_uri,
                    "hierarchy": hit.hierarchy,
                    "saliencyScore": hit.saliency_score.map(round_f64),
                    "auditStatus": hit.audit_status,
                    "verificationState": hit.verification_state,
                    "implicitBacklinks": hit.implicit_backlinks,
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_knowledge_waits_for_initial_index_publication() {
    let fixture = make_state_with_docs(vec![(
        "alpha.md",
        "# Alpha\n\nThis note contains search target keyword: wendao.\n",
    )]);

    let result = build_knowledge_search_response(
        fixture.state.studio.as_ref(),
        "wendao",
        5,
        Some("semantic_lookup".to_string()),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected cold-start knowledge search request to succeed");
    };

    assert!(response.hit_count >= 1);
}

#[tokio::test]
async fn search_intent_returns_payload() {
    let fixture = make_state_with_docs(vec![
        (
            "alpha.md",
            "# Alpha\n\nIntent search keyword: alpha_handler.\n",
        ),
        (
            "packages/rust/crates/demo/src/lib.rs",
            "pub fn alpha_handler() {}\n",
        ),
    ]);
    publish_knowledge_section_index(&fixture.state).await;
    publish_local_symbol_index(&fixture.state).await;

    let result = load_intent_search_response_with_metadata(
        fixture.state.studio.as_ref(),
        SearchQuery {
            q: Some("alpha_handler".to_string()),
            limit: Some(5),
            intent: Some("debug_lookup".to_string()),
            repo: None,
        },
    )
    .await;

    let Ok((response, _metadata)) = result else {
        panic!("expected intent search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_intent_payload",
        json!({
            "query": response.query,
            "hitCount": response.hit_count,
            "selectedMode": response.selected_mode,
            "searchMode": response.search_mode,
            "intent": response.intent,
            "intentConfidence": response.intent_confidence.map(round_f64),
            "graphConfidenceScore": response.graph_confidence_score.map(round_f64),
            "hits": response.hits.into_iter().map(|hit| {
                json!({
                    "stem": hit.stem,
                    "title": hit.title,
                    "path": hit.path,
                    "docType": hit.doc_type,
                    "score": round_f64(hit.score),
                    "bestSection": hit.best_section,
                    "matchReason": hit.match_reason,
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_intent_includes_repo_content_hits_for_code_biased_intent() {
    let fixture = make_state_with_docs(Vec::new());
    let repo_root = fixture.temp_dir.path().join("ValidPkg");
    std::fs::create_dir_all(repo_root.join("src"))
        .unwrap_or_else(|error| panic!("create repo src: {error}"));
    std::fs::write(
        repo_root.join("Project.toml"),
        "name = \"ValidPkg\"\nuuid = \"00000000-0000-0000-0000-000000000001\"\n",
    )
    .unwrap_or_else(|error| panic!("write project file: {error}"));

    fixture.state.studio.set_ui_config(UiConfig {
        projects: fixture.state.studio.configured_projects(),
        repo_projects: vec![UiRepoProjectConfig {
            id: "valid".to_string(),
            root: Some(repo_root.display().to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["julia".to_string()],
        }],
    });
    let snapshot = Arc::new(RepoIndexSnapshot {
        repo_id: "valid".to_string(),
        analysis: Arc::new(crate::analyzers::RepositoryAnalysisOutput::default()),
    });
    publish_repo_content_chunk_index(
        &fixture.state,
        "valid",
        vec![crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "src/ValidPkg.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from(
                "module ValidPkg\nusing Reexport\n@reexport using ModelingToolkit\nend\n",
            ),
            size_bytes: 62,
            modified_unix_ms: 0,
        }],
    )
    .await;
    fixture
        .state
        .studio
        .repo_index
        .set_snapshot_for_test(&snapshot);
    fixture
        .state
        .studio
        .repo_index
        .set_status_for_test(RepoIndexEntryStatus {
            repo_id: "valid".to_string(),
            phase: RepoIndexPhase::Ready,
            queue_position: None,
            last_error: None,
            last_revision: Some("abc123".to_string()),
            updated_at: Some("2026-03-22T00:00:00Z".to_string()),
            attempt_count: 1,
        });

    let result = load_intent_search_response_with_metadata(
        fixture.state.studio.as_ref(),
        SearchQuery {
            q: Some("lang:julia reexport".to_string()),
            limit: Some(5),
            intent: Some("debug_lookup".to_string()),
            repo: Some("valid".to_string()),
        },
    )
    .await;

    let Ok((response, _metadata)) = result else {
        panic!("expected repo-backed intent search request to succeed");
    };

    assert_eq!(response.selected_mode.as_deref(), Some("intent_hybrid"));
    assert!(
        response
            .hits
            .iter()
            .any(|hit| hit.doc_type.as_deref() == Some("file") && hit.path == "src/ValidPkg.jl"),
        "expected repo content hit in intent response: {:?}",
        response
            .hits
            .iter()
            .map(|hit| (&hit.path, &hit.doc_type))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn search_knowledge_uses_studio_display_paths() {
    let fixture = make_state_with_docs(vec![
        (
            "docs/alpha.md",
            "# Alpha\n\nThis note contains search target keyword: wendao.\n",
        ),
        (
            "docs/beta.md",
            "# Beta\n\nAnother note mentions wendao in text.\n",
        ),
    ]);
    publish_knowledge_section_index(&fixture.state).await;

    let result = build_knowledge_search_response(
        fixture.state.studio.as_ref(),
        "wendao",
        5,
        Some("semantic_lookup".to_string()),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected search request to succeed");
    };
    let hit_paths = response
        .hits
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<Vec<_>>();

    assert_studio_json_snapshot(
        "search_display_paths_payload",
        json!({
            "query": response.query,
            "hitCount": response.hit_count,
            "selectedMode": response.selected_mode,
            "paths": hit_paths.clone(),
        }),
    );

    if hit_paths.is_empty() {
        assert_eq!(response.selected_mode.as_deref(), Some("vector_only"));
        return;
    }

    assert!(
        hit_paths
            .iter()
            .all(|path| !std::path::Path::new(path).is_absolute()),
        "unexpected absolute hit paths: {hit_paths:?}",
    );
    assert!(
        hit_paths.iter().all(|path| !path.contains('\\')),
        "unexpected non-normalized hit paths: {hit_paths:?}",
    );
    assert!(
        hit_paths.iter().any(|path| path.ends_with("alpha.md")),
        "unexpected hit paths: {hit_paths:?}",
    );
}

#[tokio::test]
async fn search_knowledge_uses_project_scoped_display_paths_for_duplicate_roots() {
    let fixture = make_state_with_docs(vec![
        (
            "docs/kernel.md",
            "# Kernel\n\nThis note contains search target keyword: wendao.\n",
        ),
        (
            ".data/wendao-frontend/docs/main.md",
            "# Main\n\nThis note also contains search target keyword: wendao.\n",
        ),
    ]);
    fixture.state.studio.set_ui_config(UiConfig {
        projects: vec![
            UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            },
            UiProjectConfig {
                name: "main".to_string(),
                root: ".data/wendao-frontend".to_string(),
                dirs: vec!["docs".to_string()],
            },
        ],
        repo_projects: Vec::new(),
    });
    publish_knowledge_section_index(&fixture.state).await;

    let result = build_knowledge_search_response(
        fixture.state.studio.as_ref(),
        "wendao",
        10,
        Some("semantic_lookup".to_string()),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected project-scoped search request to succeed");
    };
    let hit_paths = response
        .hits
        .iter()
        .map(|hit| hit.path.as_str())
        .collect::<Vec<_>>();

    assert!(
        hit_paths.contains(&"kernel/docs/kernel.md"),
        "missing kernel project display path: {hit_paths:?}",
    );
    assert!(
        hit_paths.contains(&"main/docs/main.md"),
        "missing main project display path: {hit_paths:?}",
    );
}

#[tokio::test]
async fn search_attachments_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = load_attachment_search_response_from_studio(
        fixture.state.studio.as_ref(),
        AttachmentSearchQuery {
            q: Some("   ".to_string()),
            limit: None,
            ext: Vec::new(),
            kind: Vec::new(),
            case_sensitive: false,
        },
    )
    .await;

    let Err(error) = result else {
        panic!("expected missing-query attachment search to fail");
    };

    assert_eq!(error.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "MISSING_QUERY");
}

#[tokio::test]
async fn search_attachments_returns_payload() {
    let fixture = make_state_with_docs(vec![
        (
            "docs/alpha.md",
            "# Alpha\n\n![Topology](assets/topology.png)\n\n[Spec](files/spec.pdf)\n",
        ),
        ("docs/beta.md", "# Beta\n\n![Avatar](images/avatar.jpg)\n"),
    ]);
    fixture.state.studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });
    publish_attachment_index(&fixture.state).await;

    let result = load_attachment_search_response_from_studio(
        fixture.state.studio.as_ref(),
        AttachmentSearchQuery {
            q: Some("topology".to_string()),
            limit: Some(10),
            ext: Vec::new(),
            kind: Vec::new(),
            case_sensitive: false,
        },
    )
    .await;

    let Ok(response) = result else {
        panic!("expected attachment search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_attachments_payload",
        json!({
            "query": response.query,
            "hitCount": response.hit_count,
            "selectedScope": response.selected_scope,
            "hits": response.hits.into_iter().map(|hit| {
                json!({
                    "path": hit.path,
                    "sourceId": hit.source_id,
                    "sourceStem": hit.source_stem,
                    "sourceTitle": hit.source_title,
                    "sourcePath": hit.source_path,
                    "attachmentId": hit.attachment_id,
                    "attachmentPath": hit.attachment_path,
                    "attachmentName": hit.attachment_name,
                    "attachmentExt": hit.attachment_ext,
                    "kind": hit.kind,
                    "score": round_f64(hit.score),
                    "visionSnippet": hit.vision_snippet,
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_attachments_waits_for_initial_index_publication() {
    let fixture = make_state_with_docs(vec![(
        "docs/alpha.md",
        "# Alpha\n\n![Topology](assets/topology.png)\n\n[Spec](files/spec.pdf)\n",
    )]);
    fixture.state.studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    let result = load_attachment_search_response_from_studio(
        fixture.state.studio.as_ref(),
        AttachmentSearchQuery {
            q: Some("topology".to_string()),
            limit: Some(10),
            ext: Vec::new(),
            kind: Vec::new(),
            case_sensitive: false,
        },
    )
    .await;

    let Ok(response) = result else {
        panic!("expected cold-start attachment search request to succeed");
    };

    assert!(response.hit_count >= 1);
}

#[tokio::test]
async fn search_attachments_respects_extension_and_kind_filters() {
    let fixture = make_state_with_docs(vec![(
        "docs/alpha.md",
        "# Alpha\n\n![Topology](assets/topology.png)\n\n[Spec](files/spec.pdf)\n",
    )]);
    fixture.state.studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });
    publish_attachment_index(&fixture.state).await;

    let result = load_attachment_search_response_from_studio(
        fixture.state.studio.as_ref(),
        AttachmentSearchQuery {
            q: Some("spec".to_string()),
            limit: Some(10),
            ext: vec!["pdf".to_string()],
            kind: vec!["pdf".to_string()],
            case_sensitive: false,
        },
    )
    .await;

    let Ok(response) = result else {
        panic!("expected filtered attachment search request to succeed");
    };

    assert_eq!(response.hit_count, 1);
    assert_eq!(response.hits[0].attachment_name, "spec.pdf");
    assert_eq!(response.hits[0].attachment_ext, "pdf");
    assert_eq!(response.hits[0].kind, "pdf");
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
    publish_local_symbol_index(&fixture.state).await;

    let result = build_autocomplete_response(fixture.state.studio.as_ref(), "se", 2).await;

    let Ok(response) = result else {
        panic!("expected autocomplete request to succeed");
    };

    assert_studio_json_snapshot(
        "search_autocomplete_payload",
        json!({
            "prefix": response.prefix,
            "suggestions": response.suggestions.into_iter().map(|suggestion| {
                json!({
                    "text": suggestion.text,
                    "suggestionType": suggestion.suggestion_type,
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn autocomplete_includes_code_symbols() {
    let fixture = make_state_with_docs(vec![
        (
            "packages/rust/crates/demo/src/lib.rs",
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
        ),
        (
            "packages/python/demo/tool.py",
            "class AlphaClient:\n    pass\n\ndef alpha_helper():\n    return None\n",
        ),
    ]);
    publish_local_symbol_index(&fixture.state).await;

    let result = build_autocomplete_response(fixture.state.studio.as_ref(), "al", 10).await;

    let Ok(response) = result else {
        panic!("expected code-symbol autocomplete request to succeed");
    };

    let suggestions = response
        .suggestions
        .into_iter()
        .map(|suggestion| (suggestion.text, suggestion.suggestion_type))
        .collect::<Vec<_>>();

    assert_eq!(
        suggestions,
        vec![
            ("AlphaClient".to_string(), "symbol".to_string()),
            ("AlphaService".to_string(), "symbol".to_string()),
            ("alpha_handler".to_string(), "symbol".to_string()),
            ("alpha_helper".to_string(), "symbol".to_string()),
        ]
    );
}

#[tokio::test]
async fn autocomplete_waits_for_initial_index_publication() {
    let fixture = make_state_with_docs(vec![(
        "packages/rust/crates/demo/src/lib.rs",
        "pub struct AlphaService;\npub fn alpha_handler() {}\n",
    )]);

    let result = build_autocomplete_response(fixture.state.studio.as_ref(), "al", 10).await;

    let Ok(response) = result else {
        panic!("expected cold-start autocomplete request to succeed");
    };

    assert!(
        response
            .suggestions
            .iter()
            .any(|suggestion| suggestion.text == "AlphaService")
    );
}

#[tokio::test]
async fn search_ast_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = search_ast(
        State(Arc::clone(&fixture.state)),
        Query(AstSearchQuery {
            q: Some("   ".to_string()),
            limit: None,
        }),
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
            "notes/ignored.txt",
            "alpha should stay outside AST search fixtures.\n",
        ),
    ]);
    publish_local_symbol_index(&fixture.state).await;

    let result = search_ast(
        State(fixture.state),
        Query(AstSearchQuery {
            q: Some("alpha".to_string()),
            limit: Some(10),
        }),
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
                    "nodeKind": hit.node_kind,
                    "ownerTitle": hit.owner_title,
                    "navigationTarget": {
                        "path": hit.navigation_target.path,
                        "category": hit.navigation_target.category,
                        "projectName": hit.navigation_target.project_name,
                        "rootLabel": hit.navigation_target.root_label,
                        "line": hit.navigation_target.line,
                        "lineEnd": hit.navigation_target.line_end,
                        "column": hit.navigation_target.column,
                    },
                    "lineStart": hit.line_start,
                    "lineEnd": hit.line_end,
                    "score": round_f64(hit.score),
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_ast_includes_markdown_outline_hits() {
    let fixture = make_state_with_docs(vec![(
        "docs/03_features/204_gateway_api_contracts.md",
        "# Gateway API Contracts\n\n## AST Search\n\n- [ ] Verify docs AST alignment.\n",
    )]);
    fixture.state.studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });
    publish_local_symbol_index(&fixture.state).await;

    let result = search_ast(
        State(Arc::clone(&fixture.state)),
        Query(AstSearchQuery {
            q: Some("ast".to_string()),
            limit: Some(10),
        }),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected markdown AST search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_ast_markdown_payload",
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
                    "nodeKind": hit.node_kind,
                    "ownerTitle": hit.owner_title,
                    "navigationTarget": {
                        "path": hit.navigation_target.path,
                        "category": hit.navigation_target.category,
                        "projectName": hit.navigation_target.project_name,
                        "rootLabel": hit.navigation_target.root_label,
                        "line": hit.navigation_target.line,
                        "lineEnd": hit.navigation_target.line_end,
                        "column": hit.navigation_target.column,
                    },
                    "lineStart": hit.line_start,
                    "lineEnd": hit.line_end,
                    "score": round_f64(hit.score),
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_ast_includes_markdown_property_drawer_hits() {
    let fixture = make_state_with_docs(vec![(
        "docs/index.md",
        "# Studio Functional Ledger\n:PROPERTIES:\n:ID: SearchBarProtocol\n:OBSERVE: lang:typescript scope:\"src/components/SearchBar/**\" \"export const SearchBar: React.FC<SearchBarProps> = ({ $$$ })\"\n:END:\n\n## Runtime Contract\n",
    )]);
    fixture.state.studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "main".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });
    publish_local_symbol_index(&fixture.state).await;

    let result = search_ast(
        State(Arc::clone(&fixture.state)),
        Query(AstSearchQuery {
            q: Some("SearchBar".to_string()),
            limit: Some(10),
        }),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected markdown property AST search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_ast_markdown_property_payload",
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
                    "nodeKind": hit.node_kind,
                    "ownerTitle": hit.owner_title,
                    "navigationTarget": {
                        "path": hit.navigation_target.path,
                        "category": hit.navigation_target.category,
                        "projectName": hit.navigation_target.project_name,
                        "rootLabel": hit.navigation_target.root_label,
                        "line": hit.navigation_target.line,
                        "lineEnd": hit.navigation_target.line_end,
                        "column": hit.navigation_target.column,
                    },
                    "lineStart": hit.line_start,
                    "lineEnd": hit.line_end,
                    "score": round_f64(hit.score),
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_ast_waits_for_initial_index_publication() {
    let fixture = make_state_with_docs(vec![(
        "packages/rust/crates/demo/src/lib.rs",
        "pub struct AlphaService {\n    ready: bool,\n}\n\npub fn alpha_handler() {}\n",
    )]);

    let result = search_ast(
        State(fixture.state),
        Query(AstSearchQuery {
            q: Some("alpha".to_string()),
            limit: Some(10),
        }),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected cold-start AST search request to succeed");
    };

    assert!(response.0.hit_count >= 1);
}

#[tokio::test]
async fn search_definition_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = build_definition_response(fixture.state.studio.as_ref(), "   ", None, None).await;

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
    publish_local_symbol_index(&fixture.state).await;

    let result = build_definition_response(
        fixture.state.studio.as_ref(),
        "AlphaService",
        Some("packages/rust/crates/demo/src/lib.rs"),
        Some(2),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected definition resolve request to succeed");
    };

    assert_studio_json_snapshot(
        "search_definition_payload",
        json!({
            "query": response.query,
            "sourcePath": response.source_path,
            "sourceLine": response.source_line,
            "candidateCount": response.candidate_count,
            "selectedScope": response.selected_scope,
            "navigationTarget": {
                "path": response.navigation_target.path,
                "category": response.navigation_target.category,
                "projectName": response.navigation_target.project_name,
                "rootLabel": response.navigation_target.root_label,
                "line": response.navigation_target.line,
                "lineEnd": response.navigation_target.line_end,
                "column": response.navigation_target.column,
            },
            "definition": {
                "name": response.definition.name,
                "signature": response.definition.signature,
                "path": response.definition.path,
                "language": response.definition.language,
                "crateName": response.definition.crate_name,
                "projectName": response.definition.project_name,
                "rootLabel": response.definition.root_label,
                "lineStart": response.definition.line_start,
                "lineEnd": response.definition.line_end,
                "score": round_f64(response.definition.score),
            },
        }),
    );
}

#[tokio::test]
async fn search_definition_waits_for_initial_index_publication() {
    let fixture = make_state_with_docs(vec![
        (
            "packages/rust/crates/demo/src/lib.rs",
            "pub fn build_service() {\n    let _service = AlphaService::new();\n}\n",
        ),
        (
            "packages/rust/crates/demo/src/service.rs",
            "pub struct AlphaService {\n    ready: bool,\n}\n",
        ),
    ]);

    let result = build_definition_response(
        fixture.state.studio.as_ref(),
        "AlphaService",
        Some("packages/rust/crates/demo/src/lib.rs"),
        Some(2),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected cold-start definition resolve request to succeed");
    };

    assert_eq!(response.definition.name, "AlphaService");
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
    publish_local_symbol_index(&fixture.state).await;
    let absolute_source_path = fixture
        .state
        .studio
        .project_root
        .join("packages/rust/crates/demo/src/lib.rs")
        .to_string_lossy()
        .to_string();

    let result = build_definition_response(
        fixture.state.studio.as_ref(),
        "AlphaService",
        Some(absolute_source_path.as_str()),
        Some(2),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected definition resolve request to succeed");
    };

    assert_eq!(
        response.definition.path,
        "packages/rust/crates/demo/src/service.rs"
    );
}

#[tokio::test]
async fn search_definition_uses_markdown_observe_hints() {
    let fixture = make_state_with_docs(vec![
        (
            "packages/notes/index.md",
            "# Index\n\n:PROPERTIES:\n:OBSERVE: lang:python scope:\"packages/python/demo/**\" \"AlphaService\"\n:END:\n",
        ),
        (
            "packages/rust/crates/demo/src/service.rs",
            "pub struct AlphaService;\n",
        ),
        (
            "packages/python/demo/service.py",
            "class AlphaService:\n    pass\n",
        ),
    ]);
    publish_local_symbol_index(&fixture.state).await;

    let result = build_definition_response(
        fixture.state.studio.as_ref(),
        "AlphaService",
        Some("packages/notes/index.md"),
        Some(4),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected markdown-observe definition resolve request to succeed");
    };

    assert_studio_json_snapshot(
        "search_definition_markdown_observe_hint_payload",
        json!({
            "query": response.query,
            "sourcePath": response.source_path,
            "sourceLine": response.source_line,
            "candidateCount": response.candidate_count,
            "selectedScope": response.selected_scope,
            "navigationTarget": {
                "path": response.navigation_target.path,
                "category": response.navigation_target.category,
                "projectName": response.navigation_target.project_name,
                "rootLabel": response.navigation_target.root_label,
                "line": response.navigation_target.line,
                "lineEnd": response.navigation_target.line_end,
                "column": response.navigation_target.column,
            },
            "definition": {
                "name": response.definition.name,
                "signature": response.definition.signature,
                "path": response.definition.path,
                "language": response.definition.language,
                "crateName": response.definition.crate_name,
                "projectName": response.definition.project_name,
                "rootLabel": response.definition.root_label,
                "lineStart": response.definition.line_start,
                "lineEnd": response.definition.line_end,
                "score": round_f64(response.definition.score),
            },
        }),
    );
}

#[tokio::test]
async fn search_definition_falls_back_to_fuzzy_symbol_match() {
    let fixture = make_state_with_docs(vec![
        (
            "packages/rust/crates/demo/src/lib.rs",
            "pub fn build_service() {\n    let _service = AlphaService::new();\n}\n",
        ),
        (
            "packages/rust/crates/demo/src/service.rs",
            "pub struct AlphaService {\n    ready: bool,\n}\n",
        ),
    ]);
    publish_local_symbol_index(&fixture.state).await;

    let result = build_definition_response(
        fixture.state.studio.as_ref(),
        "AlphaServic",
        Some("packages/rust/crates/demo/src/lib.rs"),
        Some(2),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected fuzzy definition resolve request to succeed");
    };

    assert_eq!(
        response.definition.path,
        "packages/rust/crates/demo/src/service.rs"
    );
    assert!(response.candidate_count >= 1);
}

#[tokio::test]
async fn search_definition_can_resolve_markdown_heading_hits() {
    let fixture = make_state_with_docs(vec![(
        "packages/notes/guide.md",
        "# AlphaService Guide\n\nThis note explains the service.\n",
    )]);
    publish_local_symbol_index(&fixture.state).await;

    let result = build_definition_response(
        fixture.state.studio.as_ref(),
        "AlphaService Guide",
        Some("packages/notes/guide.md"),
        Some(1),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected markdown-backed definition resolve request to succeed");
    };

    assert_eq!(response.definition.language, "markdown");
    assert_eq!(response.definition.path, "packages/notes/guide.md");
}

#[tokio::test]
async fn search_references_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = load_reference_search_response(
        fixture.state.as_ref(),
        ReferenceSearchQuery {
            q: Some("   ".to_string()),
            limit: None,
        },
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
    publish_reference_occurrence_index(&fixture.state).await;

    let result = load_reference_search_response(
        fixture.state.as_ref(),
        ReferenceSearchQuery {
            q: Some("AlphaService".to_string()),
            limit: Some(10),
        },
    )
    .await;

    let Ok(response) = result else {
        panic!("expected reference search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_references_payload",
        json!({
            "query": response.query,
            "hitCount": response.hit_count,
            "selectedScope": response.selected_scope,
            "hits": response.hits.into_iter().map(|hit| {
                json!({
                    "name": hit.name,
                    "path": hit.path,
                    "language": hit.language,
                    "crateName": hit.crate_name,
                    "projectName": hit.project_name,
                    "rootLabel": hit.root_label,
                    "navigationTarget": {
                        "path": hit.navigation_target.path,
                        "category": hit.navigation_target.category,
                        "projectName": hit.navigation_target.project_name,
                        "rootLabel": hit.navigation_target.root_label,
                        "line": hit.navigation_target.line,
                        "lineEnd": hit.navigation_target.line_end,
                        "column": hit.navigation_target.column,
                    },
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
async fn search_references_waits_for_initial_index_publication() {
    let fixture = make_state_with_docs(vec![(
        "packages/rust/crates/demo/src/lib.rs",
        "pub struct AlphaService {\n    ready: bool,\n}\n\npub fn alpha_handler() {\n    let _service = AlphaService { ready: true };\n}\n",
    )]);

    let result = load_reference_search_response(
        fixture.state.as_ref(),
        ReferenceSearchQuery {
            q: Some("AlphaService".to_string()),
            limit: Some(10),
        },
    )
    .await;

    let Ok(response) = result else {
        panic!("expected cold-start reference search request to succeed");
    };

    assert!(response.hit_count >= 1);
}

#[tokio::test]
async fn search_symbols_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = load_symbol_search_response(
        fixture.state.as_ref(),
        SymbolSearchQuery {
            q: Some("   ".to_string()),
            limit: None,
        },
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
    let warmed_index = xiuxian_wendao::gateway::studio::search::build_symbol_index(
        fixture.state.studio.project_root.as_path(),
        fixture.state.studio.config_root.as_path(),
        fixture.state.studio.configured_projects().as_slice(),
    );
    fixture
        .state
        .studio
        .symbol_index_coordinator
        .set_ready_index_for_test(
            fixture.state.studio.configured_projects().as_slice(),
            Arc::clone(&fixture.state.studio.symbol_index),
            warmed_index,
        );

    let result = load_symbol_search_response(
        fixture.state.as_ref(),
        SymbolSearchQuery {
            q: Some("alpha".to_string()),
            limit: Some(10),
        },
    )
    .await;

    let Ok(response) = result else {
        panic!("expected symbol search request to succeed");
    };

    assert_studio_json_snapshot(
        "search_symbols_payload",
        json!({
            "query": response.query,
            "hitCount": response.hit_count,
            "selectedScope": response.selected_scope,
            "partial": response.partial,
            "indexingState": response.indexing_state,
            "hits": response.hits.into_iter().map(|hit| {
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
                    "navigationTarget": {
                        "path": hit.navigation_target.path,
                        "category": hit.navigation_target.category,
                        "projectName": hit.navigation_target.project_name,
                        "rootLabel": hit.navigation_target.root_label,
                        "line": hit.navigation_target.line,
                        "lineEnd": hit.navigation_target.line_end,
                        "column": hit.navigation_target.column,
                    },
                    "source": hit.source,
                    "score": round_f64(hit.score),
                })
            }).collect::<Vec<_>>(),
        }),
    );
}

#[tokio::test]
async fn search_symbols_returns_pending_payload_while_index_is_warming() {
    let fixture = make_state_with_docs(vec![(
        "packages/rust/crates/demo/src/lib.rs",
        "pub struct PendingSymbolIndex;\n",
    )]);
    let projects = fixture.state.studio.configured_projects();
    fixture
        .state
        .studio
        .symbol_index_coordinator
        .set_status_for_test(
            projects.as_slice(),
            xiuxian_wendao::gateway::studio::symbol_index::SymbolIndexStatus {
                phase: xiuxian_wendao::gateway::studio::symbol_index::SymbolIndexPhase::Indexing,
                last_error: None,
                updated_at: Some("2026-03-21T00:00:00Z".to_string()),
            },
        );

    let result = load_symbol_search_response(
        fixture.state.as_ref(),
        SymbolSearchQuery {
            q: Some("pending".to_string()),
            limit: Some(10),
        },
    )
    .await;

    let Ok(response) = result else {
        panic!("expected pending symbol search request to succeed");
    };

    assert_eq!(response.hit_count, 0);
    assert!(response.partial);
    assert_eq!(response.indexing_state.as_deref(), Some("indexing"));
    assert!(response.hits.is_empty());
}

#[tokio::test]
async fn search_symbols_respects_glob_dir_filters() {
    let fixture = make_state_with_docs(vec![
        (
            "packages/alpha/src/lib.rs",
            "pub struct GlobFilteredSymbol;\npub fn alpha_glob_symbol() {}\n",
        ),
        (
            "packages/beta/src/lib.rs",
            "pub struct GlobFilteredSymbol;\npub fn beta_glob_symbol() {}\n",
        ),
    ]);

    fixture.state.studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["packages".to_string(), "packages/alpha/**/*.rs".to_string()],
        }],
        repo_projects: Vec::new(),
    });
    let warmed_index = xiuxian_wendao::gateway::studio::search::build_symbol_index(
        fixture.state.studio.project_root.as_path(),
        fixture.state.studio.config_root.as_path(),
        fixture.state.studio.configured_projects().as_slice(),
    );
    fixture
        .state
        .studio
        .symbol_index_coordinator
        .set_ready_index_for_test(
            fixture.state.studio.configured_projects().as_slice(),
            Arc::clone(&fixture.state.studio.symbol_index),
            warmed_index,
        );

    let result = load_symbol_search_response(
        fixture.state.as_ref(),
        SymbolSearchQuery {
            q: Some("GlobFilteredSymbol".to_string()),
            limit: Some(10),
        },
    )
    .await;

    let Ok(response) = result else {
        panic!("expected glob-filtered symbol search to succeed");
    };

    let hit_paths = response
        .hits
        .iter()
        .map(|hit| hit.path.as_str())
        .collect::<Vec<_>>();
    assert!(!hit_paths.is_empty());
    assert!(
        hit_paths
            .iter()
            .all(|path| path.starts_with("packages/alpha/")),
        "unexpected glob-filtered hit paths: {hit_paths:?}",
    );
}

#[test]
fn repo_navigation_target_prefixes_repo_root_for_relative_paths() {
    let target = repo_navigation_target("mcl", "Modelica/package.mo", None, None, None);
    assert_eq!(target.path, "mcl/Modelica/package.mo");
    assert_eq!(target.category, "repo_code");
    assert_eq!(target.project_name.as_deref(), Some("mcl"));
    assert_eq!(target.root_label.as_deref(), Some("mcl"));
}

#[test]
fn repo_navigation_target_does_not_duplicate_existing_repo_root_prefix() {
    let target = repo_navigation_target("mcl", "mcl/Modelica/package.mo", None, None, None);
    assert_eq!(target.path, "mcl/Modelica/package.mo");
}

#[test]
fn parse_content_search_line_parses_ripgrep_output() {
    let parsed = parse_content_search_line(
        "/tmp/repo/src/DifferentialEquations.jl:42:@reexport using SciMLBase",
    );
    let Some((path, line_number, snippet)) = parsed else {
        panic!("expected ripgrep output to parse");
    };

    assert_eq!(path, "/tmp/repo/src/DifferentialEquations.jl");
    assert_eq!(line_number, 42);
    assert_eq!(snippet, "@reexport using SciMLBase");
}

#[test]
fn supported_code_extension_includes_julia_and_modelica() {
    assert!(is_supported_code_extension("src/Foo.jl"));
    assert!(is_supported_code_extension("Modelica/package.mo"));
    assert!(!is_supported_code_extension("docs/readme.md"));
}

#[test]
fn truncate_content_search_snippet_limits_output_length() {
    let value = "abcdefghijklmnopqrstuvwxyz";
    let truncated = truncate_content_search_snippet(value, 8);
    assert_eq!(truncated, "abcdefgh...");
}

#[test]
fn code_content_globs_do_not_exclude_cache_root() {
    assert!(!CODE_CONTENT_EXCLUDE_GLOBS.contains(&"!.cache/**"));
}

#[test]
fn parse_repo_code_search_query_extracts_lang_kind_and_term() {
    let spec = parse_repo_code_search_query("lang:julia kind:file reexport");
    assert_eq!(spec.search_term(), Some("reexport"));
    assert!(spec.language_filters.contains("julia"));
    assert!(spec.kind_filters.contains("file"));
}

#[test]
fn parse_repo_code_search_query_keeps_unknown_kind_token_in_search_term() {
    let spec = parse_repo_code_search_query("kind:custom reexport");
    assert_eq!(spec.search_term(), Some("kind:custom reexport"));
}

#[test]
fn language_filter_matches_julia_path_extensions() {
    let mut filters = std::collections::HashSet::new();
    filters.insert("julia".to_string());

    assert!(path_matches_language_filters(
        "src/BaseModelica.jl",
        &filters
    ));
    assert!(path_matches_language_filters(
        "src/generated/parser.julia",
        &filters
    ));
    assert!(!path_matches_language_filters("docs/index.md", &filters));
}
