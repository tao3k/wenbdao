use super::*;
use crate::zhenfa_router::native::semantic_check::{
    FuzzySuggestionData, IssueLocation, SemanticIssue,
};

fn test_file_content() -> String {
    "line 1\n:OBSERVE: lang:rust \"fn process_data\"\nline 3".to_string()
}

/// Get the correct byte range for the OBSERVE line in `test_file_content`.
fn observe_line_range() -> ByteRange {
    // Test content: "line 1\n:OBSERVE: lang:rust \"fn process_data\"\nline 3"
    // Let's calculate byte positions:
    // Position 0-6: "line 1\n" (7 bytes)
    // Position 7-44: ":OBSERVE: lang:rust \"fn process_data\"" (38 bytes)
    // Position 45: '\n'
    // So to get just the OBSERVE without newline: (7, 45)
    let content = test_file_content();
    let observe_content = r#":OBSERVE: lang:rust "fn process_data""#;
    let Some(start) = content.find(observe_content) else {
        panic!("OBSERVE content should exist");
    };
    let end = start + observe_content.len();
    ByteRange::new(start, end)
}

#[test]
fn test_batch_fix_from_fuzzy_suggestion() {
    let suggestion = FuzzySuggestionData {
        original_pattern: "fn process_data($$$)".to_string(),
        suggested_pattern: "fn process_records($$$)".to_string(),
        confidence: 0.85,
        source_location: Some("src/lib.rs:42".to_string()),
        replacement_drawer: r#":OBSERVE: lang:rust "fn process_records($$$)""#.to_string(),
    };

    let fix = BatchFix::from_fuzzy_suggestion(
        "docs/api.md".to_string(),
        42,
        r#":OBSERVE: lang:rust "fn process_data($$$)""#.to_string(),
        &suggestion,
    );

    assert_eq!(fix.issue_type, "invalid_observation_pattern");
    assert_eq!(fix.doc_path, "docs/api.md");
    assert_eq!(fix.line_number, 42);
    assert!(!fix.is_surgical());
}

#[test]
fn test_batch_fix_surgical() {
    let content = test_file_content();
    let base_hash = compute_hash(&content);
    let byte_range = observe_line_range();

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        2,
        byte_range,
        base_hash,
        r#":OBSERVE: lang:rust "fn process_data""#.to_string(),
        r#":OBSERVE: lang:rust "fn process_records""#.to_string(),
        0.9,
    );

    assert!(fix.is_surgical());
    assert!(fix.byte_range.is_some());
    assert!(fix.base_hash.is_some());
}

#[test]
fn test_surgical_fix_apply_success() {
    let content = test_file_content();
    let base_hash = compute_hash(&content);
    let byte_range = observe_line_range();

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        2,
        byte_range,
        base_hash,
        r#":OBSERVE: lang:rust "fn process_data""#.to_string(),
        r#":OBSERVE: lang:rust "fn process_records""#.to_string(),
        0.9,
    );

    let mut content = test_file_content();
    let result = fix.apply_surgical(&mut content);

    assert_eq!(result, FixResult::Success);
    assert!(content.contains("process_records"));
}

#[test]
fn test_surgical_fix_content_at_range_mismatch() {
    // v3.1: Hash verification is now external (one-time in AtomicFixBatch)
    // This test verifies that content mismatch at byte range is detected
    let content = test_file_content();
    let base_hash = compute_hash(&content);
    let byte_range = ByteRange::new(0, 3);

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        1,
        byte_range,
        base_hash,
        "old".to_string(), // Wrong - actual content starts with "lin"
        "new".to_string(),
        0.9,
    );

    let mut content = test_file_content();
    let result = fix.apply_surgical(&mut content);

    // Content at range (0, 3) is "lin", not "old" -> ContentMismatch
    assert!(matches!(result, FixResult::ContentMismatch { .. }));
}

#[test]
fn test_surgical_fix_out_of_bounds() {
    let content = test_file_content();
    let base_hash = compute_hash(&content);
    let byte_range = ByteRange::new(100, 200); // Out of bounds

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        2,
        byte_range,
        base_hash,
        "something".to_string(),
        "replacement".to_string(),
        0.9,
    );

    let mut content = test_file_content();
    let result = fix.apply_surgical(&mut content);

    assert!(matches!(result, FixResult::OutOfBounds { .. }));
}

