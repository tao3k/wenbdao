//! Unit tests for blocks module.

use super::*;

#[test]
fn test_extract_blocks_paragraph() {
    let text = "Hello, world!\n\nSecond paragraph.";
    let blocks = extract_blocks(text, 0, 1, &["Section".to_string()]);

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].kind, MarkdownBlockKind::Paragraph);
    assert_eq!(blocks[1].kind, MarkdownBlockKind::Paragraph);
}

#[test]
fn test_extract_blocks_code_fence() {
    let text = r"```rust
fn main() {}
```
";
    let blocks = extract_blocks(text, 0, 1, &["Code".to_string()]);

    assert_eq!(blocks.len(), 1);
    assert!(matches!(
        &blocks[0].kind,
        MarkdownBlockKind::CodeFence { language } if language == "rust"
    ));
    assert_eq!(blocks[0].structural_path, vec!["Code"]);
}

#[test]
fn test_extract_blocks_list() {
    let text = "- Item 1\n- Item 2\n- Item 3";
    let blocks = extract_blocks(text, 0, 1, &[]);

    assert_eq!(blocks.len(), 1);
    assert!(matches!(
        &blocks[0].kind,
        MarkdownBlockKind::List { ordered: false }
    ));
}

#[test]
fn test_extract_blocks_ordered_list() {
    let text = "1. First\n2. Second\n3. Third";
    let blocks = extract_blocks(text, 0, 1, &[]);

    assert_eq!(blocks.len(), 1);
    assert!(matches!(
        &blocks[0].kind,
        MarkdownBlockKind::List { ordered: true }
    ));
}

#[test]
fn test_extract_blocks_blockquote() {
    let text = "> Quoted text\n> More quote";
    let blocks = extract_blocks(text, 0, 1, &[]);

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].kind, MarkdownBlockKind::BlockQuote);
}

#[test]
fn test_extract_blocks_mixed() {
    let text = r#"Introduction paragraph.

```python
print("hello")
```

- List item 1
- List item 2

Conclusion paragraph.
"#;
    let blocks = extract_blocks(text, 0, 1, &["Section".to_string()]);

    assert_eq!(blocks.len(), 4);
    assert_eq!(blocks[0].kind, MarkdownBlockKind::Paragraph);
    assert!(
        matches!(&blocks[1].kind, MarkdownBlockKind::CodeFence { language } if language == "python")
    );
    assert!(matches!(
        &blocks[2].kind,
        MarkdownBlockKind::List { ordered: false }
    ));
    assert_eq!(blocks[3].kind, MarkdownBlockKind::Paragraph);
    // All blocks inherit the structural path
    for block in &blocks {
        assert_eq!(block.structural_path, vec!["Section"]);
    }
}

#[test]
fn test_extract_blocks_byte_range() {
    let text = "First para.\n\nSecond para.";
    let blocks = extract_blocks(text, 0, 1, &[]);

    assert_eq!(blocks.len(), 2);
    // First paragraph: "First para."
    assert_eq!(blocks[0].byte_range.1 - blocks[0].byte_range.0, 11);
    // Second paragraph: "Second para."
    assert_eq!(blocks[1].byte_range.1 - blocks[1].byte_range.0, 12);
}

#[test]
fn test_extract_blocks_line_range() {
    let text = "Line 1\n\nLine 3";
    let blocks = extract_blocks(text, 0, 1, &[]);

    assert_eq!(blocks.len(), 2);
    // First block on line 1
    assert_eq!(blocks[0].line_range.0, 1);
    // Second block on line 3
    assert_eq!(blocks[1].line_range.0, 3);
}

#[test]
fn test_extract_blocks_with_offset() {
    let text = "Content";
    let blocks = extract_blocks(text, 100, 10, &["Root".to_string(), "Child".to_string()]);

    assert_eq!(blocks.len(), 1);
    // Byte range should be offset by 100
    assert_eq!(blocks[0].byte_range.0, 100);
    // Line range should be offset by 9 (10-1, since content starts at line 1 relative)
    assert_eq!(blocks[0].line_range.0, 10);
    // Structural path should be preserved
    assert_eq!(blocks[0].structural_path, vec!["Root", "Child"]);
}

#[test]
fn test_line_col_to_byte_range_simple() {
    let text = "Hello\nWorld";
    let range = line_col_to_byte_range(text, 1, 1, 1, 5);
    assert_eq!(range, Some((0, 5)));

    let range = line_col_to_byte_range(text, 2, 1, 2, 5);
    assert_eq!(range, Some((6, 11)));
}

#[test]
fn test_line_col_to_byte_range_multiline() {
    let text = "Line 1\nLine 2\nLine 3";
    let range = line_col_to_byte_range(text, 1, 1, 3, 6);
    assert!(range.is_some());
    let Some((start, end)) = range else {
        panic!("multiline byte range should exist");
    };
    assert_eq!(start, 0);
    assert_eq!(end, text.len());
}
