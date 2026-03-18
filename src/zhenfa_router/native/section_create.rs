//! Section creation logic for `create_if_missing` in `semantic_edit`.
//!
//! Implements path traversal and section insertion for creating new heading
//! hierarchies in markdown documents.

/// Information about a sibling section for context.
#[derive(Debug, Clone)]
pub struct SiblingInfo {
    /// Title of the sibling section.
    pub title: String,
    /// First line of content (truncated for context).
    pub preview: String,
}

/// Information about where to insert new sections.
#[derive(Debug, Clone)]
pub struct InsertionInfo {
    /// Byte offset where new content should be inserted.
    pub insertion_byte: usize,
    /// Starting heading level for new sections (1-6).
    pub start_level: usize,
    /// Path components that still need to be created.
    pub remaining_path: Vec<String>,
    /// Previous sibling section (if any) for narrative context.
    pub prev_sibling: Option<SiblingInfo>,
    /// Next sibling section (if any) for narrative context.
    pub next_sibling: Option<SiblingInfo>,
}

/// Options for building new section content.
#[derive(Debug, Clone, Default)]
pub struct BuildSectionOptions {
    /// If true, generate a `:ID: <uuid>` property drawer for each new section.
    pub generate_id: bool,
    /// Custom ID prefix (e.g., "arch" -> ":ID: arch-uuid").
    pub id_prefix: Option<String>,
}

impl Default for InsertionInfo {
    fn default() -> Self {
        Self {
            insertion_byte: 0,
            start_level: 1,
            remaining_path: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
        }
    }
}

/// Find the insertion point for new sections by analyzing document content.
///
/// Traverses the existing heading structure to find the deepest matching
/// prefix of the target path, then returns where new sections should be
/// inserted and what heading levels they should use.
#[must_use]
pub fn find_insertion_point(doc_content: &str, path_components: &[String]) -> InsertionInfo {
    // Handle empty document
    if doc_content.trim().is_empty() {
        return InsertionInfo {
            insertion_byte: 0,
            start_level: 1,
            remaining_path: path_components.to_vec(),
            prev_sibling: None,
            next_sibling: None,
        };
    }

    if path_components.is_empty() {
        return InsertionInfo {
            insertion_byte: doc_content.len(),
            start_level: 1,
            remaining_path: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
        };
    }

    // Parse the document to find existing headings
    let lines: Vec<&str> = doc_content.lines().collect();
    let heading_positions = parse_headings(&lines);

    // Find the deepest matching prefix and sibling context
    let (matched_depth, last_matched_level, last_matched_end_line, matched_line_idx) =
        find_deepest_match_with_position(&heading_positions, path_components);

    // Calculate insertion byte
    let insertion_byte = calculate_insertion_byte(&lines, matched_depth, last_matched_end_line);

    // Remaining path to create
    let remaining_path: Vec<String> = path_components[matched_depth..].to_vec();

    // Starting level for new headings
    let start_level = if matched_depth == 0 {
        1
    } else {
        last_matched_level + 1
    };

    // Find sibling context
    let (prev_sibling, next_sibling) = find_sibling_context(
        &heading_positions,
        &lines,
        matched_depth,
        matched_line_idx,
        start_level,
    );

    InsertionInfo {
        insertion_byte,
        start_level,
        remaining_path,
        prev_sibling,
        next_sibling,
    }
}

/// Build new sections with optional ID generation.
#[must_use]
pub fn build_new_sections_content_with_options(
    remaining_path: &[String],
    start_level: usize,
    content: &str,
    options: &BuildSectionOptions,
) -> String {
    use std::fmt::Write;
    let mut result = String::new();
    let mut current_level = start_level;

    for (i, heading) in remaining_path.iter().enumerate() {
        let level = current_level.clamp(1, 6);
        let heading_marker = "#".repeat(level);

        if i > 0 {
            result.push('\n');
        }
        let _ = write!(result, "{heading_marker} {heading}");

        // Add :ID: property drawer if requested
        if options.generate_id {
            let id = generate_section_id(options.id_prefix.as_deref());
            let _ = write!(result, "\n:ID: {id}");
        }

        result.push_str("\n\n");
        current_level += 1;
    }

    result.push_str(content);
    result.push('\n');

    result
}

/// Generate a section ID: either prefixed or plain UUID.
pub(super) fn generate_section_id(prefix: Option<&str>) -> String {
    let uuid = uuid::Uuid::new_v4();
    let uuid_str = uuid.simple().to_string();

    match prefix {
        Some(p) => format!("{}-{}", p, &uuid_str[..8]),
        None => uuid_str[..12].to_string(),
    }
}

/// Compute Blake3 hash truncated to 16 hex characters.
#[must_use]
pub fn compute_content_hash(content: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    hash.to_hex()[..16].to_string()
}

// ============================================================================
// Private Helpers
// ============================================================================

/// A parsed heading with its position and level.
type HeadingPosition = (usize, usize, String); // (line_idx, level, title)

