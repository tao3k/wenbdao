//! Unit tests for `semantic_check` module (Blueprint v2.2).

use super::*;
use crate::link_graph::parser::CodeObservation;
use crate::link_graph::{PageIndexMeta, PageIndexNode};
use std::sync::Arc;

// =============================================================================
// ID Reference Extraction Tests
// =============================================================================

#[test]
fn test_extract_id_references() {
    let text = "See [[#intro]] and [[#architecture]] for details.";
    let refs = extract_id_references(text);
    assert_eq!(refs, vec!["#intro", "#architecture"]);
}

#[test]
fn test_extract_id_references_no_match() {
    let text = "No wiki links here, just [[regular-link]] text.";
    let refs = extract_id_references(text);
    assert!(refs.is_empty());
}

// =============================================================================
// Hash Reference Extraction Tests
// =============================================================================

#[test]
fn test_extract_hash_references_with_hash() {
    let text = "See [[#arch-v1@abc123]] for the architecture.";
    let refs = extract_hash_references(text);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].target_id, "arch-v1");
    assert_eq!(refs[0].expect_hash, Some("abc123".to_string()));
}

#[test]
fn test_extract_hash_references_without_hash() {
    let text = "See [[#intro]] for the introduction.";
    let refs = extract_hash_references(text);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].target_id, "intro");
    assert_eq!(refs[0].expect_hash, None);
}

#[test]
fn test_extract_hash_references_mixed() {
    let text = "See [[#arch-v1@abc123]] and [[#intro]] and [[#config@def456]].";
    let refs = extract_hash_references(text);
    assert_eq!(refs.len(), 3);
    assert_eq!(refs[0].target_id, "arch-v1");
    assert_eq!(refs[0].expect_hash, Some("abc123".to_string()));
    assert_eq!(refs[1].target_id, "intro");
    assert_eq!(refs[1].expect_hash, None);
    assert_eq!(refs[2].target_id, "config");
    assert_eq!(refs[2].expect_hash, Some("def456".to_string()));
}

#[test]
fn test_extract_hash_references_empty() {
    let text = "No hash-annotated references here.";
    let refs = extract_hash_references(text);
    assert!(refs.is_empty());
}

// =============================================================================
// Contract Validation Tests
// =============================================================================

#[test]
fn test_validate_contract_must_contain() {
    let content = "This document describes Rust and Lock mechanisms.";
    assert!(validate_contract("must_contain(\"Rust\", \"Lock\")", content).is_none());
    assert!(validate_contract("must_contain(\"Python\")", content).is_some());
}

#[test]
fn test_validate_contract_must_not_contain() {
    let content = "This is a stable API.";
    assert!(validate_contract("must_not_contain(\"deprecated\")", content).is_none());
    assert!(validate_contract("must_not_contain(\"stable\")", content).is_some());
}

#[test]
fn test_validate_contract_min_length() {
    let content = "Short";
    assert!(validate_contract("min_length(3)", content).is_none());
    assert!(validate_contract("min_length(100)", content).is_some());
}

#[test]
fn test_extract_function_args() {
    assert_eq!(
        extract_function_args("must_contain(\"Rust\", \"Lock\")", "must_contain"),
        Some("\"Rust\", \"Lock\"")
    );
    assert_eq!(
        extract_function_args("min_length(100)", "min_length"),
        Some("100")
    );
    assert_eq!(extract_function_args("unknown()", "must_contain"), None);
}

// =============================================================================
// Helper Function Tests
// =============================================================================

#[test]
fn test_xml_escape() {
    assert_eq!(xml_escape("a<b>c&d"), "a&lt;b&gt;c&amp;d");
    assert_eq!(xml_escape("\"quoted\""), "&quot;quoted&quot;");
}

#[test]
fn test_node_status_parsing() {
    assert_eq!(NodeStatus::parse_lossy("STABLE"), NodeStatus::Stable);
    assert_eq!(NodeStatus::parse_lossy("stable"), NodeStatus::Stable);
    assert_eq!(NodeStatus::parse_lossy("DRAFT"), NodeStatus::Draft);
    assert_eq!(
        NodeStatus::parse_lossy("DEPRECATED"),
        NodeStatus::Deprecated
    );
    assert_eq!(NodeStatus::parse_lossy("UNKNOWN"), NodeStatus::Stable);
}

