use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Standard property drawer attribute keys (Blueprint v2.0).
pub mod attrs {
    /// Explicit node identifier - takes precedence over structural_path.
    pub const ID: &str = "ID";
    /// Node status: STABLE | DRAFT | DEPRECATED.
    pub const STATUS: &str = "STATUS";
    /// Semantic contract constraint (e.g., `must_contain("Rust", "Lock")`).
    pub const CONTRACT: &str = "CONTRACT";
    /// Content fingerprint (Blake3).
    pub const HASH: &str = "HASH";
}

/// Execution drawer block types (Blueprint v2.4).
pub mod drawers {
    /// Property drawer block marker.
    pub const PROPERTIES: &str = "PROPERTIES";
    /// Execution log drawer for workflow tracking.
    pub const LOGBOOK: &str = "LOGBOOK";
    /// Block terminator.
    pub const END: &str = "END";
}

/// Node status values (Blueprint v2.0 Section 3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeStatus {
    /// Node is stable and can be safely referenced.
    #[default]
    Stable,
    /// Node is a draft, may change without notice.
    Draft,
    /// Node is deprecated, references should be updated.
    Deprecated,
}

impl NodeStatus {
    /// Parse status from string.
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "STABLE" => Self::Stable,
            "DRAFT" => Self::Draft,
            "DEPRECATED" => Self::Deprecated,
            _ => Self::Stable,
        }
    }
}

/// Execution log entry from `:LOGBOOK:` drawer (Blueprint v2.4).
///
/// Represents a single entry in the execution log for workflow tracking.
/// Format: `- [TIMESTAMP] MESSAGE`
///
/// # Example
///
/// ```markdown
/// :LOGBOOK:
/// - [2025-03-14] Agent Started: Initiating structural audit.
/// - [2025-03-14] Step [audit] completed with status OK.
/// :END:
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogbookEntry {
    /// Timestamp of the log entry (e.g., "2025-03-14").
    pub timestamp: String,
    /// The log message content.
    pub message: String,
    /// 1-based line number within the document.
    pub line_number: usize,
}

/// Parsed section row for section-aware retrieval and `HippoRAG 2` `Passage Nodes`.
#[derive(Debug, Clone)]
pub struct ParsedSection {
    /// Leaf heading title for this section.
    pub heading_title: String,
    /// Slash-delimited heading ancestry for this section.
    pub heading_path: String,
    /// Lower-cased `heading_path` for case-insensitive matching.
    pub heading_path_lower: String,
    /// Markdown heading depth for this section.
    pub heading_level: usize,
    /// Inclusive 1-based start line within the markdown body.
    pub line_start: usize,
    /// Inclusive 1-based end line within the markdown body.
    pub line_end: usize,
    /// Byte offset from start of document where this section begins.
    pub byte_start: usize,
    /// Byte offset (exclusive) where this section ends.
    pub byte_end: usize,
    /// Content contained by this section.
    pub section_text: String,
    /// Lower-cased section text for case-insensitive matching.
    pub section_text_lower: String,
    /// List of entity IDs mentioned in this specific section.
    pub entities: Vec<String>,
    /// Property drawer attributes extracted from heading (e.g., :ID: arch-v1).
    pub attributes: std::collections::HashMap<String, String>,
    /// Execution log entries from `:LOGBOOK:` drawer (Blueprint v2.4).
    pub logbook: Vec<LogbookEntry>,
}

#[derive(Clone, Copy)]
struct SectionCursor<'a> {
    heading_title: &'a str,
    heading_path: &'a str,
    heading_level: usize,
    line_range: (usize, usize),
    byte_range: (usize, usize),
}

fn normalize_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse a property drawer line in the format `:KEY: VALUE`.
///
/// Property drawers must appear immediately after a heading and use the syntax:
/// ```markdown
/// ## Heading
/// :ID: arch-v1
/// :TAGS: core, design
/// ```
fn parse_property_drawer(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with(':') {
        return None;
    }

    // Find the closing colon after the key
    let rest = &trimmed[1..]; // Skip the leading ':'
    let colon_pos = rest.find(':')?;

    let key = rest[..colon_pos].trim().to_uppercase();
    if key.is_empty() {
        return None;
    }

    let value = rest[colon_pos + 1..].trim().to_string();
    if value.is_empty() {
        return None;
    }

    Some((key, value))
}

