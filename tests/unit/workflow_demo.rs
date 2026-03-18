//! Integration tests for Workflow Demo (Blueprint v2.4).
//!
//! Tests that the LOGBOOK execution drawer parsing works correctly
//! with real workflow documents.

use std::path::Path;

use xiuxian_wendao::link_graph::parser::{LogbookEntry, parse_note};

#[test]
fn test_workflow_demo_logbook_parsing() {
    let workflow_content = include_str!("../fixtures/workflow_demo/input/workflow.md");
    let root = Path::new("/");
    let path = Path::new("workflow.md");

    let parsed = parse_note(path, root, workflow_content).expect("Should parse workflow");

    // Find the main task section
    let task_section = parsed
        .sections
        .iter()
        .find(|s| s.heading_title == "Task: Refactor Authentication Logic")
        .expect("Should find task section");

    // Verify property drawer
    assert_eq!(
        task_section.attributes.get("ID"),
        Some(&"task-auth-001".to_string())
    );
    assert_eq!(
        task_section.attributes.get("STATUS"),
        Some(&"RUNNING".to_string())
    );
    assert_eq!(
        task_section.attributes.get("WORKFLOW"),
        Some(&"auth_refactor_dag".to_string())
    );

    // Verify logbook entries
    assert_eq!(
        task_section.logbook.len(),
        5,
        "Should have 5 logbook entries"
    );

    // Check first entry
    let first_entry = &task_section.logbook[0];
    assert_eq!(first_entry.timestamp, "2026-03-15");
    assert_eq!(
        first_entry.message,
        "Agent Started: Initiating structural audit."
    );

    // Check last entry
    let last_entry = task_section.logbook.last().unwrap();
    assert_eq!(last_entry.message, "Step [test] All 47 tests passed.");
}

#[test]
fn test_workflow_demo_entities_extracted() {
    let workflow_content = include_str!("../fixtures/workflow_demo/input/workflow.md");
    let root = Path::new("/");
    let path = Path::new("workflow.md");

    let parsed = parse_note(path, root, workflow_content).expect("Should parse workflow");

    // Check that wiki links are extracted
    let all_entities: Vec<&String> = parsed
        .sections
        .iter()
        .flat_map(|s| s.entities.iter())
        .collect();

    // Should have extracted the wiki links
    assert!(all_entities.iter().any(|e| e.contains("config-module")));
    assert!(all_entities.iter().any(|e| e.contains("db-schema-v2")));
    assert!(all_entities.iter().any(|e| e.contains("api-design-spec")));
}

#[test]
fn test_logbook_entry_with_nested_brackets() {
    let workflow_content = include_str!("../fixtures/workflow_demo/input/workflow.md");
    let root = Path::new("/");
    let path = Path::new("workflow.md");

    let parsed = parse_note(path, root, workflow_content).expect("Should parse workflow");

    let task_section = parsed
        .sections
        .iter()
        .find(|s| s.heading_title == "Task: Refactor Authentication Logic")
        .expect("Should find task section");

    // Find entry with nested brackets like "Step [audit] Found..."
    let audit_entry = task_section
        .logbook
        .iter()
        .find(|e| e.message.contains("[audit]"));
    assert!(audit_entry.is_some());

    let entry = audit_entry.unwrap();
    assert_eq!(entry.timestamp, "2026-03-15");
    assert_eq!(
        entry.message,
        "Step [audit] Found 3 files requiring updates."
    );
}