/// Parse all headings from the document lines.
fn parse_headings(lines: &[&str]) -> Vec<HeadingPosition> {
    let mut headings = Vec::new();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if let Some((level, title)) = parse_heading_line(trimmed) {
            headings.push((line_idx, level, title));
        }
    }

    headings
}

/// Parse a single heading line, returning (level, title) if it's a heading.
pub(super) fn parse_heading_line(line: &str) -> Option<(usize, String)> {
    if !line.starts_with('#') {
        return None;
    }

    let mut level = 0;
    for ch in line.chars() {
        if ch == '#' {
            level += 1;
        } else {
            break;
        }
    }

    if level == 0 || level > 6 {
        return None;
    }

    let title = line[level..].trim().to_string();
    if title.is_empty() {
        return None;
    }

    Some((level, title))
}

/// Find the deepest matching path prefix in the heading structure.
///
/// Returns (`matched_depth`, `last_matched_level`, `last_matched_end_line`, `matched_line_idx`).
fn find_deepest_match_with_position(
    heading_positions: &[HeadingPosition],
    path_components: &[String],
) -> (usize, usize, usize, Option<usize>) {
    let mut matched_depth = 0;
    let mut last_matched_level = 0;
    let mut last_matched_end_line = 0;
    let mut matched_line_idx: Option<usize> = None;

    for (depth, target_title) in path_components.iter().enumerate() {
        let expected_level = depth + 1;
        let mut found = false;

        for &(line_idx, level, ref title) in heading_positions {
            if title == target_title && level == expected_level {
                matched_depth = depth + 1;
                last_matched_level = level;
                last_matched_end_line = find_section_end(heading_positions, line_idx, level);
                matched_line_idx = Some(line_idx);
                found = true;
                break;
            }
        }

        if !found {
            break;
        }
    }

    (
        matched_depth,
        last_matched_level,
        last_matched_end_line,
        matched_line_idx,
    )
}

/// Find sibling context for narrative coherence.
fn find_sibling_context(
    heading_positions: &[HeadingPosition],
    lines: &[&str],
    matched_depth: usize,
    matched_line_idx: Option<usize>,
    target_level: usize,
) -> (Option<SiblingInfo>, Option<SiblingInfo>) {
    if heading_positions.is_empty() {
        return (None, None);
    }

    let mut prev_sibling: Option<SiblingInfo> = None;
    let next_sibling: Option<SiblingInfo> = None;

    // Determine the boundary for siblings
    // If matched_depth > 0, we're inserting under a parent section
    // Siblings are at target_level within the parent's scope
    let parent_line = matched_line_idx.unwrap_or(0);
    let parent_level = if matched_depth > 0 { matched_depth } else { 0 };

    // Find the end boundary of the parent section
    let end_boundary = if matched_depth > 0 {
        // Find next heading at parent level or higher
        heading_positions
            .iter()
            .find(|&&(line_idx, level, _)| line_idx > parent_line && level <= parent_level)
            .map_or(usize::MAX, |&(line_idx, _, _)| line_idx)
    } else {
        usize::MAX
    };

    // Find all headings at target_level within the parent's scope
    let siblings_at_level: Vec<_> = heading_positions
        .iter()
        .filter(|&&(line_idx, level, _)| {
            level == target_level && line_idx > parent_line && line_idx < end_boundary
        })
        .collect();

    // Previous sibling is the last one at this level
    if let Some(last) = siblings_at_level.last() {
        let preview = extract_preview(lines, last.0);
        prev_sibling = Some(SiblingInfo {
            title: last.2.clone(),
            preview,
        });
    }

    // Next sibling would be the first one after insertion point
    // Since we're inserting at the end of the parent's children, there's no next sibling
    // (Unless inserting in the middle, which we don't support yet)

    (prev_sibling, next_sibling)
}

/// Extract a preview string from content following a heading.
fn extract_preview(lines: &[&str], heading_line_idx: usize) -> String {
    for line in lines.iter().skip(heading_line_idx + 1).take(3) {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with(':') {
            return trimmed.chars().take(80).collect();
        }
    }
    String::new()
}

/// Find the end line of a section given its starting line and level.
fn find_section_end(
    heading_positions: &[HeadingPosition],
    start_line_idx: usize,
    section_level: usize,
) -> usize {
    for &(line_idx, level, _) in heading_positions {
        if line_idx > start_line_idx && level <= section_level {
            return line_idx;
        }
    }
    // No next heading found - section extends to end of document
    usize::MAX
}

/// Calculate the byte offset for insertion.
fn calculate_insertion_byte(
    lines: &[&str],
    matched_depth: usize,
    last_matched_end_line: usize,
) -> usize {
    if matched_depth == 0 {
        // No matching prefix - insert at end
        return lines.iter().map(|l| l.len() + 1).sum();
    }

    // Insert after the last matched section
    let mut byte_offset = 0;
    for (i, line) in lines.iter().enumerate() {
        if i >= last_matched_end_line {
            break;
        }
        byte_offset += line.len() + 1; // +1 for newline
    }
    byte_offset
}

#[cfg(test)]
#[path = "../../../tests/unit/zhenfa_router/native/section_create.rs"]
mod tests;