#[test]
fn test_generate_suggested_id() {
    assert_eq!(
        generate_suggested_id("Architecture Overview"),
        "architecture-overview"
    );
    assert_eq!(generate_suggested_id("API Reference!"), "api-reference");
    assert_eq!(generate_suggested_id("  Test  "), "test");
}

#[test]
fn test_issue_type_to_code() {
    assert_eq!(issue_type_to_code("dead_link"), "ERR_DEAD_LINK");
    assert_eq!(issue_type_to_code("deprecated_ref"), "WARN_DEPRECATED_REF");
    assert_eq!(
        issue_type_to_code("contract_violation"),
        "ERR_CONTRACT_VIOLATION"
    );
    assert_eq!(issue_type_to_code("id_collision"), "ERR_DUPLICATE_ID");
    assert_eq!(
        issue_type_to_code("missing_identity"),
        "ERR_MISSING_IDENTITY"
    );
    assert_eq!(issue_type_to_code("legacy_syntax"), "WARN_LEGACY_SYNTAX");
    assert_eq!(
        issue_type_to_code("invalid_observation_pattern"),
        "ERR_INVALID_OBSERVER_PATTERN"
    );
    assert_eq!(
        issue_type_to_code("doc_identity_protocol"),
        "ERR_DOC_IDENTITY_PROTOCOL"
    );
    assert_eq!(
        issue_type_to_code("missing_package_docs_tree"),
        "WARN_MISSING_PACKAGE_DOCS_TREE"
    );
    assert_eq!(
        issue_type_to_code("missing_package_docs_index"),
        "ERR_MISSING_PACKAGE_DOCS_INDEX"
    );
    assert_eq!(
        issue_type_to_code("missing_package_docs_section_landing"),
        "WARN_MISSING_PACKAGE_DOCS_SECTION"
    );
    assert_eq!(
        issue_type_to_code("missing_package_docs_index_section_link"),
        "WARN_MISSING_PACKAGE_DOCS_INDEX_LINK"
    );
    assert_eq!(
        issue_type_to_code("missing_package_docs_index_relations_block"),
        "WARN_MISSING_PACKAGE_DOCS_RELATIONS_BLOCK"
    );
    assert_eq!(
        issue_type_to_code("missing_package_docs_index_footer_block"),
        "WARN_MISSING_PACKAGE_DOCS_FOOTER_BLOCK"
    );
    assert_eq!(
        issue_type_to_code("incomplete_package_docs_index_footer_block"),
        "WARN_INCOMPLETE_PACKAGE_DOCS_FOOTER_BLOCK"
    );
    assert_eq!(
        issue_type_to_code("stale_package_docs_index_footer_standards"),
        "WARN_STALE_PACKAGE_DOCS_FOOTER_STANDARDS"
    );
    assert_eq!(
        issue_type_to_code("missing_package_docs_index_relation_link"),
        "WARN_MISSING_PACKAGE_DOCS_RELATION_LINK"
    );
    assert_eq!(
        issue_type_to_code("stale_package_docs_index_relation_link"),
        "WARN_STALE_PACKAGE_DOCS_RELATION_LINK"
    );
    assert_eq!(issue_type_to_code("unknown"), "UNKNOWN");
}

// =============================================================================
// Health Score Tests
// =============================================================================

#[test]
fn test_build_file_reports() {
    let issues = vec![
        SemanticIssue {
            severity: "error".to_string(),
            issue_type: "dead_link".to_string(),
            doc: "doc1.md".to_string(),
            node_id: "node1".to_string(),
            message: "Test error".to_string(),
            location: None,
            suggestion: None,
            fuzzy_suggestion: None,
        },
        SemanticIssue {
            severity: "warning".to_string(),
            issue_type: "legacy_syntax".to_string(),
            doc: "doc1.md".to_string(),
            node_id: "node2".to_string(),
            message: "Test warning".to_string(),
            location: None,
            suggestion: None,
            fuzzy_suggestion: None,
        },
        SemanticIssue {
            severity: "error".to_string(),
            issue_type: "dead_link".to_string(),
            doc: "doc2.md".to_string(),
            node_id: "node3".to_string(),
            message: "Another error".to_string(),
            location: None,
            suggestion: None,
            fuzzy_suggestion: None,
        },
    ];

    let docs = vec!["doc1.md".to_string(), "doc2.md".to_string()];
    let reports = build_file_reports(&issues, &docs);

    assert_eq!(reports.len(), 2);

    // doc1.md: 1 error, 1 warning -> 100 - 20 - 5 = 75
    assert_eq!(reports[0].path, "doc1.md");
    assert_eq!(reports[0].error_count, 1);
    assert_eq!(reports[0].warning_count, 1);
    assert_eq!(reports[0].health_score, 75);

    // doc2.md: 1 error, 0 warnings -> 100 - 20 = 80
    assert_eq!(reports[1].path, "doc2.md");
    assert_eq!(reports[1].error_count, 1);
    assert_eq!(reports[1].warning_count, 0);
    assert_eq!(reports[1].health_score, 80);
}