/// Extract property drawer attributes from lines following a heading.
///
/// Supports two formats:
/// 1. Org-style block format (Blueprint v2.0):
///    ```markdown
///    :PROPERTIES:
///    :ID:       uuid-v4-or-slug
///    :STATUS:   STABLE
///    :END:
///    ```
/// 2. Compact single-line format:
///    ```markdown
///    :ID: arch-v1
///    :TAGS: core, design
///    ```
///
/// Note: Once a `:PROPERTIES:` block is encountered and closed with `:END:`,
/// no further property extraction occurs (block format takes precedence).
fn extract_property_drawers(lines: &[String]) -> HashMap<String, String> {
    let mut attributes = HashMap::new();
    let mut in_properties_block = false;
    let mut block_ended = false;

    for line in lines {
        let trimmed = line.trim();

        // Check for :PROPERTIES: block start
        if trimmed == ":PROPERTIES:" {
            in_properties_block = true;
            continue;
        }

        // Check for :END: block terminator
        if in_properties_block && trimmed == ":END:" {
            in_properties_block = false;
            block_ended = true;
            continue;
        }

        // Inside :PROPERTIES: block, parse property lines
        if in_properties_block {
            if let Some((key, value)) = parse_property_drawer(line) {
                attributes.insert(key, value);
            }
            continue;
        }

        // After block ended, stop extracting properties
        if block_ended {
            break;
        }

        // Outside block: support compact single-line format
        if let Some((key, value)) = parse_property_drawer(line) {
            attributes.insert(key, value);
        } else if trimmed.is_empty() {
            // Skip empty lines at the start of the section
            continue;
        } else {
            // Stop at first non-property line
            break;
        }
    }

    attributes
}

/// Parse a single logbook entry line.
///
/// Format: `- [TIMESTAMP] MESSAGE`
/// Example: `- [2025-03-14] Agent Started: Initiating structural audit.`
fn parse_logbook_entry(line: &str, line_number: usize) -> Option<LogbookEntry> {
    let trimmed = line.trim();

    // Must start with list item marker
    if !trimmed.starts_with('-') {
        return None;
    }

    let rest = trimmed[1..].trim_start();

    // Find timestamp in brackets
    if !rest.starts_with('[') {
        return None;
    }

    let close_bracket = rest.find(']')?;
    let timestamp = rest[1..close_bracket].trim().to_string();

    if timestamp.is_empty() {
        return None;
    }

    let message = rest[close_bracket + 1..].trim().to_string();

    if message.is_empty() {
        return None;
    }

    Some(LogbookEntry {
        timestamp,
        message,
        line_number,
    })
}

/// Extract execution log entries from `:LOGBOOK:` drawer.
///
/// Supports the format specified in Blueprint v2.4:
/// ```markdown
/// :LOGBOOK:
/// - [2025-03-14] Agent Started: Initiating structural audit.
/// - [2025-03-14] Step [audit] completed with status OK.
/// :END:
/// ```
///
/// The logbook provides an execution trail for workflow tracking,
/// enabling LLM agents to read task status like reading a document.
fn extract_logbook_entries(lines: &[String], start_line: usize) -> Vec<LogbookEntry> {
    let mut entries = Vec::new();
    let mut in_logbook_block = false;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Check for :LOGBOOK: block start
        if trimmed == ":LOGBOOK:" {
            in_logbook_block = true;
            continue;
        }

        // Check for :END: block terminator
        if in_logbook_block && trimmed == ":END:" {
            break;
        }

        // Inside :LOGBOOK: block, parse list entries
        if in_logbook_block {
            let line_number = start_line + idx + 1;
            if let Some(entry) = parse_logbook_entry(line, line_number) {
                entries.push(entry);
            }
        }
    }

    entries
}

fn parse_markdown_heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let mut level = 0usize;
    for ch in trimmed.chars() {
        if ch == '#' {
            level += 1;
        } else {
            break;
        }
    }
    if level == 0 || level > 6 {
        return None;
    }
    let rest = trimmed[level..].trim_start();
    if rest.is_empty() {
        return None;
    }
    Some((level, normalize_whitespace(rest)))
}

