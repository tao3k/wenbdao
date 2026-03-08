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
    /// Content contained by this section.
    pub section_text: String,
    /// Lower-cased section text for case-insensitive matching.
    pub section_text_lower: String,
    /// List of entity IDs mentioned in this specific section.
    pub entities: Vec<String>,
}

#[derive(Clone, Copy)]
struct SectionCursor<'a> {
    heading_title: &'a str,
    heading_path: &'a str,
    heading_level: usize,
    line_range: (usize, usize),
}

fn normalize_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
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
        section_text_lower: section_text.to_lowercase(),
        section_text,
        entities: extracted.note_links,
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
    let mut current_lines = Vec::new();
    let mut in_code_fence = false;
    let mut last_seen_line = 0usize;

    for (line_idx, line) in body.lines().enumerate() {
        let line_no = line_idx + 1;
        last_seen_line = line_no;
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            current_lines.push(line.to_string());
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
            continue;
        }
        current_lines.push(line.to_string());
    }

    push_section(
        &mut sections,
        SectionCursor {
            heading_title: &current_heading_title,
            heading_path: &current_heading_path,
            heading_level: current_heading_level,
            line_range: (current_start_line, last_seen_line.max(current_start_line)),
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
            section_text_lower: section_text.to_lowercase(),
            section_text,
            entities: extracted.note_links,
        });
    }
    sections
}