#[test]
fn test_build_file_reports_deduplicates_alias_doc_paths() {
    let cwd = std::env::current_dir().unwrap_or_else(|error| panic!("cwd: {error}"));
    let temp = tempfile::tempdir_in(&cwd).unwrap_or_else(|error| panic!("tempdir: {error}"));
    let doc_path = temp.path().join("docs/index.md");
    let parent = doc_path
        .parent()
        .unwrap_or_else(|| panic!("parent directory should exist"));
    std::fs::create_dir_all(parent).unwrap_or_else(|error| panic!("create dir: {error}"));
    std::fs::write(&doc_path, "# Demo\n").unwrap_or_else(|error| panic!("write doc: {error}"));

    let absolute_path = doc_path
        .canonicalize()
        .unwrap_or_else(|error| panic!("canonicalize: {error}"))
        .to_string_lossy()
        .to_string();
    let relative_path = doc_path
        .strip_prefix(&cwd)
        .unwrap_or_else(|error| panic!("strip prefix: {error}"))
        .to_string_lossy()
        .to_string();

    let issues = vec![SemanticIssue {
        severity: "warning".to_string(),
        issue_type: "doc_identity_protocol".to_string(),
        doc: absolute_path.clone(),
        node_id: absolute_path.clone(),
        message: "Alias path warning".to_string(),
        location: None,
        suggestion: None,
        fuzzy_suggestion: None,
    }];

    let docs = vec![relative_path.clone(), absolute_path];
    let reports = build_file_reports(&issues, &docs);

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].path, relative_path);
    assert_eq!(reports[0].warning_count, 1);
    assert_eq!(reports[0].error_count, 0);
}

#[test]
fn test_health_score_bounds() {
    // Test that health score doesn't go below 0
    let issues: Vec<SemanticIssue> = (0..10)
        .map(|_| SemanticIssue {
            severity: "error".to_string(),
            issue_type: "dead_link".to_string(),
            doc: "doc.md".to_string(),
            node_id: "node".to_string(),
            message: "Error".to_string(),
            location: None,
            suggestion: None,
            fuzzy_suggestion: None,
        })
        .collect();

    let docs = vec!["doc.md".to_string()];
    let reports = build_file_reports(&issues, &docs);

    // 10 errors * 20 = 200 penalty, but score should be 0 (not negative)
    assert_eq!(reports[0].health_score, 0);
}

// =============================================================================
// Code Observation Check Tests (Blueprint v2.7)
// =============================================================================

/// Helper to create a `PageIndexNode` with observations.
fn create_node_with_observations(
    node_id: &str,
    observations: Vec<CodeObservation>,
) -> PageIndexNode {
    PageIndexNode {
        node_id: node_id.to_string(),
        parent_id: None,
        title: "Test Node".to_string(),
        level: 1,
        text: Arc::from(""),
        summary: None,
        children: Vec::new(),
        blocks: Vec::new(),
        metadata: PageIndexMeta {
            line_range: (1, 10),
            byte_range: Some((0, 100)),
            structural_path: vec!["Test".to_string()],
            content_hash: Some("abc123".to_string()),
            attributes: std::collections::HashMap::new(),
            token_count: 10,
            is_thinned: false,
            logbook: Vec::new(),
            observations,
        },
    }
}

fn parse_observation(raw: &str) -> CodeObservation {
    let Some(observation) = CodeObservation::parse(raw) else {
        panic!("expected test observation to parse: {raw}");
    };
    observation
}

