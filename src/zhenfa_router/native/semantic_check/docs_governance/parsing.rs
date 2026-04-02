//! Parsing utilities for docs governance.

use std::path::{Component, Path};

use sha1::{Digest, Sha1};

use super::types::{FooterBlock, IdLine, LineSlice, LinksLine, TopPropertiesDrawer};

/// Derives an opaque document ID from a doc path.
#[must_use]
pub fn derive_opaque_doc_id(doc_path: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(normalize_doc_path(doc_path).as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Checks if a value is a valid opaque document ID.
#[must_use]
pub fn is_opaque_doc_id(value: &str) -> bool {
    value.len() == 40
        && value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
}

fn normalize_doc_path(doc_path: &str) -> String {
    let normalized = doc_path.replace('\\', "/");
    normalized
        .find("packages/rust/crates/")
        .map_or(normalized.clone(), |idx| normalized[idx..].to_string())
}

/// Checks if a doc path is a package-local crate doc.
#[must_use]
pub fn is_package_local_crate_doc(doc_path: &str) -> bool {
    let path = Path::new(doc_path);
    if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return false;
    }

    let components: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();

    components
        .windows(5)
        .any(|window| matches!(window, [a, b, c, _, d] if a == "packages" && b == "rust" && c == "crates" && d == "docs"))
}

/// Collects line slices from content.
#[must_use]
pub fn collect_lines(content: &str) -> Vec<LineSlice<'_>> {
    let mut lines = Vec::new();
    let mut offset = 0usize;

    for (line_number, raw_line) in content.split_inclusive('\n').enumerate() {
        let without_newline = raw_line.trim_end_matches(['\n', '\r']);
        let newline = &raw_line[without_newline.len()..];
        lines.push(LineSlice {
            line_number: line_number + 1,
            start_offset: offset,
            end_offset: offset + raw_line.len(),
            trimmed: without_newline.trim(),
            without_newline,
            newline,
        });
        offset += raw_line.len();
    }

    if !content.is_empty() && !content.ends_with('\n') {
        let without_newline = content.rsplit_once('\n').map_or(content, |(_, tail)| tail);
        if lines.is_empty()
            || lines
                .last()
                .is_some_and(|line| line.end_offset != content.len())
        {
            let start_offset = content.len() - without_newline.len();
            lines.push(LineSlice {
                line_number: lines.len() + 1,
                start_offset,
                end_offset: content.len(),
                trimmed: without_newline.trim(),
                without_newline,
                newline: "",
            });
        }
    }

    lines
}

/// Parses the top properties drawer from content.
#[must_use]
pub fn parse_top_properties_drawer(content: &str) -> Option<TopPropertiesDrawer<'_>> {
    let lines = collect_lines(content);

    let title_index = lines.iter().position(|line| !line.trimmed.is_empty())?;
    let title = lines.get(title_index)?;
    if !title.trimmed.starts_with('#') {
        return None;
    }

    let mut cursor = title_index + 1;
    while cursor < lines.len() && lines[cursor].trimmed.is_empty() {
        cursor += 1;
    }

    let properties = lines.get(cursor)?;
    if properties.trimmed != ":PROPERTIES:" {
        return None;
    }

    let newline = properties.newline;
    let insert_offset = properties.end_offset;

    let mut id_line = None;
    for line in lines.iter().skip(cursor + 1) {
        if line.trimmed == ":END:" {
            return Some(TopPropertiesDrawer {
                properties_line: properties.line_number,
                insert_offset,
                newline,
                id_line,
            });
        }

        if let Some(rest) = line.without_newline.strip_prefix(":ID:") {
            let leading_spaces = rest.chars().take_while(|ch| *ch == ' ').count();
            let value_start = line.start_offset + 4 + leading_spaces;
            let value = rest[leading_spaces..].trim();
            let value_end = value_start + value.len();
            id_line = Some(IdLine {
                line: line.line_number,
                value,
                value_start,
                value_end,
            });
        }
    }

    None
}

/// Parses the :LINKS: line from a relations block.
#[must_use]
pub fn parse_relations_links_line<'a>(lines: &'a [LineSlice<'a>]) -> Option<LinksLine<'a>> {
    let relations_idx = lines
        .iter()
        .position(|line| line.trimmed == ":RELATIONS:")?;
    for line in lines.iter().skip(relations_idx + 1) {
        if line.trimmed == ":END:" {
            break;
        }

        if let Some(rest) = line.without_newline.strip_prefix(":LINKS:") {
            let leading_spaces = rest.chars().take_while(|ch| *ch == ' ').count();
            let value = &rest[leading_spaces..];
            let value_start = line.start_offset + ":LINKS:".len() + leading_spaces;
            let value_end = line.start_offset + line.without_newline.len();
            return Some(LinksLine {
                line: line.line_number,
                value,
                value_start,
                value_end,
            });
        }
    }
    None
}

/// Parses the :FOOTER: block from lines.
#[must_use]
pub fn parse_footer_block<'a>(lines: &'a [LineSlice<'a>]) -> Option<FooterBlock<'a>> {
    let footer_idx = lines.iter().position(|line| line.trimmed == ":FOOTER:")?;
    let footer_line = &lines[footer_idx];
    let mut standards_value = None;
    let mut last_sync_value = None;

    for line in lines.iter().skip(footer_idx + 1) {
        if line.trimmed == ":END:" {
            return Some(FooterBlock {
                line: footer_line.line_number,
                start_offset: footer_line.start_offset,
                end_offset: line.end_offset,
                standards_value,
                last_sync_value,
            });
        }

        if let Some(rest) = line.without_newline.strip_prefix(":STANDARDS:") {
            standards_value = Some(rest.trim());
            continue;
        }

        if let Some(rest) = line.without_newline.strip_prefix(":LAST_SYNC:") {
            last_sync_value = Some(rest.trim());
        }
    }

    None
}

/// Extracts wikilinks from content.
#[must_use]
pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut remaining = content;

    while let Some(start) = remaining.find("[[") {
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find("]]") else {
            break;
        };
        let link = &after_start[..end];
        if !link.is_empty() {
            links.push(link.to_string());
        }
        remaining = &after_start[end + 2..];
    }

    links
}

/// Collects body links from an index document.
#[must_use]
pub fn collect_index_body_links(lines: &[LineSlice<'_>]) -> Vec<String> {
    let relations_start = lines
        .iter()
        .position(|line| line.trimmed == ":RELATIONS:")
        .unwrap_or(lines.len());

    let mut body_links = Vec::new();
    for line in &lines[..relations_start] {
        if !line.trimmed.starts_with("- ") {
            continue;
        }
        for link in extract_wikilinks(line.without_newline) {
            if !body_links.iter().any(|existing| existing == &link) {
                body_links.push(link);
            }
        }
    }
    body_links
}