fn push_section(
    out: &mut Vec<ParsedSection>,
    cursor: SectionCursor<'_>,
    lines: &[String],
    source_path: &std::path::Path,
    root: &std::path::Path,
) {
    let section_text = lines.join("\n").trim().to_string();
    if section_text.is_empty() && cursor.heading_path.trim().is_empty() {
        return;
    }

    // Extract property drawer attributes from lines following a heading
    let attributes = if cursor.heading_level > 0 {
        extract_property_drawers(lines)
    } else {
        HashMap::new()
    };

    // Extract execution log entries from :LOGBOOK: drawer (Blueprint v2.4)
    let logbook = if cursor.heading_level > 0 {
        extract_logbook_entries(lines, cursor.line_range.0)
    } else {
        Vec::new()
    };

    let extracted = super::links::extract_link_targets(&section_text, source_path, root);
    let line_start = cursor.line_range.0.max(1);
    let line_end = cursor.line_range.1.max(line_start);

    out.push(ParsedSection {
        heading_title: cursor.heading_title.to_string(),
        heading_path: cursor.heading_path.to_string(),
        heading_path_lower: cursor.heading_path.to_lowercase(),
        heading_level: cursor.heading_level,
        line_start,
        line_end,
        byte_start: cursor.byte_range.0,
        byte_end: cursor.byte_range.1,
        section_text_lower: section_text.to_lowercase(),
        section_text,
        entities: extracted.note_links,
        attributes,
        logbook,
    });
}

pub(super) fn extract_sections(
    body: &str,
    source_path: &std::path::Path,
    root: &std::path::Path,
) -> Vec<ParsedSection> {
    let mut sections = Vec::new();
    let mut heading_stack: Vec<String> = Vec::new();
    let mut current_heading_title = String::new();
    let mut current_heading_path = String::new();
    let mut current_heading_level = 0usize;
    let mut current_start_line = 1usize;
    let mut current_start_byte = 0usize;
    let mut current_lines = Vec::new();
    let mut in_code_fence = false;
    let mut last_seen_line = 0usize;
    let mut last_seen_byte = 0usize;

    // Track byte positions while iterating
    let mut byte_offset = 0usize;

    for (line_idx, line) in body.lines().enumerate() {
        let line_no = line_idx + 1;
        let line_bytes = line.len();
        last_seen_line = line_no;
        last_seen_byte = byte_offset + line_bytes;

        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            current_lines.push(line.to_string());
            byte_offset += line_bytes + 1; // +1 for newline
            continue;
        }
        if !in_code_fence && let Some((level, heading)) = parse_markdown_heading(trimmed) {
            push_section(
                &mut sections,
                SectionCursor {
                    heading_title: &current_heading_title,
                    heading_path: &current_heading_path,
                    heading_level: current_heading_level,
                    line_range: (
                        current_start_line,
                        line_no.saturating_sub(1).max(current_start_line),
                    ),
                    byte_range: (
                        current_start_byte,
                        byte_offset.saturating_sub(1).max(current_start_byte),
                    ),
                },
                &current_lines,
                source_path,
                root,
            );
            current_lines.clear();
            if heading_stack.len() >= level {
                heading_stack.truncate(level.saturating_sub(1));
            }
            heading_stack.push(heading.clone());
            current_heading_title = heading;
            current_heading_path = heading_stack.join(" / ");
            current_heading_level = level;
            current_start_line = line_no;
            current_start_byte = byte_offset;
            byte_offset += line_bytes + 1;
            continue;
        }
        current_lines.push(line.to_string());
        byte_offset += line_bytes + 1;
    }

    push_section(
        &mut sections,
        SectionCursor {
            heading_title: &current_heading_title,
            heading_path: &current_heading_path,
            heading_level: current_heading_level,
            line_range: (current_start_line, last_seen_line.max(current_start_line)),
            byte_range: (current_start_byte, last_seen_byte.max(current_start_byte)),
        },
        &current_lines,
        source_path,
        root,
    );
    if sections.is_empty() {
        let section_text = body.trim().to_string();
        let extracted = super::links::extract_link_targets(&section_text, source_path, root);
        sections.push(ParsedSection {
            heading_title: String::new(),
            heading_path: String::new(),
            heading_path_lower: String::new(),
            heading_level: 0,
            line_start: 1,
            line_end: body.lines().count().max(1),
            byte_start: 0,
            byte_end: body.len(),
            section_text_lower: section_text.to_lowercase(),
            section_text,
            entities: extracted.note_links,
            attributes: HashMap::new(),
            logbook: Vec::new(),
        });
    }
    sections
}

#[cfg(test)]
mod tests {
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
}
