//! Unit tests for sections module.

use std::collections::HashMap;

use super::*;

#[test]
fn test_parse_property_drawer_valid() {
    let line = ":ID: arch-v1";
    let result = parse_property_drawer(line);
    assert_eq!(result, Some(("ID".to_string(), "arch-v1".to_string())));
}

#[test]
fn test_parse_property_drawer_with_spaces() {
    let line = "  :TAGS: core, design  ";
    let result = parse_property_drawer(line);
    assert_eq!(
        result,
        Some(("TAGS".to_string(), "core, design".to_string()))
    );
}

#[test]
fn test_parse_property_drawer_no_leading_colon() {
    let line = "ID: arch-v1";
    let result = parse_property_drawer(line);
    assert!(result.is_none());
}

#[test]
fn test_parse_property_drawer_empty_value() {
    let line = ":ID:   ";
    let result = parse_property_drawer(line);
    assert!(result.is_none());
}

#[test]
fn test_extract_property_drawers_multiple() {
    let lines = vec![
        ":ID: test-123".to_string(),
        ":TAGS: one, two".to_string(),
        "".to_string(),
        "Content starts here".to_string(),
    ];
    let attrs = extract_property_drawers(&lines);
    assert_eq!(attrs.get("ID"), Some(&"test-123".to_string()));
    assert_eq!(attrs.get("TAGS"), Some(&"one, two".to_string()));
}

#[test]
fn test_extract_property_drawers_stops_at_content() {
    let lines = vec![
        ":ID: test-456".to_string(),
        "Not a property".to_string(),
        ":TAGS: ignored".to_string(),
    ];
    let attrs = extract_property_drawers(&lines);
    assert_eq!(attrs.get("ID"), Some(&"test-456".to_string()));
    assert!(attrs.get("TAGS").is_none()); // Should not be extracted
}

#[test]
fn test_extract_sections_with_property_drawer() {
    let body = r#"# Main Title
:ID: main-section
:TAGS: important

Content here.

## Subsection
:ID: sub-001

More content.
"#;
    let sections = extract_sections(
        body.as_ref(),
        std::path::Path::new("test.md"),
        std::path::Path::new("/"),
    );

    // First section should have :ID: main-section
    let first = sections.iter().find(|s| s.heading_title == "Main Title");
    assert!(first.is_some());
    let first = first.unwrap();
    assert_eq!(
        first.attributes.get("ID"),
        Some(&"main-section".to_string())
    );
    assert_eq!(first.attributes.get("TAGS"), Some(&"important".to_string()));

    // Subsection should have :ID: sub-001
    let sub = sections.iter().find(|s| s.heading_title == "Subsection");
    assert!(sub.is_some());
    let sub = sub.unwrap();
    assert_eq!(sub.attributes.get("ID"), Some(&"sub-001".to_string()));
}

#[test]
fn test_extract_property_drawers_org_block_format() {
    let lines = vec![
        ":PROPERTIES:".to_string(),
        ":ID:       uuid-v4-or-slug".to_string(),
        ":STATUS:   STABLE".to_string(),
        ":CONTRACT: must_contain(\"Rust\", \"Lock\")".to_string(),
        ":HASH:     blake3_fingerprint".to_string(),
        ":END:".to_string(),
        "".to_string(),
        "Content starts here".to_string(),
    ];
    let attrs = extract_property_drawers(&lines);
    assert_eq!(attrs.get("ID"), Some(&"uuid-v4-or-slug".to_string()));
    assert_eq!(attrs.get("STATUS"), Some(&"STABLE".to_string()));
    assert_eq!(
        attrs.get("CONTRACT"),
        Some(&"must_contain(\"Rust\", \"Lock\")".to_string())
    );
    assert_eq!(attrs.get("HASH"), Some(&"blake3_fingerprint".to_string()));
}

#[test]
fn test_extract_property_drawers_mixed_format() {
    // Test that block format and single-line format don't interfere
    let lines = vec![
        ":PROPERTIES:".to_string(),
        ":ID: block-id".to_string(),
        ":STATUS: DRAFT".to_string(),
        ":END:".to_string(),
        ":TAGS: ignored-after-end".to_string(), // Should NOT be extracted
    ];
    let attrs = extract_property_drawers(&lines);
    assert_eq!(attrs.get("ID"), Some(&"block-id".to_string()));
    assert_eq!(attrs.get("STATUS"), Some(&"DRAFT".to_string()));
    // TAGS should NOT be present because it comes after :END:
    assert!(attrs.get("TAGS").is_none());
}

#[test]
fn test_extract_sections_with_org_block_properties() {
    let body = r#"# Architecture Node
:PROPERTIES:
:ID:       arch-v1
:STATUS:   STABLE
:CONTRACT: must_contain("Rust", "Lock")
:HASH:     abc123def
:END:

This is the architecture section.

## Implementation
:PROPERTIES:
:ID:       impl-v1
:STATUS:   DRAFT
:END:

Implementation details here.
"#;
    let sections = extract_sections(
        body.as_ref(),
        std::path::Path::new("test.md"),
        std::path::Path::new("/"),
    );

    // First section should have all org block properties
    let arch = sections.iter().find(|s| s.heading_title == "Architecture Node");
    assert!(arch.is_some());
    let arch = arch.unwrap();
    assert_eq!(arch.attributes.get("ID"), Some(&"arch-v1".to_string()));
    assert_eq!(arch.attributes.get("STATUS"), Some(&"STABLE".to_string()));
    assert_eq!(
        arch.attributes.get("CONTRACT"),
        Some(&"must_contain(\"Rust\", \"Lock\")".to_string())
    );
    assert_eq!(arch.attributes.get("HASH"), Some(&"abc123def".to_string()));

    // Implementation section should have its own properties
    let impl_section = sections
        .iter()
        .find(|s| s.heading_title == "Implementation");
    assert!(impl_section.is_some());
    let impl_section = impl_section.unwrap();
    assert_eq!(impl_section.attributes.get("ID"), Some(&"impl-v1".to_string()));
    assert_eq!(impl_section.attributes.get("STATUS"), Some(&"DRAFT".to_string()));
}