#[test]
fn test_surgical_fix_content_mismatch() {
    let content = test_file_content();
    let base_hash = compute_hash(&content);
    let byte_range = observe_line_range();

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        2,
        byte_range,
        base_hash,
        "wrong original content".to_string(), // Doesn't match actual content at range
        "replacement".to_string(),
        0.9,
    );

    let mut content = test_file_content();
    let result = fix.apply_surgical(&mut content);

    assert!(matches!(result, FixResult::ContentMismatch { .. }));
}

#[test]
fn test_legacy_fallback() {
    let fix = BatchFix::new(
        "test".to_string(),
        "test.md".to_string(),
        1,
        "old content".to_string(),
        "new content".to_string(),
        0.9,
    );

    let mut content = "line 1\nold content\nline 3".to_string();
    let result = fix.apply_surgical(&mut content);

    assert_eq!(result, FixResult::Success);
    assert_eq!(content, "line 1\nnew content\nline 3");
}

#[test]
fn test_preview() {
    let content = test_file_content();
    let base_hash = compute_hash(&content);
    let byte_range = observe_line_range();

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        2,
        byte_range,
        base_hash,
        r#":OBSERVE: lang:rust "fn process_data""#.to_string(),
        r#":OBSERVE: lang:rust "fn process_records""#.to_string(),
        0.9,
    );

    let original = test_file_content();
    let preview = match fix.preview(&original) {
        Ok(preview) => preview,
        Err(error) => panic!("preview should succeed: {error:?}"),
    };

    assert!(preview.contains("process_records"));
    assert!(!original.contains("process_records")); // Original unchanged
}

#[test]
fn test_compute_hash_deterministic() {
    let content = "test content";
    let hash1 = compute_hash(content);
    let hash2 = compute_hash(content);

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // Blake3 hex length
}

#[test]
fn test_with_surgical() {
    let content = test_file_content();
    let base_hash = compute_hash(&content);

    let fix = BatchFix::new(
        "test".to_string(),
        "test.md".to_string(),
        2,
        "old".to_string(),
        "new".to_string(),
        0.9,
    )
    .with_surgical(ByteRange::new(7, 43), base_hash);

    assert!(fix.is_surgical());
}

#[test]
fn test_generate_surgical_fixes() {
    let doc_path = "docs/api.md".to_string();
    let file_content = "line 1\n:OBSERVE: lang:rust \"fn process_data\"\nline 3".to_string();

    let mut file_contents = std::collections::HashMap::new();
    file_contents.insert(doc_path.clone(), file_content.clone());

    let issues = vec![SemanticIssue {
        severity: "error".to_string(),
        issue_type: "invalid_observation_pattern".to_string(),
        doc: doc_path.clone(),
        node_id: "node1".to_string(),
        message: "Invalid pattern".to_string(),
        location: Some(IssueLocation {
            line: 2,
            heading_path: "API".to_string(),
            byte_range: Some((7, 43)),
        }),
        suggestion: Some(":OBSERVE: lang:rust \"fn process_data\"".to_string()),
        fuzzy_suggestion: Some(FuzzySuggestionData {
            original_pattern: "fn process_data".to_string(),
            suggested_pattern: "fn process_records($$$)".to_string(),
            confidence: 0.85,
            source_location: Some("src/lib.rs:42".to_string()),
            replacement_drawer: r#":OBSERVE: lang:rust "fn process_records($$$)""#.to_string(),
        }),
    }];

    let fixes = generate_surgical_fixes(&issues, &file_contents);

    assert_eq!(fixes.len(), 1);
    assert!(fixes[0].is_surgical());
    assert!(fixes[0].base_hash.is_some());
    assert_eq!(fixes[0].byte_range, Some(ByteRange::new(7, 45))); // Full line including newline
}

// =============================================================================
// FixResult Display Tests
// =============================================================================

#[test]
fn test_fix_result_display_success() {
    let result = FixResult::Success;
    assert_eq!(format!("{result}"), "Fix applied successfully");
}

#[test]
fn test_fix_result_display_hash_mismatch() {
    let result = FixResult::HashMismatch {
        expected: "a1b2c3d4e5f6".to_string(),
        actual: "x1y2z3a4b5c6".to_string(),
    };
    let display = format!("{result}");
    assert!(display.contains("Hash mismatch"));
    assert!(display.contains("a1b2c3d4"));
    assert!(display.contains("x1y2z3a4"));
}

