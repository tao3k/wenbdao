//! Unit tests for section creation logic.

use super::*;

#[test]
fn test_parse_heading_line() {
    assert_eq!(
        parse_heading_line("# Title"),
        Some((1, "Title".to_string()))
    );
    assert_eq!(
        parse_heading_line("## Sub Title"),
        Some((2, "Sub Title".to_string()))
    );
    assert_eq!(parse_heading_line("###Deep"), Some((3, "Deep".to_string())));
    assert_eq!(parse_heading_line("No heading"), None);
    assert_eq!(parse_heading_line("####### Too deep"), None);
}

#[test]
fn test_find_insertion_point_empty_doc() {
    let doc = "";
    let result = find_insertion_point(doc, &["Section".to_string()]);
    assert_eq!(result.insertion_byte, 0);
    assert_eq!(result.start_level, 1);
    assert_eq!(result.remaining_path, vec!["Section".to_string()]);
}

#[test]
fn test_find_insertion_point_whitespace_doc() {
    let doc = "   \n\n   \n";
    let result = find_insertion_point(doc, &["First".to_string(), "Second".to_string()]);
    assert_eq!(result.insertion_byte, 0);
    assert_eq!(result.start_level, 1);
    assert_eq!(result.remaining_path.len(), 2);
}

#[test]
fn test_find_insertion_point_existing_parent() {
    let doc = "# Parent\n\nSome content.\n\n## Child\n\nMore content.\n";
    let result = find_insertion_point(doc, &["Parent".to_string(), "NewChild".to_string()]);
    assert!(result.insertion_byte > 0);
    assert_eq!(result.start_level, 2);
    assert_eq!(result.remaining_path, vec!["NewChild".to_string()]);
}

#[test]
fn test_find_insertion_point_with_siblings() {
    let doc = "# Main\n\nIntro.\n\n## Alpha\n\nAlpha content.\n\n## Beta\n\nBeta content.\n";
    let result = find_insertion_point(doc, &["Main".to_string(), "NewSection".to_string()]);
    assert_eq!(result.start_level, 2);
    // When inserting a new H2 under Main, after existing H2s,
    // prev_sibling should be Beta (the last H2), next_sibling should be None
    let Some(prev_sibling) = result.prev_sibling.as_ref() else {
        panic!("should have prev_sibling");
    };
    assert_eq!(prev_sibling.title, "Beta");
    assert!(
        result.next_sibling.is_none(),
        "should not have next_sibling at end"
    );
}

#[test]
fn test_build_new_sections_content() {
    let content = build_new_sections_content_with_options(
        &["Section".to_string()],
        1,
        "Hello world",
        &BuildSectionOptions::default(),
    );
    assert!(content.starts_with("# Section\n\nHello world\n"));

    let nested = build_new_sections_content_with_options(
        &["A".to_string(), "B".to_string()],
        1,
        "Content",
        &BuildSectionOptions::default(),
    );
    assert!(nested.contains("# A"));
    assert!(nested.contains("## B"));
    assert!(nested.contains("Content"));
}

#[test]
fn test_build_new_sections_content_with_id() {
    let content = build_new_sections_content_with_options(
        &["MySection".to_string()],
        2,
        "Content here",
        &BuildSectionOptions {
            generate_id: true,
            id_prefix: Some("sec".to_string()),
        },
    );

    assert!(content.contains("## MySection"));
    assert!(content.contains(":ID: sec-"));
    assert!(content.contains("Content here"));
}

#[test]
fn test_build_new_sections_content_with_plain_id() {
    let content = build_new_sections_content_with_options(
        &["Section".to_string()],
        1,
        "Test",
        &BuildSectionOptions {
            generate_id: true,
            id_prefix: None,
        },
    );

    assert!(content.contains("# Section\n:ID:"));
    // ID should be 12 chars (truncated UUID)
    let id_line: Vec<&str> = content.lines().collect();
    let Some(id_part) = id_line[1].strip_prefix(":ID: ") else {
        panic!("expected generated ID line to start with ':ID: '");
    };
    assert_eq!(id_part.len(), 12);
}

#[test]
fn test_compute_content_hash() {
    let hash1 = compute_content_hash("test");
    let hash2 = compute_content_hash("test");
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 16);
}

#[test]
fn test_generate_section_id() {
    let id1 = generate_section_id(None);
    let id2 = generate_section_id(Some("arch"));

    assert_eq!(id1.len(), 12);
    assert!(id2.starts_with("arch-"));
    assert_eq!(id2.len(), 13); // "arch-" (5) + 8 hex chars
}