#[test]
fn test_node_status_parsing() {
    assert_eq!(NodeStatus::from_str("STABLE"), NodeStatus::Stable);
    assert_eq!(NodeStatus::from_str("stable"), NodeStatus::Stable);
    assert_eq!(NodeStatus::from_str("  STABLE  "), NodeStatus::Stable);
    assert_eq!(NodeStatus::from_str("DRAFT"), NodeStatus::Draft);
    assert_eq!(NodeStatus::from_str("draft"), NodeStatus::Draft);
    assert_eq!(NodeStatus::from_str("DEPRECATED"), NodeStatus::Deprecated);
    assert_eq!(NodeStatus::from_str("deprecated"), NodeStatus::Deprecated);
    // Unknown values default to Stable
    assert_eq!(NodeStatus::from_str("UNKNOWN"), NodeStatus::Stable);
    assert_eq!(NodeStatus::from_str(""), NodeStatus::Stable);
}

// =========================================================================
// LOGBOOK Execution Drawer Tests (Blueprint v2.4)
// =========================================================================

#[test]
fn test_parse_logbook_entry_valid() {
    let line = "- [2025-03-14] Agent Started: Initiating structural audit.";
    let entry = parse_logbook_entry(line, 1);
    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.timestamp, "2025-03-14");
    assert_eq!(entry.message, "Agent Started: Initiating structural audit.");
    assert_eq!(entry.line_number, 1);
}

#[test]
fn test_parse_logbook_entry_with_brackets_in_message() {
    let line = "- [2025-03-14] Step [audit] completed with status OK.";
    let entry = parse_logbook_entry(line, 2);
    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.timestamp, "2025-03-14");
    assert_eq!(entry.message, "Step [audit] completed with status OK.");
}

#[test]
fn test_parse_logbook_entry_invalid_format() {
    // No list marker
    assert!(parse_logbook_entry("[2025-03-14] Message", 1).is_none());
    // No timestamp brackets
    assert!(parse_logbook_entry("- 2025-03-14 Message", 1).is_none());
    // Empty message
    assert!(parse_logbook_entry("- [2025-03-14] ", 1).is_none());
    // Empty timestamp
    assert!(parse_logbook_entry("- [] Message", 1).is_none());
}

#[test]
fn test_extract_logbook_entries_basic() {
    let lines = vec![
        ":LOGBOOK:".to_string(),
        "- [2025-03-14] Agent Started: Initiating structural audit.".to_string(),
        "- [2025-03-14] Step [audit] completed with status OK.".to_string(),
        ":END:".to_string(),
        "Content after logbook.".to_string(),
    ];
    let entries = extract_logbook_entries(&lines, 1);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].timestamp, "2025-03-14");
    assert_eq!(entries[0].message, "Agent Started: Initiating structural audit.");
    assert_eq!(entries[1].message, "Step [audit] completed with status OK.");
}

#[test]
fn test_extract_logbook_entries_empty() {
    let lines = vec![
        ":LOGBOOK:".to_string(),
        ":END:".to_string(),
    ];
    let entries = extract_logbook_entries(&lines, 1);
    assert!(entries.is_empty());
}

#[test]
fn test_extract_logbook_entries_no_block() {
    let lines = vec![
        "- [2025-03-14] This is not in a logbook block.".to_string(),
        "Just some content.".to_string(),
    ];
    let entries = extract_logbook_entries(&lines, 1);
    assert!(entries.is_empty());
}

#[test]
fn test_extract_sections_with_logbook() {
    let body = r#"# Task: Refactor Authentication
:PROPERTIES:
:ID:       task-auth-001
:STATUS:   RUNNING
:END:

:LOGBOOK:
- [2025-03-14] Agent Started: Initiating structural audit.
- [2025-03-14] Step [audit] completed with status OK.
:END:

Some task content here.
"#;
    let sections = extract_sections(
        body.as_ref(),
        std::path::Path::new("test.md"),
        std::path::Path::new("/"),
    );

    assert_eq!(sections.len(), 1);
    let section = &sections[0];

    // Check properties
    assert_eq!(section.attributes.get("ID"), Some(&"task-auth-001".to_string()));
    assert_eq!(section.attributes.get("STATUS"), Some(&"RUNNING".to_string()));

    // Check logbook entries
    assert_eq!(section.logbook.len(), 2);
    assert_eq!(section.logbook[0].timestamp, "2025-03-14");
    assert_eq!(section.logbook[0].message, "Agent Started: Initiating structural audit.");
    assert_eq!(section.logbook[1].message, "Step [audit] completed with status OK.");
}