#[test]
fn test_fix_result_display_out_of_bounds() {
    let result = FixResult::OutOfBounds {
        range: ByteRange::new(100, 200),
        file_size: 50,
    };
    let display = format!("{result}");
    assert!(display.contains("Byte range"));
    assert!(display.contains("exceeds file size"));
    assert!(display.contains("50"));
}

#[test]
fn test_fix_result_display_content_mismatch() {
    let result = FixResult::ContentMismatch {
        expected: "expected content".to_string(),
        actual: "actual content".to_string(),
    };
    let display = format!("{result}");
    assert!(display.contains("Content mismatch"));
    assert!(display.contains("expected"));
    assert!(display.contains("actual"));
}

// =============================================================================
// AuditBridge Trait Tests
// =============================================================================

#[test]
fn test_default_audit_bridge_generate_fixes() {
    let bridge = DefaultAuditBridge;

    let issues = vec![
        SemanticIssue {
            severity: "error".to_string(),
            issue_type: "invalid_observation_pattern".to_string(),
            doc: "docs/api.md".to_string(),
            node_id: "node1".to_string(),
            message: "Invalid pattern".to_string(),
            location: Some(IssueLocation {
                line: 42,
                heading_path: "API".to_string(),
                byte_range: None,
            }),
            suggestion: Some("Fix it".to_string()),
            fuzzy_suggestion: Some(FuzzySuggestionData {
                original_pattern: "fn process_data".to_string(),
                suggested_pattern: "fn process_records($$$)".to_string(),
                confidence: 0.85,
                source_location: Some("src/lib.rs:42".to_string()),
                replacement_drawer: r#":OBSERVE: lang:rust "fn process_records($$$)""#.to_string(),
            }),
        },
        // Issue without fuzzy_suggestion - should be filtered out
        SemanticIssue {
            severity: "error".to_string(),
            issue_type: "dead_link".to_string(),
            doc: "docs/other.md".to_string(),
            node_id: "node2".to_string(),
            message: "Dead link".to_string(),
            location: None,
            suggestion: None,
            fuzzy_suggestion: None,
        },
    ];

    let fixes = bridge.generate_fixes(&issues);

    // Only one fix should be generated (for the issue with fuzzy_suggestion)
    assert_eq!(fixes.len(), 1);
    assert_eq!(fixes[0].doc_path, "docs/api.md");
}

#[test]
fn test_generate_batch_fixes_function() {
    let issues = vec![SemanticIssue {
        severity: "error".to_string(),
        issue_type: "invalid_observation_pattern".to_string(),
        doc: "docs/api.md".to_string(),
        node_id: "node1".to_string(),
        message: "Invalid pattern".to_string(),
        location: None,
        suggestion: Some("Fix it".to_string()),
        fuzzy_suggestion: Some(FuzzySuggestionData {
            original_pattern: "fn process_data".to_string(),
            suggested_pattern: "fn process_records($$$)".to_string(),
            confidence: 0.85,
            source_location: Some("src/lib.rs:42".to_string()),
            replacement_drawer: r#":OBSERVE: lang:rust "fn process_records($$$)""#.to_string(),
        }),
    }];

    let fixes = generate_batch_fixes(&issues);
    assert_eq!(fixes.len(), 1);
}

// =============================================================================
// Edge Cases and Boundary Tests
// =============================================================================

#[test]
fn test_surgical_fix_empty_content() {
    let content = "";
    let base_hash = compute_hash(content);
    let byte_range = ByteRange::new(0, 0);

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        1,
        byte_range,
        base_hash,
        String::new(),
        "new content".to_string(),
        0.9,
    );

    let mut content = String::new();
    let result = fix.apply_surgical(&mut content);

    assert_eq!(result, FixResult::Success);
    assert_eq!(content, "new content");
}

#[test]
fn test_surgical_fix_same_content_replacement() {
    let content = test_file_content();
    let base_hash = compute_hash(&content);
    let byte_range = observe_line_range();
    let observe_content = r#":OBSERVE: lang:rust "fn process_data""#;

    // Replace with same content
    let fix = BatchFix::surgical(
        "test.md".to_string(),
        2,
        byte_range,
        base_hash,
        observe_content.to_string(),
        observe_content.to_string(), // Same as original
        0.9,
    );

    let mut content = test_file_content();
    let result = fix.apply_surgical(&mut content);

    assert_eq!(result, FixResult::Success);
    // Content should be unchanged
    assert_eq!(content, test_file_content());
}