#[test]
fn test_check_code_observations_valid_pattern() {
    // Create a valid Rust observation
    let obs = parse_observation(r#"lang:rust "fn $NAME($$$) -> Result<$$$>""#);
    let node = create_node_with_observations("test.md#valid", vec![obs]);

    let mut issues = Vec::new();
    check_code_observations(&node, "test.md", &[], None, &mut issues);

    // Should not report any issues for a valid pattern
    assert!(issues.is_empty());
}

#[test]
fn test_check_code_observations_unsupported_language() {
    // Create an observation with unsupported language
    let obs = parse_observation(r#"lang:brainfuck "+-<>""#);
    let node = create_node_with_observations("test.md#unsupported", vec![obs]);

    let mut issues = Vec::new();
    check_code_observations(&node, "test.md", &[], None, &mut issues);

    // Should report unsupported language error
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].issue_type, "invalid_observation_language");
    assert!(issues[0].message.contains("Unsupported language"));
    assert!(issues[0].message.contains("brainfuck"));
}

// Note: Invalid pattern tests are covered by snapshot tests in tests/snapshots/

#[test]
fn test_check_code_observations_multiple_issues() {
    // Create multiple observations with various issues
    let obs1 = parse_observation(r#"lang:rust "fn $NAME()""#); // valid
    let obs2 = parse_observation(r#"lang:brainfuck "+-<>""#); // unsupported
    let obs3 = parse_observation(r#"lang:python "def $NAME():""#); // valid

    let node = create_node_with_observations("test.md#mixed", vec![obs1, obs2, obs3]);

    let mut issues = Vec::new();
    check_code_observations(&node, "test.md", &[], None, &mut issues);

    // Should report only one issue (unsupported language)
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].issue_type, "invalid_observation_language");
}

#[test]
fn test_check_code_observations_no_observations() {
    // Create a node without observations
    let node = create_node_with_observations("test.md#none", Vec::new());

    let mut issues = Vec::new();
    check_code_observations(&node, "test.md", &[], None, &mut issues);

    // Should not report any issues
    assert!(issues.is_empty());
}

#[test]
fn test_check_code_observations_python_valid() {
    // Test valid Python pattern
    let obs = parse_observation(r#"lang:python "def $NAME($$$): $$$BODY""#);
    let node = create_node_with_observations("test.md#python", vec![obs]);

    let mut issues = Vec::new();
    check_code_observations(&node, "test.md", &[], None, &mut issues);

    assert!(issues.is_empty());
}

#[test]
fn test_check_code_observations_typescript_valid() {
    // Test valid TypeScript pattern
    let obs = parse_observation(r#"lang:typescript "function $NAME($$$): $$$RET""#);
    let node = create_node_with_observations("test.md#ts", vec![obs]);

    let mut issues = Vec::new();
    check_code_observations(&node, "test.md", &[], None, &mut issues);

    assert!(issues.is_empty());
}

// =============================================================================
// Fuzzy Suggestion Tests (Blueprint v2.9)
// =============================================================================

#[test]
fn test_check_code_observations_with_fuzzy_suggestion() {
    // Create an observation with a pattern that won't match
    // (The pattern is syntactically valid but has no matches in source files)
    let obs = parse_observation(r#"lang:rust "fn nonexistent_function($$$)""#);
    let node = create_node_with_observations("test.md#fuzzy", vec![obs]);

    // Create a source file with a similar function
    let source = SourceFile {
        path: "src/lib.rs".to_string(),
        content: "fn existing_function(x: i32) -> i32 { x + 1 }".to_string(),
    };

    let mut issues = Vec::new();
    check_code_observations(&node, "test.md", &[source], None, &mut issues);

    // The pattern is valid but finds no matches, so a warning is issued
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity, "warning");
    assert_eq!(issues[0].issue_type, "observation_target_missing");
    // The fuzzy suggestion should find the similar function
    assert!(issues[0].fuzzy_suggestion.is_some());
    let Some(fuzzy) = issues[0].fuzzy_suggestion.as_ref() else {
        panic!("expected fuzzy suggestion data for missing observation target");
    };
    assert!(fuzzy.suggested_pattern.contains("existing_function"));
}
