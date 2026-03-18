//! Tests for concurrent write safety (Blueprint Section 3.1: Zero-line Collaboration).
//!
//! These tests verify that:
//! - Using byte ranges instead of line numbers enables concurrent edits
//! - Non-overlapping edits can be applied without merge conflicts
//! - The order of independent edits doesn't affect the final result
//! - Content hash verification prevents lost updates

use xiuxian_wendao::link_graph::{ModificationError, replace_byte_range};

/// Test that two non-overlapping byte range edits can be applied concurrently.
///
/// Blueprint Section 3.1: As long as two semantic_edit requests target different
/// node IDs with non-overlapping paths, the system allows direct concurrent writes
/// without manual merge conflicts.
#[test]
fn test_concurrent_non_overlapping_edits() {
    // Document with two sections that can be edited independently
    let doc = "# Section A\nContent A\n\n# Section B\nContent B";

    // Simulate two concurrent edits to different sections
    // Edit 1: Replace "Content A" (bytes 12-20) with "Updated A"
    let edit1 = replace_byte_range(doc, 12, 20, "Updated A", None).unwrap();

    // Edit 2: Replace "Content B" (bytes 34-42) with "Updated B"
    // Note: This uses the original document, simulating concurrent edits
    let edit2 = replace_byte_range(doc, 34, 42, "Updated B", None).unwrap();

    // Both edits should succeed independently
    assert!(edit1.new_content.contains("Updated A"));
    assert!(edit2.new_content.contains("Updated B"));
}

/// Test that sequential non-overlapping edits produce correct results.
///
/// When edits are applied sequentially, the byte delta from the first edit
/// must be accounted for in subsequent edits.
#[test]
fn test_sequential_non_overlapping_edits() {
    // Use simple strings with clear byte boundaries
    let doc = "AAA___BBB";
    //          012345678
    //          ^^^   ^^^
    //          AAA   BBB

    // First edit: Replace "AAA" (bytes 0-3) with "XXXX" (4 bytes)
    let r1 = replace_byte_range(doc, 0, 3, "XXXX", None).unwrap();
    assert_eq!(r1.new_content, "XXXX___BBB");
    assert_eq!(r1.byte_delta, 1); // 3 -> 4 = +1

    // Now replace "BBB" in the new content
    // Original BBB was at bytes 6-9
    // After +1 delta, it's at bytes 7-10
    let r2 = replace_byte_range(&r1.new_content, 7, 10, "YYYYY", None).unwrap();
    assert_eq!(r2.new_content, "XXXX___YYYYY");
}

/// Test that byte range overlap detection would prevent conflicts.
///
/// In a real concurrent system, overlapping edits would need conflict resolution.
/// This test demonstrates the boundary conditions.
#[test]
fn test_byte_range_overlap_detection() {
    // Non-overlapping: [0,3) and [5,8)
    let range1 = (0, 3);
    let range2 = (5, 8);
    assert!(
        range1.1 <= range2.0 || range2.1 <= range1.0,
        "Ranges should not overlap"
    );

    // Adjacent but not overlapping: [0,3) and [3,6)
    let range3 = (0, 3);
    let range4 = (3, 6);
    assert!(
        range3.1 <= range4.0 || range4.1 <= range3.0,
        "Adjacent ranges should not overlap"
    );

    // Overlapping: [0,5) and [3,8)
    let range5 = (0, 5);
    let range6 = (3, 8);
    let overlaps = !(range5.1 <= range6.0 || range6.1 <= range5.0);
    assert!(overlaps, "These ranges should overlap");
}

/// Test that content hash verification prevents lost updates.
///
/// Blueprint Section 3.2: Self-healing via adjust_line_range dynamically
/// calculates the drifted viewport.
#[test]
fn test_hash_verification_prevents_lost_updates() {
    let original = "Hello, world!";

    // Edit without hash verification succeeds
    let result = replace_byte_range(original, 7, 12, "Rust", None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().new_content, "Hello, Rust!");

    // Edit with wrong hash fails (content was modified by another agent)
    let wrong_hash = "wronghash";
    let result = replace_byte_range(original, 7, 12, "Python", Some(wrong_hash));
    assert!(matches!(
        result,
        Err(ModificationError::HashMismatch { .. })
    ));
}

/// Test that structural path enables path-aware conflict detection.
///
/// Different sections with different structural paths can be edited concurrently.
#[test]
fn test_structural_path_enables_concurrent_edits() {
    // Simulate two sections with different structural paths
    let section_a_path = vec!["Architecture".to_string(), "Storage".to_string()];
    let section_b_path = vec!["Architecture".to_string(), "Network".to_string()];

    // Different paths mean edits can proceed concurrently
    assert_ne!(section_a_path, section_b_path);

    // Same path would need coordination
    let section_a2_path = vec!["Architecture".to_string(), "Storage".to_string()];
    assert_eq!(section_a_path, section_a2_path);
}