#[test]
fn test_surgical_fix_byte_range_at_file_boundary() {
    let content = "test content";
    let base_hash = compute_hash(content);
    let byte_range = ByteRange::new(0, 12); // Exact file length

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        1,
        byte_range,
        base_hash,
        "test content".to_string(),
        "replaced all".to_string(),
        0.9,
    );

    let mut content = "test content".to_string();
    let result = fix.apply_surgical(&mut content);

    assert_eq!(result, FixResult::Success);
    assert_eq!(content, "replaced all");
}

#[test]
fn test_surgical_fix_start_equals_end() {
    let content = test_file_content();
    let base_hash = compute_hash(&content);
    let byte_range = ByteRange::new(7, 7); // Zero-width range

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        2,
        byte_range,
        base_hash,
        String::new(), // Empty original
        "inserted".to_string(),
        0.9,
    );

    let mut content = test_file_content();
    let result = fix.apply_surgical(&mut content);

    // Should succeed - inserting at position 7
    assert_eq!(result, FixResult::Success);
    assert!(content.contains("inserted"));
}

#[test]
fn test_legacy_fallback_not_found() {
    let fix = BatchFix::new(
        "test".to_string(),
        "test.md".to_string(),
        1,
        "nonexistent".to_string(),
        "new".to_string(),
        0.9,
    );

    let mut content = "some other content".to_string();
    let result = fix.apply_surgical(&mut content);

    assert!(matches!(result, FixResult::ContentMismatch { .. }));
}

#[test]
fn test_preview_error() {
    // v3.1: Hash verification is now external (one-time in AtomicFixBatch)
    // This test verifies that content mismatch is detected during preview
    let content = "different content";
    let base_hash = compute_hash(content);

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        1,
        ByteRange::new(0, 3),
        base_hash,
        "old".to_string(), // Wrong - actual content at (0, 3) is "dif"
        "new".to_string(),
        0.9,
    );

    let result = fix.preview(content);

    match result {
        Err(FixResult::ContentMismatch { .. }) => {}
        Err(other) => panic!("unexpected preview error: {other:?}"),
        Ok(preview) => panic!("preview should fail, got: {preview}"),
    }
}

#[test]
fn test_surgical_fix_multibyte_utf8() {
    // Test with UTF-8 content containing multibyte characters
    let content = "line 1\n:OBSERVE: lang:rust \"fn 处理数据\"\nline 3";
    let base_hash = compute_hash(content);

    // Find the byte position of the OBSERVE line
    let observe = r#":OBSERVE: lang:rust "fn 处理数据""#;
    let Some(start) = content.find(observe) else {
        panic!("expected to find multibyte observation");
    };
    let end = start + observe.len();

    let fix = BatchFix::surgical(
        "test.md".to_string(),
        2,
        ByteRange::new(start, end),
        base_hash,
        observe.to_string(),
        r#":OBSERVE: lang:rust "fn process_data""#.to_string(),
        0.9,
    );

    let mut content = content.to_string();
    let result = fix.apply_surgical(&mut content);

    assert_eq!(result, FixResult::Success);
    assert!(content.contains("process_data"));
}

#[test]
fn test_is_surgical_method() {
    // Non-surgical fix
    let non_surgical = BatchFix::new(
        "test".to_string(),
        "test.md".to_string(),
        1,
        "old".to_string(),
        "new".to_string(),
        0.9,
    );
    assert!(!non_surgical.is_surgical());

    // Surgical fix
    let surgical = BatchFix::surgical(
        "test.md".to_string(),
        1,
        ByteRange::new(0, 3),
        "hash".to_string(),
        "old".to_string(),
        "new".to_string(),
        0.9,
    );
    assert!(surgical.is_surgical());

    // Partial surgical (only byte_range, no base_hash)
    let partial = BatchFix {
        issue_type: "test".to_string(),
        doc_path: "test.md".to_string(),
        line_number: 1,
        original_content: "old".to_string(),
        replacement: "new".to_string(),
        confidence: 0.9,
        source_location: None,
        base_hash: None,
        byte_range: Some(ByteRange::new(0, 3)),
    };
    assert!(!partial.is_surgical()); // Both fields required
}
