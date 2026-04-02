use std::collections::HashMap;
use std::path::Path;

use super::super::code_observation::extract_observations;
use super::super::links::extract_link_targets;
use super::logbook::extract_logbook_entries;
use super::properties::extract_property_drawers;
use super::types::ParsedSection;

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
    source_path: &Path,
    root: &Path,
) {
    let section_text = lines.join("\n").trim().to_string();
    if section_text.is_empty() && cursor.heading_path.trim().is_empty() {
        return;
    }

    let attributes = if cursor.heading_level > 0 {
        extract_property_drawers(lines)
    } else {
        HashMap::new()
    };

    let logbook = if cursor.heading_level > 0 {
        extract_logbook_entries(lines, cursor.line_range.0)
    } else {
        Vec::new()
    };

    let observations = if cursor.heading_level > 0 {
        extract_observations(&attributes)
    } else {
        Vec::new()
    };

    let extracted = extract_link_targets(&section_text, source_path, root);
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
        observations,
    });
}

pub(crate) fn extract_sections(body: &str, source_path: &Path, root: &Path) -> Vec<ParsedSection> {
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
            byte_offset += line_bytes + 1;
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
        let extracted = extract_link_targets(&section_text, source_path, root);
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
            observations: Vec::new(),
        });
    }

    sections
}
